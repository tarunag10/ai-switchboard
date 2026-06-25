import fs from "node:fs";

const betaSmokeDoc = "docs/beta-smoke-test.md";
const installDoc = "docs/install.md";
const releaseDoc = "docs/macos-release.md";
const appPath = "/Applications/Mac AI Switchboard.app";

const requiredSignals = {
  [betaSmokeDoc]: [
    "Switchboard checks",
    "Planned connectors are visible but manual",
    "copyable manual setup guide",
    "Launcher auto-setup and proxy verification should include only managed connectors",
    "Codex traffic is actively optimized",
    "Pause / resume",
  ],
  [installDoc]: [
    "Mac-AI-Switchboard_<version>.dmg",
    "Full optimization",
    "RTK only",
    "Off mode",
    "Codex Compression Troubleshooting",
  ],
  [releaseDoc]: [
    "npm run release:check",
    "Mac-AI-Switchboard_",
    "notarization",
    "staging-rolling",
  ],
};

const failures = [];

function read(path) {
  if (!fs.existsSync(path)) {
    failures.push(`Missing ${path}`);
    return "";
  }
  return fs.readFileSync(path, "utf8");
}

for (const [path, signals] of Object.entries(requiredSignals)) {
  const body = read(path);
  for (const signal of signals) {
    if (!body.includes(signal)) {
      failures.push(`${path} missing smoke signal: ${signal}`);
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

const installed = fs.existsSync(appPath);

console.log("Smoke preflight passed.");
console.log(`Installed app present: ${installed ? "yes" : "no"} (${appPath})`);
console.log(`Next: install the DMG, then run ${betaSmokeDoc} on the installed app.`);
