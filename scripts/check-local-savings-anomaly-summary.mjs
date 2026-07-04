#!/usr/bin/env node
import fs from "node:fs";

const reportPath = "dist/local-savings-anomaly-validation-summary.json";
const requiredAlertKinds = ["output_growth", "low_savings", "cost_growth"];

function fail(message) {
  console.error(`local savings anomaly summary failed: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} missing; run npm run smoke:savings-anomalies:local first`);
}

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));

if (report.schemaVersion !== 1) fail("schemaVersion must be 1");
if (report.kind !== "mac_ai_switchboard.local_savings_anomaly_validation") {
  fail("kind must be mac_ai_switchboard.local_savings_anomaly_validation");
}
if (report.releaseGateEvidence !== false) {
  fail("releaseGateEvidence must remain false for local savings anomaly evidence");
}
if (report.readOnly !== true) fail("readOnly must be true");
if (report.passed !== true) fail("passed must be true");
if (report.command !== "npm run test:frontend -- src/lib/savingsCalculator.test.ts") {
  fail("command must run the savings calculator frontend tests");
}
for (const kind of requiredAlertKinds) {
  if (!report.requiredAlertKinds?.includes(kind)) {
    fail(`requiredAlertKinds missing ${kind}`);
  }
  if (report.sourceCoverage?.[kind] !== true) {
    fail(`sourceCoverage missing ${kind}`);
  }
}
for (const needle of [
  "rtk_output_growth",
  "headroom_engine_low_savings",
  "repo_intelligence_cost_growth",
  "Anomalies:",
]) {
  if (report.testCoverage?.[needle] !== true) {
    fail(`testCoverage missing ${needle}`);
  }
}
if (report.commandStatus !== 0) fail("commandStatus must be 0");
if (report.missingSourceNeedles?.length) fail("missingSourceNeedles must be empty");
if (report.missingTestNeedles?.length) fail("missingTestNeedles must be empty");

console.log("Local savings anomaly summary OK.");
