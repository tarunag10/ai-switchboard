#!/usr/bin/env node

import fs from "node:fs";

const reportPath = "dist/local-rollback-validation-summary.json";
const requiredStepIds = ["managed-changes-frontend", "native-undo-all", "gemini-cleanup", "dedicated-cleanup"];
const requiredDomains = ["managed-storage", "repo-intelligence", "login-item", "plugins-backups"];

function fail(message) {
  console.error(`rollback summary check failed: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} missing; run npm run smoke:rollback:local first`);
}

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));

if (report.kind !== "mac_ai_switchboard.local_rollback_validation") {
  fail("unexpected report kind");
}
if (report.releaseGateEvidence !== false) {
  fail("releaseGateEvidence must remain false for local rollback evidence");
}
if (report.passed !== true) {
  fail("report.passed must be true");
}
if (!String(report.relaunchSurvivalEvidence || "").includes("mac_ai_switchboard.rollback_relaunch_survival_probe")) {
  fail("missing relaunch survival probe evidence");
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

const domains = report.dedicatedCleanupDomains ?? [];
for (const domain of requiredDomains) {
  if (!domains.includes(domain)) {
    fail(`missing rollback evidence domain ${domain}`);
  }
}

console.log(`Rollback summary OK (${requiredStepIds.length} checks, ${requiredDomains.length} domains).`);
