#!/usr/bin/env node

import fs from "node:fs";

const reportPath = "dist/local-repo-intelligence-validation-summary.json";
const requiredStepIds = ["repo-intelligence-frontend", "repo-intelligence-backend"];
const requiredIndexerVersion = "path-graph-v10";

function fail(message) {
  console.error(`repo intelligence summary check failed: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} missing; run npm run smoke:repo-intelligence:local first`);
}

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
const backendSource = fs.readFileSync("src-tauri/src/repo_intelligence.rs", "utf8");
const frontendSource = fs.readFileSync("src/lib/repoIntelligence.ts", "utf8");
const cliSource = fs.readFileSync("scripts/repo-intelligence.mjs", "utf8");

if (report.schemaVersion !== 1) {
  fail("schemaVersion must be 1");
}
if (report.kind !== "mac_ai_switchboard.local_repo_intelligence_validation") {
  fail("unexpected report kind");
}
if (report.releaseGateEvidence !== false) {
  fail("releaseGateEvidence must remain false for local Repo Intelligence evidence");
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
if (!Array.isArray(report.steps)) {
  fail("steps must be an array");
}
for (const [label, source] of [
  ["backend", backendSource],
  ["frontend", frontendSource],
  ["CLI", cliSource],
]) {
  if (!source.includes(requiredIndexerVersion)) {
    fail(`${label} indexer version must include ${requiredIndexerVersion}`);
  }
}

for (const stepId of requiredStepIds) {
  const step = report.steps.find((item) => item.id === stepId);
  if (!step) {
    fail(`missing result ${stepId}`);
  }
  if (step.ok !== true || step.status !== 0) {
    fail(`${stepId} did not pass`);
  }
  if (!String(step.fullCommand || "").trim()) {
    fail(`${stepId} missing command evidence`);
  }
}

console.log(`Repo Intelligence summary OK (${requiredStepIds.length} checks).`);
