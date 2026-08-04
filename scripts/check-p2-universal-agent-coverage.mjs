#!/usr/bin/env node
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

const requiredSignals = [
  {
    label: "agent session pack recommendation",
    file: "src/lib/agentSessionPacks.ts",
    needles: ["recommendAgentSessionPackId", "resolveAgentSessionPreferredPackId", 'id: "cursor"'],
  },
  {
    label: "agent session panel",
    file: "src/components/AgentSessionPanel.tsx",
    needles: ["resolveAgentSessionPreferredPackId", "(recommended)"],
  },
  {
    label: "cursor native gate",
    file: "src/lib/cursorNativeGate.ts",
    needles: ["evaluateCursorNativePromotionGate", "describeCursorNativeGate"],
  },
  {
    label: "cursor native gate fixture",
    file: "fixtures/cursor-native-gate-evidence.json",
  },
  {
    label: "repo memory MCP supervision",
    file: "src/lib/repoMemoryMcpSupervision.ts",
    needles: ["deriveRepoMemoryMcpSupervisionSummary"],
  },
  {
    label: "repo memory MCP supervision fixture",
    file: "fixtures/repo-memory-mcp-supervision-evidence.json",
  },
  {
    label: "connector promotion gate",
    file: "src/lib/connectorPromotionGate.ts",
    needles: ["evaluateConnectorPromotionGate"],
  },
  {
    label: "connector promotion fixture",
    file: "fixtures/connector-promotion-evidence.json",
  },
];

function fail(message) {
  console.error(`P2 universal agent coverage check failed: ${message}`);
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
}

const cursorGate = JSON.parse(run("node scripts/check-cursor-native-gate.mjs"));
const connectorPromotion = JSON.parse(run("node scripts/check-connector-promotion-gate.mjs"));
const mcpSupervision = JSON.parse(
  run("node scripts/check-repo-memory-mcp-supervision-gate.mjs"),
);

console.log(
  JSON.stringify(
    {
      ok: true,
      phase: "P2",
      title: "Universal agent coverage",
      cursorGate,
      connectorPromotion,
      mcpSupervision,
      signals: requiredSignals.map((signal) => signal.label),
    },
    null,
    2,
  ),
);
