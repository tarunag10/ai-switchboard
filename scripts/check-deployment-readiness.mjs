import fs from "node:fs";

const requiredFiles = [
  ".env.example",
  "docs/install.md",
  "docs/macos-release.md",
  "docs/beta-smoke-test.md",
  "docs/codex-compression-troubleshooting.md",
  "scripts/build-macos-dmg.sh",
"scripts/verify-release.sh",
"scripts/check-release-env.mjs",
"scripts/check-deployment-readiness.mjs",
  "scripts/smoke-preflight.mjs",
  ".github/workflows/release-macos.yml",
  ".github/workflows/release-macos-staging.yml",
];

const requiredScripts = {
  "package.json": [
    '"build:mac:dmg"',
'"release:check"',
'"release:env"',
'"smoke:preflight"',
    '"check:colors"',
    '"check:governance"',
    '"check:deployment"',
    '"fmt:desktop"',
  ],
"scripts/verify-release.sh": [
"npm run check:deployment",
"node scripts/check-release-env.mjs --strict",
"npm run smoke:preflight",
    "npm run check:colors",
    "npm run check:governance",
    "npm run build",
    "npm run test:coverage",
    "npm run fmt:desktop",
    "npm run test:desktop",
  ],
};

const requiredDocSignals = {
  ".env.example": [
    'HEADROOM_LOCAL_ONLY="1"',
    'VITE_HEADROOM_LOCAL_ONLY="1"',
    'VITE_HEADROOM_REMOTE_TELEMETRY="0"',
    'VITE_CLARITY_PROJECT_ID=""',
    "# Optional: app updater configuration for signed release builds",
    "# Optional: local signed macOS DMG builds",
  ],
  "README.md": [
 "Read-only local repo index, context packs, persisted summary, Doctor warnings, and clear/copy UI",
 "Read-only foundation",
 "the app now ships a read-only foundation",
 "Remaining work is the full Graphy-style symbol graph",
 "Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, Goose",
 ],
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
"npm run release:env",
"release environment preflight",
"notarization",
    "release-macos.yml",
    "release-macos-staging.yml",
    "staging-rolling",
    "latest.json",
    "GitHub Releases",
    "npm run smoke:preflight",
    "Rust formatting",
    "a production frontend build",
    "handing a DMG to a tester",
  ],
  "docs/beta-smoke-test.md": [
    "Local-only",
    "Mode buttons",
    "Doctor repairs missing RTK",
    "Oversized Codex compression refusal",
    "Codex model/provider mismatch",
    "Planned connectors are visible but manual",
    "copyable manual setup guide",
    "Launcher auto-setup and proxy verification should include only managed connectors",
    "Mac AI Switchboard.app",
    "Pause / resume",
    "Codex traffic is actively optimized",
    "Copy pack",
    "bounded Markdown context pack",
  ],
  "docs/codex-compression-troubleshooting.md": [
    "compression_refused",
    "RTK only",
    "Reset Codex",
    "multiple active chats",
    "The '' model is not supported",
  ],
  "docs/architecture.md": [
    "src-tauri/src/repo_intelligence.rs",
    "repo-intelligence-latest.json",
    "User repositories are not modified",
    "does not yet provide a full Graphy-style symbol graph",
  ],
};

