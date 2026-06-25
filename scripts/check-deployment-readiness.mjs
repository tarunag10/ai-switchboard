import fs from "node:fs";

const requiredFiles = [
  "docs/install.md",
  "docs/macos-release.md",
  "docs/beta-smoke-test.md",
  "docs/codex-compression-troubleshooting.md",
  "scripts/build-macos-dmg.sh",
  "scripts/verify-release.sh",
  ".github/workflows/release-macos.yml",
  ".github/workflows/release-macos-staging.yml",
];

const requiredScripts = {
  "package.json": [
    '"build:mac:dmg"',
    '"release:check"',
    '"check:colors"',
    '"check:governance"',
    '"check:deployment"',
  ],
  "scripts/verify-release.sh": [
    "npm run check:deployment",
    "npm run check:colors",
    "npm run check:governance",
    "npm run build",
    "npm run test:coverage",
    "npm run test:desktop",
  ],
};

const requiredDocSignals = {
  "docs/install.md": [
    "Mac-AI-Switchboard_<version>.dmg",
    "Applications",
    "local-first, not offline-only",
    "Full optimization",
    "RTK only",
    "Off mode",
    "413 Payload Too Large",
  ],
  "docs/macos-release.md": [
    "HEADROOM_UPDATER_PUBLIC_KEY",
    "HEADROOM_UPDATER_ENDPOINTS",
    "APPLE_SIGNING_IDENTITY",
    "notarization",
    "release-macos.yml",
    "release-macos-staging.yml",
    "staging-rolling",
  ],
  "docs/beta-smoke-test.md": [
    "Local-only",
    "Mode buttons",
    "Doctor repairs missing RTK",
    "Oversized Codex compression refusal",
    "Codex model/provider mismatch",
    "Mac AI Switchboard.app",
    "Pause / resume",
    "Codex traffic is actively optimized",
  ],
  "docs/codex-compression-troubleshooting.md": [
    "compression_refused",
    "RTK only",
    "Reset Codex",
    "multiple active chats",
    "The '' model is not supported",
  ],
};

const workflowSignals = {
  ".github/workflows/release-macos.yml": [
    "HEADROOM_UPDATER_PUBLIC_KEY",
    "TAURI_SIGNING_PRIVATE_KEY",
    "APPLE_SIGNING_IDENTITY",
  ],
  ".github/workflows/release-macos-staging.yml": [
    "staging-rolling",
    "HEADROOM_UPDATER_PUBLIC_KEY",
    "TAURI_SIGNING_PRIVATE_KEY",
  ],
};

const forbiddenUserCopy = {
  "src-tauri/src/lib.rs": [
    "The local proxy not answering",
    "compression oversized Codex",
    "Codex temporarily going direct",
    "This cause model errors",
    "empty unsupported model",
    "currently configured use Headroom",
    "return connect it",
    "RTK required for requested",
  ],
};

const failures = [];

function read(path) {
  return fs.readFileSync(path, "utf8");
}

function requireFile(path) {
  if (!fs.existsSync(path)) {
    failures.push(`Missing ${path}`);
    return false;
  }
  return true;
}

for (const path of requiredFiles) {
  requireFile(path);
}

for (const [path, signals] of Object.entries(requiredScripts)) {
  if (!requireFile(path)) continue;
  const body = read(path);
  for (const signal of signals) {
    if (!body.includes(signal)) {
      failures.push(`${path} missing deployment script signal: ${signal}`);
    }
  }
}

for (const [path, signals] of Object.entries(requiredDocSignals)) {
  if (!requireFile(path)) continue;
  const body = read(path);
  for (const signal of signals) {
    if (!body.includes(signal)) {
      failures.push(`${path} missing deployment doc signal: ${signal}`);
    }
  }
}

for (const [path, signals] of Object.entries(workflowSignals)) {
  if (!requireFile(path)) continue;
  const body = read(path);
  for (const signal of signals) {
    if (!body.includes(signal)) {
      failures.push(`${path} missing release workflow signal: ${signal}`);
    }
  }
}

for (const [path, phrases] of Object.entries(forbiddenUserCopy)) {
  if (!requireFile(path)) continue;
  const body = read(path);
  for (const phrase of phrases) {
    if (body.includes(phrase)) {
      failures.push(`${path} contains rough user-facing copy: ${phrase}`);
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("Deployment readiness docs, scripts, and workflows are linked.");
