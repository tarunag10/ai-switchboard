#!/usr/bin/env node

import fs from "node:fs";

const reportPath = "dist/local-connector-readiness-summary.json";
const requiredConnectors = ["aider", "amazon_q", "cursor", "grok_cli"];
const requiredStages = ["detect", "dryRunDiff", "backup", "apply", "verify", "rollback", "offCleanup"];

function fail(message) {
  console.error(`connector readiness summary check failed: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} missing; run npm run smoke:connectors:local first`);
}

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));

if (report.schemaVersion !== 1) {
  fail("schemaVersion must be 1");
}
if (report.kind !== "mac_ai_switchboard.local_connector_readiness_validation") {
  fail("kind must be mac_ai_switchboard.local_connector_readiness_validation");
}
if (report.releaseGateEvidence !== false) {
  fail("releaseGateEvidence must remain false for local connector readiness evidence");
}
if (report.readOnly !== false) {
  fail("readOnly must remain false because this evidence describes native-write readiness");
}
if (report.status !== 0) {
  fail("check:connectors status must be 0");
}
if (report.ready !== true) {
  fail("report.ready must be true");
}
for (const stage of requiredStages) {
  if (!report.requiredLifecycleStages?.includes(stage)) {
    fail(`requiredLifecycleStages missing ${stage}`);
  }
  if (!report.sharedStages?.includes(stage)) {
    fail(`sharedStages missing ${stage}`);
  }
}

for (const connectorId of requiredConnectors) {
  if (!report.requiredGatedNativeWrite?.includes(connectorId)) {
    fail(`requiredGatedNativeWrite missing ${connectorId}`);
  }
  if (!report.gatedNativeWriteConnectors?.includes(connectorId)) {
    fail(`gatedNativeWriteConnectors missing ${connectorId}`);
  }
  const coverage = report.lifecycleCoverage?.[connectorId];
  if (!coverage) {
    fail(`lifecycleCoverage missing ${connectorId}`);
  }
  for (const stage of requiredStages) {
    if (!coverage.stages?.includes(stage)) {
      fail(`${connectorId} lifecycle coverage missing ${stage}`);
    }
  }
  if (coverage.complete !== true) {
    fail(`${connectorId} lifecycle coverage is incomplete`);
  }
  if (coverage.automationDisabled !== true) {
    fail(`${connectorId} automation must remain disabled until native writes are promoted`);
  }
}

if (report.requiredGatedNativeWritePresent !== true) {
  fail("requiredGatedNativeWritePresent must be true");
}
if (report.lifecycleCoverageComplete !== true) {
  fail("lifecycleCoverageComplete must be true");
}

console.log(`Connector readiness summary OK (${requiredConnectors.length} gated connectors).`);
