#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const fixturePath = process.argv[2]
  ? path.resolve(root, process.argv[2])
  : path.join(root, "benchmarks/fixtures/model-routing-quality-evidence.json");
const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
const metrics = [
  "sampleCount",
  "successRateBps",
  "qualityScoreBps",
  "p95LatencyMs",
  "successfulTaskCostMicros",
  "followUpReworkRateBps",
];
const thresholds = {
  maximumSuccessRegressionBps: 100,
  maximumQualityRegressionBps: 100,
  minimumCostImprovementBps: 1_000,
  maximumReworkRateBps: 500,
  maximumLatencyRegressionMs: 50,
};

function validateFixture(value) {
  if (value.schemaVersion !== 1) throw new Error("unsupported schemaVersion");
  if (!["offline_static_fixture", "approved_live_run"].includes(value.evidenceClass)) {
    throw new Error("evidenceClass must be offline_static_fixture or approved_live_run");
  }
  if (!Number.isInteger(value.minimumSamples) || value.minimumSamples <= 0) {
    throw new Error("minimumSamples must be a positive integer");
  }
  for (const arm of ["baseline", "candidate"]) {
    if (!value[arm] || typeof value[arm] !== "object") throw new Error(`missing ${arm} arm`);
    for (const metric of metrics) {
      if (!Number.isSafeInteger(value[arm][metric]) || value[arm][metric] < 0) {
        throw new Error(`${arm}.${metric} must be a non-negative integer`);
      }
    }
    if (value[arm].successRateBps > 10_000 || value[arm].qualityScoreBps > 10_000 || value[arm].followUpReworkRateBps > 10_000) {
      throw new Error(`${arm} basis-point metrics must be at most 10000`);
    }
  }
  if (value.baseline.sampleCount !== value.candidate.sampleCount) {
    throw new Error("baseline and candidate sampleCount must match");
  }
  if (value.evidenceClass !== "approved_live_run" && value.promotionEligible !== false) {
    throw new Error("offline evidence must never be promotion eligible");
  }
  if (value.evidenceClass === "approved_live_run") {
    if (value.baseline.sampleCount < value.minimumSamples) {
      throw new Error(`approved live runs require at least ${value.minimumSamples} samples per arm`);
    }
    if (value.promotionEligible !== true) throw new Error("approved live run must state promotionEligible explicitly");
    for (const field of ["runId", "capturedAt", "approvalReceipt"]) {
      if (typeof value[field] !== "string" || value[field].trim() === "") throw new Error(`approved live run requires ${field}`);
    }
    const capturedAt = Date.parse(value.capturedAt);
    if (Number.isNaN(capturedAt) || capturedAt > Date.now()) {
      throw new Error("approved live run capturedAt must be a valid non-future timestamp");
    }
  }
}

function evaluatePromotionEligibility(value) {
  const successRegressionBps = Math.max(0, value.baseline.successRateBps - value.candidate.successRateBps);
  const qualityRegressionBps = Math.max(0, value.baseline.qualityScoreBps - value.candidate.qualityScoreBps);
  const latencyRegressionMs = value.candidate.p95LatencyMs - value.baseline.p95LatencyMs;
  const baselineCost = BigInt(value.baseline.successfulTaskCostMicros);
  const candidateCost = BigInt(value.candidate.successfulTaskCostMicros);
  const costImprovementBps = baselineCost === 0n
    ? 0
    : Number(((baselineCost - candidateCost) * 10_000n) / baselineCost);
  const enoughSamples = value.baseline.sampleCount >= value.minimumSamples;
  return {
    enoughSamples,
    successRegressionBps,
    qualityRegressionBps,
    costImprovementBps,
    latencyRegressionMs,
    reworkRateBps: value.candidate.followUpReworkRateBps,
    eligible: enoughSamples
      && successRegressionBps <= thresholds.maximumSuccessRegressionBps
      && qualityRegressionBps <= thresholds.maximumQualityRegressionBps
      && costImprovementBps >= thresholds.minimumCostImprovementBps
      && latencyRegressionMs <= thresholds.maximumLatencyRegressionMs
      && value.candidate.followUpReworkRateBps <= thresholds.maximumReworkRateBps,
  };
}

try {
  validateFixture(fixture);
} catch (error) {
  console.error(`model routing evidence check failed: ${error.message}`);
  process.exit(1);
}

const eligibility = evaluatePromotionEligibility(fixture);
if (fixture.promotionEligible !== eligibility.eligible) {
  console.error(`model routing evidence check failed: promotionEligible does not match recomputed threshold result (${eligibility.eligible})`);
  process.exit(1);
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
  eligibility,
  automaticRouting: fixture.promotionEligible ? "eligible_for_threshold_evaluation" : "observe_only",
}, null, 2));
