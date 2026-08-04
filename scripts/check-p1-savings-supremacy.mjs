#!/usr/bin/env node
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

const requiredSignals = [
  {
    label: "golden benchmark suite",
    file: "benchmarks/fixtures.json",
    minFixtures: 8,
  },
  {
    label: "benchmark schema gate",
    file: "scripts/check-world-class-benchmarks.mjs",
  },
  {
    label: "exact cache default policy",
    file: "src/lib/exactCacheDefaultPolicy.ts",
    needles: ["recommendExactCacheDefault"],
  },
  {
    label: "switchboard mode resolver for cache",
    file: "src/lib/switchboardModeForCache.ts",
    needles: ["resolveSwitchboardModeForCache"],
  },
  {
    label: "leanctx promotion gate",
    file: "src/lib/leanctxPromotionGate.ts",
    needles: ["resolveMasterActivationLocalOptimizations"],
  },
  {
    label: "leanctx promotion fixture",
    file: "fixtures/leanctx-promotion-evidence.json",
  },
  {
    label: "provider-billed counterfactual frontend",
    file: "src/lib/providerBilledCounterfactual.ts",
    needles: ["validateProviderBilledCounterfactual", "recordProviderBilledCounterfactual"],
  },
  {
    label: "provider-billed counterfactual backend",
    file: "src-tauri/src/provider_billed_counterfactual.rs",
    needles: [
      "build_provider_billed_attribution_event",
      "extract_codex_billed_input_tokens",
      "extract_claude_billed_input_tokens",
    ],
  },
];

function fail(message) {
  console.error(`P1 savings supremacy check failed: ${message}`);
  process.exit(1);
}

function run(command) {
  return execSync(command, { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

for (const signal of requiredSignals) {
  const absolute = path.join(root, signal.file);
  if (!fs.existsSync(absolute)) {
    fail(`missing ${signal.file}`);
  }
  if (signal.needles) {
    const contents = fs.readFileSync(absolute, "utf8");
    for (const needle of signal.needles) {
      if (!contents.includes(needle)) {
        fail(`${signal.label} missing needle ${needle} in ${signal.file}`);
      }
    }
  }
  if (signal.minFixtures) {
    const fixtures = JSON.parse(fs.readFileSync(absolute, "utf8"));
    if (!Array.isArray(fixtures) || fixtures.length < signal.minFixtures) {
      fail(`${signal.label} requires at least ${signal.minFixtures} fixtures`);
    }
  }
}

const benchmarks = JSON.parse(run("node scripts/check-world-class-benchmarks.mjs"));
const leanctxGate = JSON.parse(run("node scripts/check-leanctx-promotion-gate.mjs"));

console.log(
  JSON.stringify(
    {
      ok: true,
      phase: "P1",
      title: "Savings supremacy",
      benchmarks,
      leanctxGate,
      signals: requiredSignals.map((signal) => signal.label),
    },
    null,
    2,
  ),
);
