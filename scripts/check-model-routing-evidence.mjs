#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const fixturePath = path.join(root, "benchmarks/fixtures/model-routing-quality-evidence.json");
const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
const metrics = [
  "sampleCount",
  "successRateBps",
  "qualityScoreBps",
  "p95LatencyMs",
  "successfulTaskCostMicros",
  "followUpReworkRateBps",
];

function fail(message) {
  console.error(`model routing evidence check failed: ${message}`);
  process.exit(1);
}

if (fixture.schemaVersion !== 1) fail("unsupported schemaVersion");
if (!["offline_static_fixture", "approved_live_run"].includes(fixture.evidenceClass)) {
  fail("evidenceClass must be offline_static_fixture or approved_live_run");
}
for (const arm of ["baseline", "candidate"]) {
  if (!fixture[arm] || typeof fixture[arm] !== "object") fail(`missing ${arm} arm`);
  for (const metric of metrics) {
    if (!Number.isInteger(fixture[arm][metric]) || fixture[arm][metric] < 0) {
      fail(`${arm}.${metric} must be a non-negative integer`);
    }
  }
  if (fixture[arm].successRateBps > 10_000 || fixture[arm].qualityScoreBps > 10_000 || fixture[arm].followUpReworkRateBps > 10_000) {
    fail(`${arm} basis-point metrics must be at most 10000`);
  }
}

if (fixture.evidenceClass !== "approved_live_run" && fixture.promotionEligible !== false) {
  fail("offline evidence must never be promotion eligible");
}
if (fixture.evidenceClass === "approved_live_run") {
  if (fixture.baseline.sampleCount < fixture.minimumSamples || fixture.candidate.sampleCount < fixture.minimumSamples) {
    fail(`approved live runs require at least ${fixture.minimumSamples} samples per arm`);
  }
  if (fixture.promotionEligible !== true) fail("approved live run must state promotionEligible explicitly");
  for (const field of ["runId", "capturedAt", "approvalReceipt"]) {
    if (typeof fixture[field] !== "string" || fixture[field].trim() === "") fail(`approved live run requires ${field}`);
  }
}

console.log(JSON.stringify({
  ok: true,
  evidenceClass: fixture.evidenceClass,
  promotionEligible: fixture.promotionEligible,
  minimumSamples: fixture.minimumSamples,
  sampleCounts: {
    baseline: fixture.baseline.sampleCount,
    candidate: fixture.candidate.sampleCount,
  },
  automaticRouting: fixture.promotionEligible ? "eligible_for_threshold_evaluation" : "observe_only",
}, null, 2));
