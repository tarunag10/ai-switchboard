#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const distDir = "dist";
const jsonPath = path.join(distDir, "local-savings-anomaly-validation-summary.json");
const markdownPath = path.join(distDir, "local-savings-anomaly-validation-summary.md");
const sourcePath = "src/lib/savingsCalculator.ts";
const testPath = "src/lib/savingsCalculator.test.ts";

function sourceIncludes(filePath, needles) {
  const body = fs.readFileSync(filePath, "utf8");
  return Object.fromEntries(needles.map((needle) => [needle, body.includes(needle)]));
}

const requiredAlertKinds = ["output_growth", "low_savings", "cost_growth"];
const sourceNeedles = [
  "buildSavingsAnomalyAlerts",
  "formatSavingsAnomalyAlerts",
  ...requiredAlertKinds,
];
const testNeedles = [
  "rtk_output_growth",
  "headroom_engine_low_savings",
  "repo_intelligence_cost_growth",
  "Anomalies:",
];

const sourceCoverage = sourceIncludes(sourcePath, sourceNeedles);
const testCoverage = sourceIncludes(testPath, testNeedles);
const testResult = spawnSync(
  "npm",
  ["run", "test:frontend", "--", testPath],
  {
    encoding: "utf8",
    timeout: 180_000,
    maxBuffer: 16 * 1024 * 1024,
  },
);

const missingSourceNeedles = Object.entries(sourceCoverage)
  .filter(([, present]) => !present)
  .map(([needle]) => needle);
const missingTestNeedles = Object.entries(testCoverage)
  .filter(([, present]) => !present)
  .map(([needle]) => needle);

const passed =
  testResult.status === 0 &&
  missingSourceNeedles.length === 0 &&
  missingTestNeedles.length === 0;

const payload = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  kind: "mac_ai_switchboard.local_savings_anomaly_validation",
  releaseGateEvidence: false,
  readOnly: true,
  passed,
  requiredAlertKinds,
  sourcePath,
  testPath,
  sourceCoverage,
  testCoverage,
  missingSourceNeedles,
  missingTestNeedles,
  command: `npm run test:frontend -- ${testPath}`,
  commandStatus: testResult.status,
  stdoutPreview: (testResult.stdout ?? "").slice(0, 4000),
  stderrPreview: (testResult.stderr ?? "").slice(0, 4000),
};

fs.mkdirSync(distDir, { recursive: true });
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);
fs.writeFileSync(
  markdownPath,
  [
    "# Local Savings Anomaly Validation",
    "",
    `Generated: ${payload.generatedAt}`,
    `Passed: ${passed ? "yes" : "no"}`,
    `Release gate evidence: ${payload.releaseGateEvidence ? "yes" : "no"}`,
    `Read-only: ${payload.readOnly ? "yes" : "no"}`,
    `Alert kinds: ${requiredAlertKinds.join(", ")}`,
    `Command: ${payload.command}`,
    `Missing source needles: ${missingSourceNeedles.join(", ") || "none"}`,
    `Missing test needles: ${missingTestNeedles.join(", ") || "none"}`,
    "",
  ].join("\n"),
);

console.log("Local savings anomaly validation summary written.");
console.log(`Summary written: ${markdownPath}`);
console.log(`JSON written: ${jsonPath}`);
process.exitCode = passed ? 0 : 1;