const requiredSourceSignals = {
  "src/lib/localMode.ts": [
    "VITE_HEADROOM_LOCAL_ONLY",
    "VITE_HEADROOM_REMOTE_TELEMETRY",
    "!localOnlyModeEnabled()",
  ],
  "src/lib/analytics.ts": ["remoteTelemetryEnabled()"],
  "src/lib/bootstrapSentry.ts": ["remoteTelemetryEnabled()"],
  "src/lib/trayHelpers.ts": [
    'view === "upgrade"',
    'view === "upgradeAuth"',
    'return view === "upgrade" || view === "upgradeAuth" ? "home" : view',
  ],
  "src/lib/trayHelpers.test.ts": [
    "redirects upgrade views to home in local-only mode",
    "redirects auth notification actions to home in local-only mode",
  ],
  "src/lib/analytics.test.ts": [
    "does not track analytics events in local-only mode",
  ],
  "src/lib/bootstrapSentry.test.ts": [
    "does not report bootstrap failures in local-only mode",
  ],
  "src/lib/uninstallDisclosure.ts": [
    "Remove managed routing hooks and environment changes",
    "Delete managed hook scripts and shell-profile blocks",
    "repo-intelligence-latest.json",
    "User repositories are not modified",
    "Use Off mode instead if you only want to stop routing without deleting runtime files",
  ],
  "src/lib/uninstallDisclosure.test.ts": [
    "lists the reversible local footprint removed by uninstall",
    "keeps stable ids for modal rendering",
  ],
  "src-tauri/src/lib.rs": [
    "planned_connectors_detected",
    "ClientConnectorSupportStatus::Planned",
    "ClientConnectorSupportStatus::Managed",
    "automatic routing is disabled until backup, restore, and off-mode cleanup are implemented",
  ],
  "src-tauri/src/models.rs": [
    "setup_phase",
    "setup_hint",
  ],
  "src-tauri/src/client_adapters.rs": [
    "planned_connector_setup_phase",
    "planned_connector_setup_hint",
    "Automatic reversible setup, verification, repair, and off-mode cleanup are supported.",
  ],
  "src/lib/dashboardHelpers.ts": [
    "connectorSupportsAutomaticSetup",
    "connector.setupHint",
    "connector.setupPhase",
  ],
  "src/lib/repoIntelligence.ts": [
    "formatRepoContextPackMarkdown",
    "Repo Intelligence Context Pack",
    "Estimated savings vs full scan",
  ],
  "src/lib/repoIntelligence.test.ts": [
    "formats bounded context packs for agent handoff",
    "Repo Intelligence Context Pack",
  ],
  "scripts/smoke-preflight.mjs": [
    "Planned connectors are visible but manual",
    "copyable manual setup guide",
    "copy bounded context pack",
    "Installed app present",
    "dist/smoke-preflight-summary.md",
    "Required Installed-App Smoke Areas",
    "docs/beta-smoke-test.md",
  ],
};

const workflowSignals = {
  ".github/workflows/release-macos.yml": [
    "branches:",
    "- main",
    "HEADROOM_UPDATER_PUBLIC_KEY",
    "HEADROOM_UPDATER_ENDPOINTS",
    "HEADROOM_UPDATER_STAGING_ENDPOINTS",
    "TAURI_SIGNING_PRIVATE_KEY",
    "APPLE_SIGNING_IDENTITY",
    "HEADROOM_ACCOUNT_API_BASE_URL",
    "npm ci",
    "Run release checks",
    "./scripts/verify-release.sh",
    "tauri-apps/tauri-action",
    "latest.json",
    "releases/latest/download/latest.json",
  ],
  ".github/workflows/release-macos-staging.yml": [
    "branches:",
    "- staging",
    "staging-rolling",
    "HEADROOM_UPDATER_PUBLIC_KEY",
    "HEADROOM_UPDATER_ENDPOINTS",
    "HEADROOM_UPDATER_STAGING_ENDPOINTS",
    "TAURI_SIGNING_PRIVATE_KEY",
    "APPLE_SIGNING_IDENTITY",
    "HEADROOM_ACCOUNT_API_BASE_URL",
    "npm ci",
    "Run release checks",
    "./scripts/verify-release.sh",
    "tauri-apps/tauri-action",
    "latest.json",
    "releases/download/staging-rolling/latest.json",
  ],
};

const dmgScriptSignals = {
  "scripts/build-macos-dmg.sh": [
    "require_env APPLE_SIGNING_IDENTITY",
    "require_env TAURI_SIGNING_PRIVATE_KEY",
    "require_env TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
    "prepare_notarization",
    "npx tauri build --bundles dmg --ci",
    "Mac-AI-Switchboard_",
    "rename_built_dmg",
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
  "src/lib/uninstallDisclosure.ts": [
    "hooks environment changes",
    "scripts shell-profile blocks",
    "backup files created next edited configs",
    "Off mode is safer you only want",
  ],
  "src/lib/plannedAddons.ts": [
    "memory layer symbols",
    "reads help agents",
    "when tool stable config surface",
    "consistent Claude Code Codex",
    "RTK future Repo Intelligence",
  ],
"src/lib/plannedConnectors.ts": [
"only provider configuration supports",
"after stable local CLI surface",
"Track separately generic",
"Local config adapter explicit backup restore",
"Switchboard has stable connector capability model",
],
"src/App.tsx": [
"RTK are optional: install them",
"in your shell profiles",
"tracked planned adapter",
"command-output savings.",
"provider config backup restore",
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

for (const [path, signals] of Object.entries(requiredSourceSignals)) {
  if (!requireFile(path)) continue;
  const body = read(path);
  for (const signal of signals) {
    if (!body.includes(signal)) {
      failures.push(`${path} missing local-first source signal: ${signal}`);
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

for (const [path, signals] of Object.entries(dmgScriptSignals)) {
  if (!requireFile(path)) continue;
  const body = read(path);
  for (const signal of signals) {
    if (!body.includes(signal)) {
      failures.push(`${path} missing DMG build signal: ${signal}`);
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
