#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { loadGodFileRegistry } from "./lib/god-file-registry.mjs";

const root = process.cwd();
const registry = loadGodFileRegistry(root);
const godFiles = registry.godFiles.map((entry) => entry.path);

const requiredSignals = [
  {
    label: "god file registry fixture",
    file: "fixtures/god-file-registry.json",
    needles: ["splitModules", "watchlist", "maxGrowthLines", "P3.3 complete"],
  },
  {
    label: "god file registry loader",
    file: "scripts/lib/god-file-registry.mjs",
    needles: ["evaluateGodFileRegistry", "trackedOversizePathSet"],
  },
  {
    label: "god file registry gate",
    file: "scripts/check-god-file-registry.mjs",
    needles: ["evaluateGodFileRegistry", "lineCeiling"],
  },
  {
    label: "god file splits gate",
    file: "scripts/check-god-file-splits.mjs",
    needles: ["splitModules", "originalBaselines"],
  },
  {
    label: "repo memory relaunch supervision",
    file: "src-tauri/src/state/repo_memory_mcp.rs",
    needles: [
      "verify_repo_memory_mcp_on_app_relaunch",
      "relaunch_survival_status",
      "relaunch_verified",
    ],
  },
  {
    label: "frontend supervision summary",
    file: "src/lib/repoMemoryMcpSupervision.ts",
    needles: ["deriveRepoMemoryMcpSupervisionSummary", "relaunch_verified"],
  },
  {
    label: "file size budget script",
    file: "scripts/check-file-size-budget.mjs",
    needles: ["loadGodFileRegistry", "trackedOversizePathSet"],
  },
  {
    label: "benchmark leaderboard export",
    file: "scripts/export-benchmark-leaderboard.mjs",
    needles: ["fixtures.json", "leaderboard"],
  },
  {
    label: "god file registry typescript",
    file: "src/lib/godFileRegistry.ts",
    needles: ["describeGodFileRegistry", "godFileRegistry"],
  },
  {
    label: "cross-platform cli entrypoint",
    file: "bin/switchboard.mjs",
    needles: ["repo-intelligence", "optimize"],
  },
  {
    label: "platform support matrix",
    file: "docs/platform-support.md",
    needles: ["Linux", "Windows", "Repo-local preview"],
  },
  {
    label: "switchboard cli gate",
    file: "scripts/check-switchboard-cli.mjs",
    needles: ["repo-intelligence", "platform-support.md"],
  },
];

function fail(message) {
  console.error(`phase 3 maintainability check failed: ${message}`);
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

for (const file of godFiles) {
  const absolute = path.join(root, file);
  if (!fs.existsSync(absolute)) {
    fail(`missing tracked god file ${file}`);
  }
}

for (const script of [
  "scripts/check-god-file-registry.mjs",
  "scripts/check-god-file-splits.mjs",
  "scripts/check-switchboard-cli.mjs",
]) {
  try {
    execFileSync("node", [script], {
      cwd: root,
      stdio: "pipe",
      encoding: "utf8",
    });
  } catch (error) {
    fail(`${script} failed: ${error.stderr || error.message}`);
  }
}

try {
  execFileSync(
    "node",
    [
      "scripts/check-file-size-budget.mjs",
      "src-tauri/src/optimization",
      "src/lib/agentSessionPacks.ts",
      "src/lib/leanctxPromotionGate.ts",
      "src/lib/cursorNativeGate.ts",
      "src/lib/repoMemoryMcpSupervision.ts",
    ],
    {
      cwd: root,
      stdio: "pipe",
      encoding: "utf8",
    },
  );
} catch (error) {
  fail(`focused file size budget failed: ${error.stderr || error.message}`);
}

console.log(
  JSON.stringify(
    {
      ok: true,
      godFiles,
      defaultBudget: registry.defaultBudget,
      signals: requiredSignals.map((signal) => signal.label),
    },
    null,
    2,
  ),
);
