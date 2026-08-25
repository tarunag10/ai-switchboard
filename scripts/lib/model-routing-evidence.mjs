// Shared contract for ModelRoutingQualityEvidence-shaped benchmark evidence.
// Consumed by scripts/check-model-routing-evidence.mjs (the promotion gate)
// and scripts/lib/class-c-tasks.mjs (the Class C task runner) so promotion
// thresholds are defined exactly once.

export const MODEL_ROUTING_EVIDENCE_ARM_METRICS = [
  "sampleCount",
  "successfulTaskCount",
  "successRateBps",
  "qualityScoreBps",
  "p95LatencyMs",
  "successfulTaskCostMicros",
  "followUpReworkRateBps",
];

export const MODEL_ROUTING_THRESHOLDS = {
  maximumSuccessRegressionBps: 100,
  maximumQualityRegressionBps: 100,
  minimumCostImprovementBps: 1_000,
  maximumReworkRateBps: 500,
  maximumLatencyRegressionMs: 50,
};

export function evaluatePromotionEligibility(value) {
  const successRegressionBps = Math.max(0, value.baseline.successRateBps - value.candidate.successRateBps);
  const qualityRegressionBps = Math.max(0, value.baseline.qualityScoreBps - value.candidate.qualityScoreBps);
  const latencyRegressionMs = value.candidate.p95LatencyMs - value.baseline.p95LatencyMs;
  const baselineCost = BigInt(value.baseline.successfulTaskCostMicros);
  const candidateCost = BigInt(value.candidate.successfulTaskCostMicros);
  const costImprovementBps = baselineCost === 0n || candidateCost >= baselineCost
    ? 0
    : Number(((baselineCost - candidateCost) * 10_000n) / baselineCost);
  const enoughSamples = value.baseline.sampleCount >= value.minimumSamples;
  if (value.evidenceClass === "local_runtime_observation") {
    return {
      enoughSamples,
      successRegressionBps,
      qualityRegressionBps,
      costImprovementBps,
      latencyRegressionMs,
      reworkRateBps: value.candidate.followUpReworkRateBps,
      eligible: false,
    };
  }
  return {
    enoughSamples,
    successRegressionBps,
    qualityRegressionBps,
    costImprovementBps,
    latencyRegressionMs,
    reworkRateBps: value.candidate.followUpReworkRateBps,
    eligible: enoughSamples
      && successRegressionBps <= MODEL_ROUTING_THRESHOLDS.maximumSuccessRegressionBps
      && qualityRegressionBps <= MODEL_ROUTING_THRESHOLDS.maximumQualityRegressionBps
      && costImprovementBps >= MODEL_ROUTING_THRESHOLDS.minimumCostImprovementBps
      && latencyRegressionMs <= MODEL_ROUTING_THRESHOLDS.maximumLatencyRegressionMs
      && value.candidate.followUpReworkRateBps <= MODEL_ROUTING_THRESHOLDS.maximumReworkRateBps,
  };
}

export function thresholdChecks(value) {
  const eligibility = evaluatePromotionEligibility(value);
  return {
    enoughSamples: eligibility.enoughSamples,
    successWithinRegressionLimit: eligibility.successRegressionBps <= MODEL_ROUTING_THRESHOLDS.maximumSuccessRegressionBps,
    qualityWithinRegressionLimit: eligibility.qualityRegressionBps <= MODEL_ROUTING_THRESHOLDS.maximumQualityRegressionBps,
    costImprovementMeetsMinimum: eligibility.costImprovementBps >= MODEL_ROUTING_THRESHOLDS.minimumCostImprovementBps,
    latencyWithinRegressionLimit: eligibility.latencyRegressionMs <= MODEL_ROUTING_THRESHOLDS.maximumLatencyRegressionMs,
    reworkWithinLimit: eligibility.reworkRateBps <= MODEL_ROUTING_THRESHOLDS.maximumReworkRateBps,
  };
}
