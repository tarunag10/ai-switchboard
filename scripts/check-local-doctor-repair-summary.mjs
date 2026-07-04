#!/usr/bin/env node

import fs from "node:fs";

const reportPath = "dist/local-doctor-repair-validation-summary.json";
const requiredStepIds = ["doctor-frontend", "off-mode-guard", "post-write-verification-guard"];

function fail(message) {
  console.error(`doctor repair summary check failed: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} missing; run npm run smoke:doctor-repair:local first`);
}

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));

if (report.schemaVersion !== 1) {
  fail("schemaVersion must be 1");
}
if (report.kind !== "mac_ai_switchboard.local_doctor_repair_validation") {
  fail("unexpected report kind");
}
if (report.releaseGateEvidence !== false) {
  fail("releaseGateEvidence must remain false for local Doctor repair evidence");
}
if (report.passed !== true) {
  fail("report.passed must be true");
}
if (!Array.isArray(report.steps)) {
  fail("steps must be an array");
}

for (const stepId of requiredStepIds) {
  const result = report.steps.find((item) => item.id === stepId);
  if (!result) {
    fail(`missing result ${stepId}`);
  }
  if (result.ok !== true || result.status !== 0) {
    fail(`${stepId} did not pass`);
  }
  if (!String(result.command || "").trim()) {
    fail(`${stepId} missing command evidence`);
  }
}

console.log(`Doctor repair summary OK (${requiredStepIds.length} checks).`);
