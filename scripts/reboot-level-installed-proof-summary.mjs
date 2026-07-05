#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const summaryPath = "dist/reboot-level-installed-proof-summary.md";
const jsonPath = "dist/reboot-level-installed-proof-summary.json";
const defaultMarkerPath = "dist/reboot-level-installed-proof-marker.json";
const markerPath =
  process.env.MAC_AI_SWITCHBOARD_REBOOT_MARKER_PATH || defaultMarkerPath;
const appPathCandidates = [
  "/Applications/AI Switchboard for Mac.app",
  "/Applications/AI Switchboard.app",
  "/Applications/Mac AI Switchboard.app",
  "/Applications/Mac Switchboard.app",
];
const appPath =
  appPathCandidates.find((candidate) => fs.existsSync(candidate)) ??
  appPathCandidates[0];
const appInfoPlistPath = path.join(appPath, "Contents", "Info.plist");

const supportingArtifacts = [
  {
    id: "installed-smoke",
    label: "Public installed-app smoke summary",
    path: "dist/installed-smoke-summary.md",
    requiredForRelease: true,
    check: (content) =>
      content.includes("Confirmation: explicit tester confirmation received") &&
      content.includes("Result: tester confirmed beta smoke checklist passed"),
  },
  {
    id: "local-doctor-repair",
    label: "Local Doctor repair validation summary",
    path: "dist/local-doctor-repair-validation-summary.json",
    requiredForRelease: false,
    checkJson: (report) =>
      report.kind === "mac_ai_switchboard.local_doctor_repair_validation" &&
      report.releaseGateEvidence === false &&
      report.passed === true,
  },
  {
    id: "local-rollback",
    label: "Local Rollback validation summary",
    path: "dist/local-rollback-validation-summary.json",
    requiredForRelease: false,
    checkJson: (report) =>
      report.kind === "mac_ai_switchboard.local_rollback_validation" &&
      report.releaseGateEvidence === false &&
      report.passed === true,
  },
  {
    id: "local-uninstall",
    label: "Local uninstall dry-run validation summary",
    path: "dist/local-uninstall-validation-summary.json",
    requiredForRelease: false,
    checkJson: (report) =>
      report.kind === "mac_ai_switchboard.local_uninstall_validation" &&
      report.releaseGateEvidence === false &&
      report.destructive === false &&
      report.passed === true,
  },
];

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    timeout: 120_000,
  });
  return {
    command: [command, ...args].join(" "),
    status: result.status,
    ok: result.status === 0,
    stdout: result.stdout?.trim() ?? "",
    stderr: result.stderr?.trim() ?? "",
  };
}

function safeReadJson(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
}

function plistValue(key) {
  if (!fs.existsSync(appInfoPlistPath)) {
    return null;
  }
  const result = run("/usr/libexec/PlistBuddy", [
    "-c",
    `Print :${key}`,
    appInfoPlistPath,
  ]);
  return result.ok ? result.stdout : null;
}

function currentBootTime() {
  const result = run("sysctl", ["-n", "kern.boottime"]);
  const match = result.stdout.match(/sec = (\d+)/);
  return {
    command: result.command,
    ok: result.ok && Boolean(match),
    raw: result.stdout || result.stderr,
    unixSeconds: match ? Number(match[1]) : null,
  };
}

function inspectArtifact(artifact) {
  const present = fs.existsSync(artifact.path);
  if (!present) {
    return {
      ...artifact,
      present,
      valid: false,
      blocker: `${artifact.label} missing`,
    };
  }

  if (artifact.checkJson) {
    const report = safeReadJson(artifact.path);
    const valid = Boolean(report && artifact.checkJson(report));
    return {
      ...artifact,
      present,
      valid,
      generatedAt: report?.generatedAt ?? null,
      blocker: valid ? null : `${artifact.label} is present but not passing/current`,
    };
  }

  const content = fs.readFileSync(artifact.path, "utf8");
  const valid = artifact.check(content);
  return {
    ...artifact,
    present,
    valid,
    blocker: valid ? null : `${artifact.label} is present but lacks required confirmation`,
  };
}

const generatedAt = new Date().toISOString();
const bootTime = currentBootTime();
const marker = safeReadJson(markerPath);
const appPresent = fs.existsSync(appPath);
const metadataPresent = fs.existsSync(appInfoPlistPath);
const codesignVerify = appPresent
  ? run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", appPath])
  : null;
const spctlAssess = appPresent
  ? run("spctl", ["--assess", "--type", "execute", "--verbose=4", appPath])
  : null;
const staplerValidate = appPresent
  ? run("xcrun", ["stapler", "validate", appPath])
  : null;
const artifacts = supportingArtifacts.map(inspectArtifact);
const markerMatchesCurrentBoot =
  marker?.kind === "mac_ai_switchboard.reboot_level_installed_marker" &&
  marker?.appPath === appPath &&
  marker?.currentBootTimeUnixSeconds === bootTime.unixSeconds &&
  marker?.recordedAfterManualReboot === true;
