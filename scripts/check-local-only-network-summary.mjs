#!/usr/bin/env node

import fs from "node:fs";

const reportPath = "dist/local-only-network-validation-summary.json";
const requiredStepIds = [
  "local-only-network-certification",
  "local-build-privacy-scan",
];

function fail(message) {
  console.error(`local-only network summary check failed: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} missing; run npm run smoke:local-only:local first`);
}

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));

if (report.schemaVersion !== 1) {
  fail("schemaVersion must be 1");
}
if (report.kind !== "mac_ai_switchboard.local_only_network_validation") {
  fail("unexpected report kind");
}
if (report.releaseGateEvidence !== false) {
  fail("releaseGateEvidence must remain false for local-only network evidence");
}
if (report.localOnly !== true) {
  fail("localOnly must be true");
}
if (report.appOwnedRemoteCallsBlocked !== true) {
  fail("appOwnedRemoteCallsBlocked must be true");
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

console.log(`Local-only network summary OK (${requiredStepIds.length} checks).`);
