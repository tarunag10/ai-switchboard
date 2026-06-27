import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const appPath = "/Applications/Mac AI Switchboard.app";
const appInfoPlistPath = path.join(appPath, "Contents", "Info.plist");
const packageJson = JSON.parse(fs.readFileSync("package.json", "utf8"));
const tauriConfig = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const arch = process.arch === "arm64" ? "aarch64" : process.arch;
const dmgPath = `dist/release-artifacts/Mac-AI-Switchboard_${packageJson.version}-local-unsigned-${arch}.dmg`;
const rawDmgPath = `src-tauri/target/release/bundle/dmg/Mac AI Switchboard_${packageJson.version}_${arch}.dmg`;
const summaryPath = "dist/local-installed-smoke-summary.md";
const jsonPath = "dist/local-installed-smoke-summary.json";

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    ...options,
  });
  return {
    status: result.status,
    stdout: result.stdout?.trim() ?? "",
    stderr: result.stderr?.trim() ?? "",
    ok: result.status === 0,
  };
}

function plistValue(key) {
  const result = run("/usr/libexec/PlistBuddy", [
    "-c",
    `Print :${key}`,
    appInfoPlistPath,
  ]);
  return result.ok ? result.stdout : null;
}

function sha256(filePath) {
  if (!fs.existsSync(filePath)) return null;
  return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

if (!fs.existsSync(dmgPath) && fs.existsSync(rawDmgPath)) {
  fs.mkdirSync(path.dirname(dmgPath), { recursive: true });
  fs.copyFileSync(rawDmgPath, dmgPath);
  fs.writeFileSync(`${dmgPath}.sha256`, `${sha256(dmgPath)}  ${dmgPath}\n`);
}

const generatedAt = new Date().toISOString();
const appPresent = fs.existsSync(appPath);
const metadataPresent = fs.existsSync(appInfoPlistPath);
const dmgPresent = fs.existsSync(dmgPath);
const bundleIdentifier = metadataPresent ? plistValue("CFBundleIdentifier") : null;
const version = metadataPresent ? plistValue("CFBundleShortVersionString") : null;
const displayName = metadataPresent ? plistValue("CFBundleDisplayName") : null;
const bundleName = metadataPresent ? plistValue("CFBundleName") : null;
const metadataMatches =
  bundleIdentifier === tauriConfig.identifier &&
  version === packageJson.version &&
  (displayName === tauriConfig.productName || bundleName === tauriConfig.productName);
const dmgSha256 = sha256(dmgPath);
const dmgVerify = dmgPresent ? run("hdiutil", ["verify", dmgPath]) : null;
const codesignVerify = appPresent
  ? run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", appPath])
  : null;
const spctlAssess = appPresent
  ? run("spctl", ["--assess", "--type", "execute", "--verbose=4", appPath])
  : null;
const running = run("pgrep", ["-fl", "headroom-desktop|Mac AI Switchboard"]);

const payload = {
  generatedAt,
  kind: "mac_ai_switchboard.local_installed_smoke",
  releaseGateEvidence: false,
  app: {
    path: appPath,
    present: appPresent,
    metadataPresent,
    bundleIdentifier,
    version,
    displayName,
    bundleName,
    metadataMatches,
    running: running.ok,
    runningProcess: running.stdout || null,
  },
  dmg: {
    path: dmgPath,
    present: dmgPresent,
    sha256: dmgSha256,
    hdiutilVerifyOk: dmgVerify?.ok ?? false,
  },
  signing: {
    codesignVerifyOk: codesignVerify?.ok ?? false,
    spctlAssessOk: spctlAssess?.ok ?? false,
    distributionReady: false,
    note:
      "Local unsigned/ad-hoc setup evidence only. This does not replace signed/notarized installed smoke confirmation.",
  },
};

const summary = `# Local Installed Smoke Summary

Generated: ${generatedAt}

- Evidence kind: local unsigned/ad-hoc install check
- Release gate evidence: no
- App present: ${appPresent ? "yes" : "no"} (${appPath})
- App metadata present: ${metadataPresent ? "yes" : "no"} (${appInfoPlistPath})
- Bundle identifier: ${bundleIdentifier ?? "unknown"}
- Version: ${version ?? "unknown"}
- Display name: ${displayName ?? "unknown"}
- Bundle name: ${bundleName ?? "unknown"}
- Metadata matches repo config: ${metadataMatches ? "yes" : "no"}
- Running process: ${running.ok ? "yes" : "no"}
- Local DMG present: ${dmgPresent ? "yes" : "no"} (${dmgPath})
- Local DMG SHA-256: ${dmgSha256 ?? "missing"}
- DMG hdiutil verify: ${dmgVerify?.ok ? "pass" : "not verified"}
- Local codesign verify: ${codesignVerify?.ok ? "pass" : "fail"}
- Gatekeeper assessment: ${spctlAssess?.ok ? "pass" : "reject"}

This file proves the local unsigned/ad-hoc app was built, installed, and inspected on this Mac. It does not prove a public signed/notarized release. Keep using \`npm run smoke:installed -- --confirm\` only after \`docs/beta-smoke-test.md\` passes on a signed DMG install.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, summary);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

if (!appPresent || !metadataPresent || !metadataMatches || !dmgPresent || !dmgVerify?.ok) {
  console.error("Local installed smoke summary recorded with missing required local evidence.");
  console.error(`Summary written: ${summaryPath}`);
  process.exit(1);
}

console.log("Local installed smoke summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);
