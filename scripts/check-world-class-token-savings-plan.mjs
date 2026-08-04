#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const planDir = path.join(root, "docs/world-class-token-savings");
const requiredFiles = [
  "README.md",
  "CURSOR-ANALYSIS-2026-07-30.md",
  "IMPLEMENTATION-PLAN.md",
  "slice-status.json",
];
const requiredCodeSignals = [
  {
    label: "proxy session auth module",
    file: "src-tauri/src/proxy_session_auth.rs",
    needles: ["ProxySessionAuth", "PROXY_SESSION_HEADER", "get_proxy_session_auth_status"],
  },
  {
    label: "mode inspector verdict",
    file: "src/lib/modeInspectorVerdict.ts",
    needles: ["deriveModeInspectorVerdict"],
  },
  {
    label: "exact cache default policy",
    file: "src/lib/exactCacheDefaultPolicy.ts",
    needles: ["recommendExactCacheDefault"],
  },
  {
    label: "leanctx promotion gate",
    file: "src/lib/leanctxPromotionGate.ts",
    needles: ["resolveMasterActivationLocalOptimizations", "evaluateLeanctxPromotionGate"],
  },
  {
    label: "provider-billed counterfactual",
    file: "src/lib/providerBilledCounterfactual.ts",
    needles: ["validateProviderBilledCounterfactual", "recordProviderBilledCounterfactual"],
  },
  {
    label: "switchboard mode resolver for cache",
    file: "src/lib/switchboardModeForCache.ts",
    needles: ["resolveSwitchboardModeForCache"],
  },
  {
    label: "agent session pack recommendation",
    file: "src/lib/agentSessionPacks.ts",
    needles: ["recommendAgentSessionPackId", "resolveAgentSessionPreferredPackId"],
  },
  {
    label: "connector promotion gate",
    file: "src/lib/connectorPromotionGate.ts",
    needles: ["evaluateConnectorPromotionGate"],
  },
  {
    label: "repo memory MCP supervision",
    file: "src/lib/repoMemoryMcpSupervision.ts",
    needles: ["deriveRepoMemoryMcpSupervisionSummary"],
  },
  {
    label: "cursor native write gate",
    file: "src/lib/cursorNativeGate.ts",
    needles: ["describeCursorNativeGate"],
  },
  {
    label: "repo memory MCP supervision",
    file: "src/lib/repoMemoryMcpSupervision.ts",
    needles: ["deriveRepoMemoryMcpSupervisionSummary"],
  },
];

function fail(message) {
  console.error(`world-class plan check failed: ${message}`);
  process.exit(1);
}

for (const file of requiredFiles) {
  const absolute = path.join(planDir, file);
  if (!fs.existsSync(absolute)) {
    fail(`missing ${absolute}`);
  }
}

const ledger = JSON.parse(
  fs.readFileSync(path.join(planDir, "slice-status.json"), "utf8"),
);
if (!ledger.phases?.P0 || !ledger.phases?.P1 || !ledger.phases?.P2 || !ledger.phases?.P3) {
  fail("slice-status.json must define P0, P1, P2, and P3 phases");
}

for (const signal of requiredCodeSignals) {
  const absolute = path.join(root, signal.file);
  if (!fs.existsSync(absolute)) {
    fail(`missing code surface for ${signal.label}: ${signal.file}`);
  }
  const contents = fs.readFileSync(absolute, "utf8");
  for (const needle of signal.needles) {
    if (!contents.includes(needle)) {
      fail(`${signal.label} missing needle ${needle} in ${signal.file}`);
    }
  }
}

const inProgress = Object.values(ledger.phases)
  .flatMap((phase) => phase.slices)
  .filter((slice) => slice.status === "in_progress").length;

console.log(
  JSON.stringify(
    {
      ok: true,
      planDir: "docs/world-class-token-savings",
      analysisDate: "2026-07-30",
      inProgressSlices: inProgress,
      requiredFiles,
      codeSignals: requiredCodeSignals.map((signal) => signal.label),
    },
    null,
    2,
  ),
);
