#!/usr/bin/env node

import fs from "node:fs";

const reportPath = "dist/local-repo-memory-mcp-validation-summary.json";
const requiredTools = [
  "switchboard.list_context_packs",
  "switchboard.build_context_pack",
  "switchboard.get_repo_graph_summary",
  "repo_context_pack",
  "repo_symbol_lookup",
  "repo_dependents_of",
];

function fail(message) {
  console.error(`repo-memory MCP summary check failed: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} missing; run npm run smoke:repo-memory-mcp:local first`);
}

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));

if (report.schemaVersion !== 1) {
  fail("schemaVersion must be 1");
}
if (report.kind !== "mac_ai_switchboard.local_repo_memory_mcp_validation") {
  fail("unexpected report kind");
}
if (report.releaseGateEvidence !== false) {
  fail("releaseGateEvidence must remain false for local MCP evidence");
}
if (report.readOnly !== true) {
  fail("readOnly must be true");
}
if (report.modifiesRepository !== false) {
  fail("modifiesRepository must be false");
}
if (report.passed !== true) {
  fail("report.passed must be true");
}
if (!String(report.relaunchSurvivalEvidence || "").includes("app-managed descriptor smoke recheck")) {
  fail("missing app-managed descriptor recheck evidence");
}

for (const field of [
  "connectorBridgeRecipesVerified",
  "budgetedPackVerified",
  "graphQueriesVerified",
  "staleIndexHealthVerified",
  "expectedToolsPresent",
]) {
  if (report[field] !== true) {
    fail(`${field} must be true`);
  }
}

for (const tool of requiredTools) {
  if (!report.tools?.includes(tool)) {
    fail(`missing MCP tool ${tool}`);
  }
}

const step = report.steps?.find((item) => item.id === "repo-memory-mcp-read-only-smoke");
if (!step || step.ok !== true || step.status !== 0) {
  fail("repo-memory-mcp-read-only-smoke step did not pass");
}

console.log(`Repo Memory MCP summary OK (${requiredTools.length} read-only tools).`);
