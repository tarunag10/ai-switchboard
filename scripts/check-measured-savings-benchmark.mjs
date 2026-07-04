#!/usr/bin/env node
import fs from "node:fs";

const reportPath = "dist/measured-savings-benchmark.json";
const requiredSources = ["caveman", "markitdown", "ponytail"];

function fail(message) {
  console.error(`measured savings benchmark check failed: ${message}`);
  process.exitCode = 1;
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} is missing; run npm run savings:benchmark first`);
  process.exit();
}

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
if (report.schemaVersion !== 1) {
  fail("schemaVersion must be 1");
}
if (report.kind !== "mac_ai_switchboard.measured_savings_benchmark") {
  fail("kind must be mac_ai_switchboard.measured_savings_benchmark");
}
if (report.releaseGateEvidence !== false) {
  fail("releaseGateEvidence must be false for fixture savings evidence");
}
if (report.measurementClass !== "fixture_before_after") {
  fail("measurementClass must be fixture_before_after");
}
if (!String(report.caveat || "").includes("Fixture benchmark evidence")) {
  fail("caveat must identify fixture benchmark evidence");
}
if (!Array.isArray(report.rows)) {
  fail("rows must be an array");
}
if (Number(report.totals?.savedTokens ?? 0) <= 0) {
  fail("totals.savedTokens must be positive");
}
if (!Number.isFinite(Number(report.totals?.savingsPct)) || Number(report.totals.savingsPct) <= 0) {
  fail("totals.savingsPct must be positive");
}

for (const source of requiredSources) {
  const row = report.rows.find((item) => item.source === source);
  if (!row) {
    fail(`missing benchmark row for ${source}`);
    continue;
  }
  if (row.confidence !== "measured_fixture") {
    fail(`${source} confidence must be measured_fixture`);
  }
  if (Number(row.beforeTokens) <= Number(row.afterTokens)) {
    fail(`${source} beforeTokens must be greater than afterTokens`);
  }
  if (Number(row.savedTokens) <= 0) {
    fail(`${source} savedTokens must be positive`);
  }
  if (!Number.isFinite(Number(row.savingsPct)) || Number(row.savingsPct) <= 0) {
    fail(`${source} savingsPct must be positive`);
  }
}

if (process.exitCode) {
  process.exit();
}

console.log(
  `Measured savings benchmark OK (${requiredSources.join(", ")}; ${report.totals.savedTokens} tokens saved).`,
);
