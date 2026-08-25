#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { validateReleaseEvidenceTimestamp } from "./release-evidence-time.mjs";
import {
  MODEL_ROUTING_EVIDENCE_ARM_METRICS as metrics,
  MODEL_ROUTING_THRESHOLDS as thresholds,
  evaluatePromotionEligibility,
} from "./lib/model-routing-evidence.mjs";

const root = process.cwd();
const fixturePath = process.argv[2]
  ? path.resolve(root, process.argv[2])
  : path.join(root, "benchmarks/fixtures/model-routing-quality-evidence.json");
let fixture;
try {
  fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
} catch (error) {
  const reason = error instanceof SyntaxError ? "contains invalid JSON" : "could not be read";
  console.error(`model-routing evidence check failed: ${fixturePath} ${reason}`);
  process.exit(1);
}

function validateFixture(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("fixture must be a JSON object");
  }
  if (value.schemaVersion !== 1) throw new Error("unsupported schemaVersion");
  if (!["offline_static_fixture", "local_runtime_observation", "approved_live_run"].includes(value.evidenceClass)) {
    throw new Error("evidenceClass must be offline_static_fixture, local_runtime_observation, or approved_live_run");
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
    if (value[arm].successfulTaskCount > value[arm].sampleCount) {
      throw new Error(`${arm}.successfulTaskCount must not exceed sampleCount`);
    }
    if (value[arm].sampleCount > 0) {
      const expectedSuccessRateBps = Math.floor((value[arm].successfulTaskCount * 10_000) / value[arm].sampleCount);
      if (value[arm].successRateBps !== expectedSuccessRateBps) {
        throw new Error(`${arm}.successRateBps must match successfulTaskCount/sampleCount`);
      }
    }
  }
  if (value.baseline.sampleCount !== value.candidate.sampleCount) {
    throw new Error("baseline and candidate sampleCount must match");
  }
  const provenance = value.provenance;
  if (!provenance || typeof provenance !== "object") {
    throw new Error("provenance is required");
  }
  for (const field of ["taskClass", "baselineModel", "candidateModel", "source"]) {
    if (typeof provenance[field] !== "string" || provenance[field].trim() === "") {
      throw new Error(`provenance.${field} is required`);
    }
  }
  if (!["local_estimate", "provider_declared"].includes(provenance.costAttribution)) {
    throw new Error("provenance.costAttribution must be local_estimate or provider_declared");
  }
  if (provenance.costAttribution === "provider_declared") {
    if (typeof provenance.providerId !== "string" || provenance.providerId.trim() === "") {
      throw new Error("provider-declared cost attribution requires provenance.providerId");
    }
  } else if (provenance.providerId !== undefined) {
    throw new Error("local-estimate cost attribution must not include provenance.providerId");
  }
  if (provenance.baselineModel.trim() === provenance.candidateModel.trim()) {
    throw new Error("provenance baselineModel and candidateModel must differ");
  }
  if (value.evidenceClass === "offline_static_fixture" && provenance.source !== "offline_fixture") {
    throw new Error("offline evidence provenance.source must be offline_fixture");
  }
  if (value.evidenceClass === "offline_static_fixture" && provenance.costAttribution !== "local_estimate") {
    throw new Error("offline evidence cost attribution must remain local_estimate");
  }
  if (value.evidenceClass === "local_runtime_observation") {
    if (provenance.source !== "local_runtime_observation") {
      throw new Error("local evidence provenance.source must be local_runtime_observation");
    }
    if (provenance.costAttribution !== "local_estimate") {
      throw new Error("local runtime evidence cost attribution must remain local_estimate");
    }
    for (const field of ["runId", "capturedAt"]) {
      if (typeof value.provenance[field] !== "string" || value.provenance[field].trim() === "") {
        throw new Error(`local runtime evidence requires provenance.${field}`);
      }
    }
    const capturedAt = validateReleaseEvidenceTimestamp(value.provenance.capturedAt, {
      label: "capturedAt",
    });
    if (!capturedAt.ok) throw new Error(`local runtime evidence ${capturedAt.reason}`);
    if (value.promotionEligible !== false) {
      throw new Error("local runtime evidence must remain observe-only");
    }
  }
  if (value.evidenceClass === "offline_static_fixture" && value.promotionEligible !== false) {
    throw new Error("offline evidence must never be promotion eligible");
  }
  if (value.evidenceClass === "approved_live_run") {
    if (provenance.costAttribution !== "provider_declared") {
      throw new Error("approved live runs require provider_declared cost attribution");
    }
    if (value.baseline.sampleCount < value.minimumSamples) {
      throw new Error(`approved live runs require at least ${value.minimumSamples} samples per arm`);
    }
    for (const arm of ["baseline", "candidate"]) {
      if (value[arm].successfulTaskCount < 1) {
        throw new Error(`approved live runs require at least one successful task in ${arm}`);
      }
    }
    if (value.promotionEligible !== true) throw new Error("approved live run must state promotionEligible explicitly");
    if (provenance.source !== "approved_live_run") {
      throw new Error("approved live run provenance.source must be approved_live_run");
    }
    for (const field of ["runId", "capturedAt", "approvalReceipt"]) {
      if (typeof value[field] !== "string" || value[field].trim() === "") throw new Error(`approved live run requires ${field}`);
    }
    const capturedAt = validateReleaseEvidenceTimestamp(value.capturedAt, {
      label: "capturedAt",
    });
    if (!capturedAt.ok) {
      throw new Error(`approved live run ${capturedAt.reason}`);
    }
  }
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
  provenance: fixture.provenance,
}, null, 2));