const trustReady =
  appPresent &&
  metadataPresent &&
  codesignVerify?.ok === true &&
  spctlAssess?.ok === true &&
  staplerValidate?.ok === true;
const supportingArtifactsReady = artifacts.every((artifact) => artifact.valid);
const blockers = [
  appPresent ? null : `installed app missing at ${appPath}`,
  metadataPresent ? null : `installed app metadata missing at ${appInfoPlistPath}`,
  codesignVerify?.ok ? null : "installed app codesign verification failed or was not run",
  spctlAssess?.ok ? null : "installed app Gatekeeper assessment failed or was not run",
  staplerValidate?.ok ? null : "installed app notarization stapler validation failed or was not run",
  ...artifacts.map((artifact) => artifact.blocker).filter(Boolean),
  markerMatchesCurrentBoot
    ? null
    : `post-reboot marker missing or stale at ${markerPath}`,
].filter(Boolean);
const proofReady = blockers.length === 0 && trustReady && supportingArtifactsReady;

const payload = {
  schemaVersion: 1,
  generatedAt,
  kind: "mac_ai_switchboard.reboot_level_installed_proof",
  releaseGateEvidence: proofReady,
  proofReady,
  destructive: false,
  blockers,
  app: {
    path: appPath,
    present: appPresent,
    metadataPresent,
    infoPlistPath: appInfoPlistPath,
    bundleIdentifier: plistValue("CFBundleIdentifier"),
    version: plistValue("CFBundleShortVersionString"),
    displayName: plistValue("CFBundleDisplayName"),
    bundleName: plistValue("CFBundleName"),
  },
  trust: {
    ready: trustReady,
    codesignVerify: codesignVerify
      ? {
          command: codesignVerify.command,
          ok: codesignVerify.ok,
          status: codesignVerify.status,
          stderrPreview: codesignVerify.stderr.slice(0, 2000),
        }
      : null,
    gatekeeperAssess: spctlAssess
      ? {
          command: spctlAssess.command,
          ok: spctlAssess.ok,
          status: spctlAssess.status,
          stderrPreview: spctlAssess.stderr.slice(0, 2000),
        }
      : null,
    staplerValidate: staplerValidate
      ? {
          command: staplerValidate.command,
          ok: staplerValidate.ok,
          status: staplerValidate.status,
          stdoutPreview: staplerValidate.stdout.slice(0, 2000),
          stderrPreview: staplerValidate.stderr.slice(0, 2000),
        }
      : null,
  },
  rebootMarker: {
    path: markerPath,
    present: Boolean(marker),
    matchesCurrentBoot: markerMatchesCurrentBoot,
    requiredKind: "mac_ai_switchboard.reboot_level_installed_marker",
    currentBootTimeUnixSeconds: bootTime.unixSeconds,
    bootTimeCommand: bootTime.command,
    markerGeneratedAt: marker?.generatedAt ?? null,
  },
  supportingArtifacts: artifacts.map(({ check, checkJson, ...artifact }) => artifact),
  note:
    "This script is non-destructive. It records blockers when current installed-app trust, supporting Doctor/Rollback/uninstall evidence, or a real post-reboot marker is missing.",
};

const markdown = `# Reboot-Level Installed Proof Summary

Generated: ${generatedAt}

- Release gate evidence: ${proofReady ? "yes" : "no"}
- Proof ready: ${proofReady ? "yes" : "no"}
- Destructive actions: no
- App: ${appPresent ? "present" : "missing"} (${appPath})
- App metadata: ${metadataPresent ? "present" : "missing"} (${appInfoPlistPath})
- Codesign verification: ${codesignVerify?.ok ? "pass" : "fail/not run"}
- Gatekeeper assessment: ${spctlAssess?.ok ? "pass" : "fail/not run"}
- Stapler validation: ${staplerValidate?.ok ? "pass" : "fail/not run"}
- Current boot time marker: ${bootTime.unixSeconds ?? "unknown"}
- Post-reboot marker: ${markerMatchesCurrentBoot ? "present for current boot" : `missing or stale (${markerPath})`}
- Blockers: ${blockers.join(", ") || "none"}

## Supporting Evidence

${artifacts
  .map(
    (artifact) =>
      `- ${artifact.valid ? "pass" : "blocked"}: ${artifact.label} (${artifact.path})`,
  )
  .join("\n")}

## Reboot Marker Requirement

The proof is intentionally blocked until a marker with kind \`mac_ai_switchboard.reboot_level_installed_marker\`, app path \`${appPath}\`, current boot time \`${bootTime.unixSeconds ?? "unknown"}\`, and \`recordedAfterManualReboot: true\` exists at \`${markerPath}\`.

This summary does not reboot the machine, uninstall the app, or mutate user files. It only records the current evidence state.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, markdown);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log("Reboot-level installed proof summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!proofReady) {
  console.error(`Reboot-level installed proof blocked: ${blockers.join("; ")}`);
  process.exitCode = 1;
}
