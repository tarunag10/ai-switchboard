#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const armPath =
  process.env.MAC_AI_SWITCHBOARD_REBOOT_ARM_PATH ??
  "dist/reboot-level-installed-proof-arm.json";
const markerPath =
  process.env.MAC_AI_SWITCHBOARD_REBOOT_MARKER_PATH ??
  "dist/reboot-level-installed-proof-marker.json";
const appPathCandidates = [
  "/Applications/AI Switchboard for Mac.app",
  "/Applications/AI Switchboard.app",
  "/Applications/Mac AI Switchboard.app",
  "/Applications/Mac Switchboard.app",
];
const appPath = appPathCandidates.find((candidate) => fs.existsSync(candidate));

function run(command, args) {
  const result = spawnSync(command, args, { encoding: "utf8", timeout: 120_000 });
  return {
    command: [command, ...args].join(" "),
    ok: result.status === 0,
    status: result.status,
    stdout: result.stdout?.trim() ?? "",
    stderr: result.stderr?.trim() ?? "",
  };
}

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
}

function currentBootSession() {
  const result = run("sysctl", ["-n", "kern.boottime"]);
  const match = result.stdout.match(/sec = (\d+)/);
  return {
    bootTimeUnixSeconds: match ? Number(match[1]) : null,
    hostUuid: run("sysctl", ["-n", "kern.hostuuid"]).stdout || null,
  };
}

function verifyPublicArtifact() {
  const artifactPath = process.env.MAC_AI_SWITCHBOARD_PUBLIC_ARTIFACT_PATH;
  if (!artifactPath) return { supplied: false, verified: null };
  const present = fs.existsSync(artifactPath);
  const hdiutil = present ? run("hdiutil", ["verify", artifactPath]) : null;
  return {
    supplied: true,
    path: artifactPath,
    present,
    hdiutilVerifyOk: hdiutil?.ok ?? false,
    sha256: present
      ? crypto.createHash("sha256").update(fs.readFileSync(artifactPath)).digest("hex")
      : null,
    verified: present && hdiutil?.ok === true,
  };
}

const arm = readJson(armPath);
const session = currentBootSession();
const infoPlistPath = appPath ? path.join(appPath, "Contents", "Info.plist") : null;
const codesign = appPath ? run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", appPath]) : null;
const gatekeeper = appPath ? run("spctl", ["--assess", "--type", "execute", "--verbose=4", appPath]) : null;
const stapler = appPath ? run("xcrun", ["stapler", "validate", appPath]) : null;
const artifact = verifyPublicArtifact();
const armValid =
  arm?.kind === "mac_ai_switchboard.reboot_level_installed_proof_arm" &&
  Number.isInteger(arm?.bootSession?.bootTimeUnixSeconds) &&
  typeof arm?.nonce === "string";
const rebootObserved =
  armValid &&
  session.bootTimeUnixSeconds !== null &&
  arm.bootSession.bootTimeUnixSeconds !== session.bootTimeUnixSeconds;
const trustReady = Boolean(
  appPath &&
    infoPlistPath &&
    fs.existsSync(infoPlistPath) &&
    codesign?.ok &&
    gatekeeper?.ok &&
    stapler?.ok,
);
const blockers = [
  armValid ? null : `valid pre-reboot arm receipt missing at ${armPath}`,
  rebootObserved ? null : "current boot session does not differ from the armed boot session",
  appPath ? null : "signed installed app is missing from /Applications",
  infoPlistPath && fs.existsSync(infoPlistPath) ? null : "installed app Info.plist is missing",
  codesign?.ok ? null : "installed app codesign verification failed",
  gatekeeper?.ok ? null : "installed app Gatekeeper assessment failed",
  stapler?.ok ? null : "installed app notarization stapler validation failed",
  artifact.supplied && !artifact.verified ? "supplied public artifact failed hdiutil verification" : null,
].filter(Boolean);

if (blockers.length) {
  console.error("Post-reboot marker was NOT written:");
  for (const blocker of blockers) console.error(`- ${blocker}`);
  process.exit(1);
}

const marker = {
  schemaVersion: 2,
  kind: "mac_ai_switchboard.reboot_level_installed_marker",
  generatedAt: new Date().toISOString(),
  recordedAfterManualReboot: true,
  armPath,
  armNonce: arm.nonce,
  armedBootSession: arm.bootSession,
  recordedBootSession: session,
  appPath,
  appTrust: {
    verified: trustReady,
    infoPlistPath,
    codesignVerify: { command: codesign.command, ok: codesign.ok },
    gatekeeperAssess: { command: gatekeeper.command, ok: gatekeeper.ok },
    staplerValidate: { command: stapler.command, ok: stapler.ok },
  },
  publicArtifact: artifact,
  note:
    "Written only after a changed macOS boot-session identity and successful installed-app signing, Gatekeeper, and stapler checks. This does not fabricate a reboot or perform cleanup.",
};
fs.mkdirSync(path.dirname(markerPath), { recursive: true });
fs.writeFileSync(markerPath, `${JSON.stringify(marker, null, 2)}\n`);
console.log(`Post-reboot installed-app marker written: ${markerPath}`);
