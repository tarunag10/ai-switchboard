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
  "scripts/release-readiness-report.mjs",
  "scripts/check-release-report-schema.mjs",
  "scripts/check-deployment-readiness.mjs",
  "scripts/smoke-preflight.mjs",
  "scripts/installed-smoke-summary.mjs",
  "scripts/repo-intelligence.mjs",
  ".github/workflows/rust-tauri.yml",
  ".github/workflows/release-macos.yml",
  ".github/workflows/release-macos-staging.yml",
];

const requiredScripts = {
  "package.json": [
    '"build:mac:dmg"',
'"release:check"',
    '"release:env"',
    '"release:env:json"',
    '"release:report"',
    '"release:report:check"',
    '"smoke:preflight"',
    '"smoke:installed"',
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
    "CLI now exposes an agent-readable `--manifest`",
    "Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, Zed AI",
 ],
 "docs/install.md": [
    "Mac-AI-Switchboard_<version>.dmg",
    "Applications",
    "local-first, not offline-only",
    "Full optimization",
    "RTK only",
    "Off mode",
    "Shareable Build Checklist",
    "Rust backend validation ready",
    "HEADROOM_UPDATER_PUBLIC_KEY",
    "HEADROOM_UPDATER_ENDPOINTS",
    "dist/smoke-preflight-summary.md",
    "installed app",
    "degraded-mode Doctor guidance",
    "planned connector automation gates",
    "manual workflow",
    "Repo Intelligence recipes",
    "per-tool agent handoffs",
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
    "/Applications/Mac AI Switchboard.app",
    "Treat the build as blocked",
    "Rust formatting",
    "a production frontend build",
    "handing a DMG to a tester",
    "degraded-mode Doctor guidance",
    "planned connector automation gates and manual workflow",
    "Repo Intelligence agent handoffs",
  ],
  "docs/beta-smoke-test.md": [
    "Local-only",
    "Mode buttons",
    "Requested mode vs active mode is honest",
    "degraded-mode guidance",
    "re-run Doctor until requested mode becomes active",
    "Doctor repairs missing RTK",
    "Oversized Codex compression refusal",
    "Codex model/provider mismatch",
    "Planned connectors are visible but manual",
"backend detection evidence",
"Automation gates",
"Manual workflow",
    "RTK only or Repo packs",
    "copyable manual setup guide",
    "Launcher auto-setup and proxy verification should include only managed connectors",
    "Mac AI Switchboard.app",
    "Pause / resume",
    "Codex traffic is actively optimized",
    "Shareable DMG gates",
    "backend validation",
    "signing/notarization",
    "Copy pack",
    "Clear index",
    "Re-indexing remains a deliberate Addons action",
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
    "--pack implementation --format markdown",
    "Default packs exclude secret-like paths",
    "User repositories are not modified",
    "now surfaces dependency hubs",
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
    "infer_switchboard_mode",
    "infer_switchboard_mode(&runtime, enabled_clients.len())",
    "infer_switchboard_mode(&runtime, enabled_clients)",
    "planned_connectors_detected",
    "ClientConnectorSupportStatus::Planned",
    "ClientConnectorSupportStatus::Managed",
    "planned_connector_doctor_body",
    "keeps routing manual until backup, restore, and Off mode cleanup are implemented",
    "Backend checks:",
    "Config locations watched:",
"Detection evidence:",
"Automation gates:",
"Manual workflow:",
"Safe today: use RTK-only mode or Repo Intelligence packs",
  ],
  "src-tauri/src/models.rs": [
    "setup_phase",
    "setup_hint",
    "detection_sources",
"detection_evidence",
"config_locations",
"automation_gates",
"manual_workflow",
],
  "src-tauri/src/client_adapters.rs": [
    "struct PlannedClientSpec",
    "detection_sources",
"detection_evidence",
"config_locations",
"automation_gates",
"manual_workflow",
"planned_connector_registry_includes_backend_detection_metadata",
    "Automatic reversible setup, verification, repair, and off-mode cleanup are supported.",
  ],
  "src/lib/dashboardHelpers.ts": [
    "connectorSupportsAutomaticSetup",
    "connector.setupHint",
    "connector.setupPhase",
  ],
  "src/lib/types.ts": [
    "automationGates",
    "manualWorkflow",
  ],
  "src/App.tsx": [
    "connector.automationGates",
    "connector.manualWorkflow",
    "Gates",
    "Manual",
  ],
  "src/lib/doctorRepairCopy.ts": [
    "doctorIssueGuidance",
    "switchboard_mode_degraded",
    "re-run Doctor until requested mode becomes active",
    "planned_connectors_detected",
    "repo_intelligence_repo_missing",
    "headroom_paused",
  ],
"src/components/SwitchboardDoctorPanel.manual.test.tsx": [
    "Repair all will leave manual steps visible",
    "1 automatic",
    "1 manual",
"separates manual connector guidance from automatic Repo Intelligence cleanup",
    "Open Settings, review detected evidence and each planned connector guide",
"saved Repo Intelligence summary",
],
"src/lib/releaseReadiness.ts": [
  "releaseShareableGates",
  "releaseReadinessGroups",
  "planned connector safety evidence",
  "automation gates",
  "manual workflow",
  "dist/smoke-preflight-summary.md",
  "dist/installed-smoke-summary.md",
],
"src/lib/releaseReadiness.test.ts": [
  "automation gates",
  "manual workflow",
  "planned connector evidence",
],
"src-tauri/src/models.rs": [
    "RepoGraphSummary",
    "RepoGraphNode",
    "pub graph: Option<RepoGraphSummary>",
    "pub dependency_hubs",
  ],
  "src-tauri/src/repo_intelligence.rs": [
    "build_repo_graph_summary",
    "builds_repo_graph_summary_for_agent_context",
    "top_directories",
    "config_hubs",
    "dependency_hubs",
    "is_dependency_hub",
  ],
  "src/App.tsx": [
    "Copy agent manifest",
    "repo-intelligence-recipes",
    "repo-intelligence-handoffs",
    "Agent handoffs",
    "Agent recipes",
    "Copy recipe pack",
    "releaseReadinessGroups",
    "releaseShareableGates",
    "Shareable DMG gates",
    "detectionEvidence",
    "Evidence",
    "Release readiness",
    "repo-intelligence-graph",
    "Repo Intelligence graph summary",
    "Top directories",
    "Likely tests",
  ],
  "src/styles.css": [
    ".switchboard-panel__footprint",
    ".release-readiness-card",
    ".repo-intelligence-graph",
    "grid-template-columns: repeat(4, minmax(0, 1fr))",
  ],
  "src/lib/repoIntelligence.ts": [
    "formatRepoContextPackMarkdown",
    "isSecretLikeRepoPath",
    "Repo Intelligence Context Pack",
    "RepoGraphSummary",
    "buildRepoGraphSummary",
    "Repo Graph Summary",
    "Estimated savings vs full scan",
    "agentRecipes",
    "Gemini CLI",
    "Aider",
    "Qwen Code",
    "Amazon Q Developer CLI",
    "Windsurf",
    "Zed AI",
    "provider routing remains manual",
  ],
  "src/lib/repoIntelligence.test.ts": [
    "formats an agent-readable manifest",
    "formats bounded context packs for agent handoff",
    "builds a bounded repo graph summary for agent context",
    "Repo Intelligence Context Pack",
    "Repo Graph Summary",
    "cli_implementation",
    "editor_context",
  ],
  "scripts/smoke-preflight.mjs": [
    "Planned connectors are visible but manual",
    "backend detection evidence",
    "planned connector manual warnings with detection evidence",
    "safe RTK-only or Repo Intelligence pack guidance",
    "copyable manual setup guide",
    "copy bounded context pack",
 "copy individual task pack",
    "Installed app present",
    "dist/smoke-preflight-summary.md",
"Required Installed-App Smoke Areas",
"docs/beta-smoke-test.md",
],
"scripts/repo-intelligence.mjs": [
    "--pack <id>",
    "--agent <id>",
    "--format <format>",
    "--manifest",
    "mac_ai_switchboard.repo_intelligence_manifest",
"excludesSecretLikePaths",
    "formatSinglePackMarkdown",
    "secretPathPatterns",
    "agentRecipes",
    "agentHandoffProfiles",
    "formatAgentHandoffMarkdown",
    "Claude Code",
    "Codex",
    "Gemini CLI",
    "Grok / xAI CLI",
    "Qwen Code",
    "Amazon Q Developer CLI",
    "Windsurf",
    "Zed AI",
    "provider routing remains manual",
    "Available packs",
  ],
  "scripts/check-release-env.mjs": [
    "jsonOutput",
    "--json",
    "ok: blockers.length === 0",
    "blockers",
    "warnings",
  ],
  "scripts/release-readiness-report.mjs": [
    "backendValidation",
    "buildBackendValidation",
    "staticSmokePreflight",
    "staticSmokeRequiredEvidence",
    "Planned connector automation gates",
    "Planned connector manual workflow",
    "buildStaticSmokePreflight",
    "installedSmokeSummary",
    "installedSmokeSummaryPath",
    "installedSmokeRequiredEvidence",
    "installedSmoke",
    "buildInstalledSmoke",
    "missingEvidence",
    "evidenceReady",
    "shareableDmgGate",
    "buildShareableDmgGate",
    "Shareable DMG Gates",
    "updaterFeedReady",
    "staticSmokePreflightReady",
    "npm run fmt:desktop",
    "npm run test:desktop",
    "unblockCommands",
    "rustup target add aarch64-apple-darwin x86_64-apple-darwin",
    "Backend validation pending",
  ],
  "scripts/check-release-report-schema.mjs": [
    "dist/release-readiness-report.json",
    "backendValidation.requiredCommands",
    "backendValidation.unblockCommands",
    "staticSmokePreflight.smokeSummaryPresent",
    "staticSmokePreflight.requiredCommand",
    "staticSmokePreflight.requiredEvidence",
    "planned connector automation gates",
    "planned connector manual workflow",
    "installedSmokeSummary.present",
    "installedSmoke.smokeSummaryPresent",
    "installedSmoke.requiredEvidence",
    "installedSmoke.missingEvidence",
    "installedSmoke.evidenceReady",
    "shareableDmgGate.staticSmokePreflightReady",
    "shareableDmgGate.updaterFeedReady",
    "releaseEnv.blockers",
    "releaseEnv.warnings",
  ],
  ".github/workflows/rust-tauri.yml": [
    "Rust Tauri Validation",
    "tarun/local-switchboard",
    "npm run fmt:desktop",
    "cargo nextest run --manifest-path src-tauri/Cargo.toml",
    "libwebkit2gtk-4.1-dev",
  ],
  "scripts/installed-smoke-summary.mjs": [
    "dist/installed-smoke-summary.md",
    "/Applications/Mac AI Switchboard.app",
    "docs/beta-smoke-test.md",
    "npm run smoke:installed",
    "--confirm",
    "MAC_AI_SWITCHBOARD_INSTALLED_SMOKE_PASSED",
    "explicit tester confirmation received",
    "Contents",
    "Info.plist",
    "Installed app metadata present",
    "Confirmed Evidence Areas",
    "Switchboard modes and degraded-mode Doctor guidance",
    "Planned connector automation gates and manual workflow",
    "Per-tool agent handoffs",
    "Codex compression recovery",
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
