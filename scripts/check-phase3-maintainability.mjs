#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const root = process.cwd();

const godFiles = [
  "src-tauri/src/client_adapters.rs",
  "src/App.tsx",
  "src/styles.css",
];

const requiredSignals = [
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
    needles: ["MAX_LINES", "MAX_BYTES"],
  },
  {
    label: "benchmark leaderboard export",
    file: "scripts/export-benchmark-leaderboard.mjs",
    needles: ["fixtures.json", "leaderboard"],
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
  fail(`file size budget failed: ${error.stderr || error.message}`);
}

console.log(
  JSON.stringify(
    {
      ok: true,
      godFiles,
      signals: requiredSignals.map((signal) => signal.label),
    },
    null,
    2,
  ),
);
