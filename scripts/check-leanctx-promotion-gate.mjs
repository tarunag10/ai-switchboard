#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

const requiredSignals = [
  {
    label: "leanctx promotion evidence",
    file: "src-tauri/src/tool_manager/leanctx.rs",
    needles: [
      "LeanctxPromotionEvidence",
      "evaluate_promotion_capabilities",
      "live_request_routing: false",
    ],
  },
  {
    label: "optimization addon readiness",
    file: "src-tauri/src/optimization_addons_readiness.rs",
    needles: ["get_optimization_addon_readiness", '"leanctx"'],
  },
  {
    label: "frontend promotion gate",
    file: "src/lib/leanctxPromotionGate.ts",
    needles: [
      "evaluateLeanctxPromotionGate",
      "resolveMasterActivationLocalOptimizations",
      "leanctx-shadow",
    ],
  },
];

function fail(message) {
  console.error(`leanctx promotion gate check failed: ${message}`);
  process.exit(1);
}

for (const signal of requiredSignals) {
  const absolute = path.join(root, signal.file);
  if (!fs.existsSync(absolute)) {
    fail(`missing ${signal.file}`);
  }
  const contents = fs.readFileSync(absolute, "utf8");
  for (const needle of signal.needles) {
    if (!contents.includes(needle)) {
      fail(`${signal.label} missing needle ${needle} in ${signal.file}`);
    }
  }
}

const leanctxSource = fs.readFileSync(
  path.join(root, "src-tauri/src/tool_manager/leanctx.rs"),
  "utf8",
);
if (!leanctxSource.includes("live_request_routing: false")) {
  fail("leanctx must keep live_request_routing disabled");
}

const gateSource = fs.readFileSync(
  path.join(root, "src/lib/leanctxPromotionGate.ts"),
  "utf8",
);
if (!gateSource.includes('MASTER_ACTIVATION_LEANCTX_SHADOW_ID = "leanctx-shadow"')) {
  fail("master activation leanctx-shadow id is missing");
}

console.log(
  JSON.stringify(
    {
      ok: true,
      signals: requiredSignals.map((signal) => signal.label),
      liveProviderRouting: "blocked",
    },
    null,
    2,
  ),
);
