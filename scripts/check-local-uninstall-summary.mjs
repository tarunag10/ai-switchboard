#!/usr/bin/env node

import fs from "node:fs";

const reportPath = "dist/local-uninstall-validation-summary.json";
const requiredStepIds = ["uninstall-disclosure-frontend", "uninstall-dry-run-backend"];

function fail(message) {
  console.error(`uninstall summary check failed: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} missing; run npm run smoke:uninstall:local first`);
}

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));

if (report.schemaVersion !== 1) {
  fail("schemaVersion must be 1");
}
if (report.kind !== "mac_ai_switchboard.local_uninstall_validation") {
  fail("unexpected report kind");
}
if (report.releaseGateEvidence !== false) {
  fail("releaseGateEvidence must remain false for local uninstall evidence");
}
if (report.destructive !== false) {
  fail("destructive must remain false for local uninstall evidence");
}
if (report.passed !== true) {
  fail("report.passed must be true");
}
if (!Array.isArray(report.steps)) {
  fail("steps must be an array");
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

console.log(`Uninstall summary OK (${requiredStepIds.length} checks).`);
