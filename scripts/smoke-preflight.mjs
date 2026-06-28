import fs from "node:fs";
import path from "node:path";

const betaSmokeDoc = "docs/beta-smoke-test.md";
const installDoc = "docs/install.md";
const releaseDoc = "docs/macos-release.md";
const appPath = "/Applications/Mac AI Switchboard.app";
const summaryPath = "dist/smoke-preflight-summary.md";
const releaseReportPath = "dist/release-readiness-report.md";

const requiredSignals = {
  [betaSmokeDoc]: [
    "Switchboard checks",
    "Copy state",
    "local footprint matrix",
    "Doctor triage shows automatic and manual counts",
    "Copy report",
    "degraded-mode guidance",
    "re-run Doctor until requested mode becomes active",
    "Planned connectors are visible but manual",
"backend detection evidence",
"Automation gates",
"Manual workflow",
"Gemini dry-run preview evidence",
    "RTK only or Repo packs",
    "Repair all will leave manual steps visible",
    "copyable manual setup guide",
"Copy agent manifest",
"Agent handoffs",
"mac_ai_switchboard.repo_agent_handoff",
"ready-to-paste bounded handoff",
    "Clear index",
    "Re-indexing remains a deliberate Addons action",
    "Release readiness visible in Settings",
"npm run smoke:installed -- --confirm",
"Contents/Info.plist",
    "Launcher auto-setup and proxy verification should include only managed connectors",
"Codex traffic is actively optimized",
"Pause / resume",
"Savings calculator",
"saved tokens",
"estimated dollars",
],
  [installDoc]: [
    "Mac-AI-Switchboard_<version>.dmg",
"Contents/Info.plist",
"npm run smoke:installed -- --confirm",
    "Full optimization",
    "RTK only",
    "Off mode",
    "Codex Compression Troubleshooting",
  ],
  "scripts/repo-intelligence.mjs": [
    "--manifest",
    "--agent <id>",
    "--session",
    "--task <type>",
    "--list-agents",
    "--format json",
    "mac_ai_switchboard.agent_session_preparation",
    "mac_ai_switchboard.repo_intelligence_manifest",
    "mac_ai_switchboard.repo_agent_handoff",
    "buildAgentSessionPreparation",
    "formatAgentHandoffMarkdown",
    "excludesSecretLikePaths",
  ],
 [releaseDoc]: [
    "npm run release:check",
    "Mac-AI-Switchboard_",
    "notarization",
"npm run smoke:installed -- --confirm",
    "staging-rolling",
  ],
"scripts/installed-smoke-summary.mjs": [
"--confirm",
"MAC_AI_SWITCHBOARD_INSTALLED_SMOKE_PASSED",
"explicit tester confirmation received",
"Contents",
"Info.plist",
],
};

const failures = [];

function read(pathname) {
  if (!fs.existsSync(pathname)) {
    failures.push(`Missing ${pathname}`);
    return "";
  }
  return fs.readFileSync(pathname, "utf8");
}

for (const [pathname, signals] of Object.entries(requiredSignals)) {
  const body = read(pathname);
  for (const signal of signals) {
    if (!body.includes(signal)) {
      failures.push(`${pathname} missing smoke signal: ${signal}`);
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

const installed = fs.existsSync(appPath);
const generatedAt = new Date().toISOString();
const summary = `# Smoke Preflight Summary

Generated: ${generatedAt}

- Static smoke coverage: pass
- Installed app present: ${installed ? "yes" : "no"} (${appPath})
- Installed-app checklist: ${betaSmokeDoc}
- Release instructions: ${releaseDoc}
- Release readiness report: ${releaseReportPath}
- Installed smoke evidence command: npm run smoke:installed -- --confirm

## Required Installed-App Smoke Areas

- Switchboard modes: Full optimization, Headroom only, RTK only, Off
- Doctor repairs: runtime, Codex setup, RTK, planned connector manual warnings with detection evidence, config creation plan, Gemini dry-run preview evidence, and Repo Intelligence stale/missing-index warnings
- Planned connectors: Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, Zed AI; each stays disabled and manual with safe RTK-only or Repo Intelligence pack guidance, copyable config creation plan, and Gemini shows binary/version/config compatibility evidence plus backend dry-run preview evidence when detected
- Repo Intelligence: local repo index, persisted latest summary, copy bounded context pack, copy individual task pack, copy agent manifest, copy per-tool Markdown and JSON agent handoffs, clear saved index, context-pack preview, per-pack copy
- Savings calculator: Session / Overall scopes, copyable confidence-labelled ledger, saved tokens, estimated dollars, reduction, equation, source breakdown
- Local-first behavior: remote services gated, Off mode reversible cleanup
- Codex resilience: compression refusal reset and model/provider repair

Next step: run npm run release:ready, install DMG, confirm ${appPath}/Contents/Info.plist exists, run ${betaSmokeDoc}, then run npm run smoke:installed -- --confirm.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, summary);

console.log("Smoke preflight passed.");
console.log(`Installed app present: ${installed ? "yes" : "no"} (${appPath})`);
console.log(`Summary written: ${summaryPath}`);
console.log(`Next: run npm run release:ready, install DMG, run ${betaSmokeDoc}, then run npm run smoke:installed -- --confirm.`);
