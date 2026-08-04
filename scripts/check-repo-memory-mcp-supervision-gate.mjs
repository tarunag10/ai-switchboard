#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const fixturePath = path.join(root, "fixtures/repo-memory-mcp-supervision-evidence.json");

const requiredSignals = [
  {
    label: "repo memory MCP supervision summary",
    file: "src/lib/repoMemoryMcpSupervision.ts",
    needles: ["deriveRepoMemoryMcpSupervisionSummary", "os_daemon_not_supported"],
  },
  {
    label: "repo memory MCP doctor warnings",
    file: "src-tauri/src/doctor.rs",
    needles: ["repo_memory_mcp_doctor_issue", "repo_memory_mcp_relaunch_failed"],
  },
  {
    label: "repo memory MCP runtime relaunch fields",
    file: "src-tauri/src/state/runtime_lifecycle.rs",
    needles: ["repo_memory_mcp_relaunch_survival_status", "repo_memory_mcp_supervision_scope"],
  },
  {
    label: "repo memory MCP supervision card",
    file: "src/components/RepoMemoryMcpSupervisionCard.tsx",
    needles: ["deriveRepoMemoryMcpSupervisionSummary", "Refresh supervision"],
  },
];

function fail(message) {
  console.error(`repo memory MCP supervision gate check failed: ${message}`);
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

if (!fs.existsSync(fixturePath)) {
  fail("missing fixtures/repo-memory-mcp-supervision-evidence.json");
}

const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
if (fixture.osDaemonSurvivalClaimed !== false) {
  fail("repo memory MCP fixture must not claim OS daemon survival");
}

console.log(
  JSON.stringify(
    {
      ok: true,
      supervisionScope: fixture.supervisionScope,
      osDaemonSurvivalClaimed: fixture.osDaemonSurvivalClaimed,
      signals: requiredSignals.map((signal) => signal.label),
    },
    null,
    2,
  ),
);
