import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const requiredTools = [
  "switchboard.list_context_packs",
  "switchboard.build_context_pack",
  "switchboard.get_repo_graph_summary",
  "repo_context_pack",
  "repo_symbol_lookup",
  "repo_dependents_of",
];

function fixture(generatedAt) {
  return {
    schemaVersion: 1,
    generatedAt,
    kind: "mac_ai_switchboard.local_repo_memory_mcp_validation",
    releaseGateEvidence: false,
    readOnly: true,
    modifiesRepository: false,
    passed: true,
    relaunchSurvivalEvidence: "app-managed descriptor smoke recheck",
    connectorBridgeRecipesVerified: true,
    budgetedPackVerified: true,
    graphQueriesVerified: true,
    staleIndexHealthVerified: true,
    expectedToolsPresent: true,
    tools: requiredTools,
    steps: [{ id: "repo-memory-mcp-read-only-smoke", ok: true, status: 0 }],
  };
}

test("accepts fresh local Repo Memory MCP evidence", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-mcp-summary-"));
  try {
    const reportPath = path.join(tempDir, "fresh.json");
    fs.writeFileSync(reportPath, JSON.stringify(fixture(new Date().toISOString())));
    execFileSync(process.execPath, ["scripts/check-local-repo-memory-mcp-summary.mjs", reportPath], { stdio: "pipe" });
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("rejects stale, missing, and timezone-free local evidence timestamps", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-mcp-summary-time-"));
  try {
    const reportPath = path.join(tempDir, "invalid.json");
    for (const generatedAt of [
      "2020-01-01T00:00:00Z",
      "2026-08-21T00:00:00",
      undefined,
    ]) {
      const report = fixture(generatedAt);
      if (generatedAt === undefined) delete report.generatedAt;
      fs.writeFileSync(reportPath, JSON.stringify(report));
      assert.throws(() => execFileSync(process.execPath, ["scripts/check-local-repo-memory-mcp-summary.mjs", reportPath], { stdio: "pipe" }));
    }
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});
