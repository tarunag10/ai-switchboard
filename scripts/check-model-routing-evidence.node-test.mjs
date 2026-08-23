import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

test("offline model-routing evidence remains observe-only", () => {
  const output = execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs"], { encoding: "utf8" });
  const result = JSON.parse(output);
  assert.equal(result.ok, true);
  assert.equal(result.evidenceClass, "offline_static_fixture");
  assert.equal(result.promotionEligible, false);
  assert.equal(result.automaticRouting, "observe_only");
});

test("local runtime model-routing evidence remains observe-only", () => {
  const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
  fixture.evidenceClass = "local_runtime_observation";
  fixture.provenance = {
    ...fixture.provenance,
    source: "local_runtime_observation",
    runId: "local-run-1",
    capturedAt: new Date().toISOString(),
  };
  fixture.promotionEligible = false;
  const fixturePath = path.join(os.tmpdir(), `model-routing-local-${process.pid}.json`);
  fs.writeFileSync(fixturePath, JSON.stringify(fixture));
  const result = JSON.parse(execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
  assert.equal(result.ok, true);
  assert.equal(result.automaticRouting, "observe_only");
  fs.unlinkSync(fixturePath);
});

test("rejects missing or non-distinct evidence provenance", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-provenance-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    const fixturePath = path.join(tempDir, "invalid.json");
    delete fixture.provenance;
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    assert.throws(() => execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));

    fixture.provenance = {
      taskClass: "formatting",
      baselineModel: "same-model",
      candidateModel: "same-model",
      source: "offline_fixture",
    };
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    assert.throws(() => execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("rejects corrupt evidence JSON with a concise diagnostic", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-corrupt-"));
  try {
    const fixturePath = path.join(tempDir, "corrupt.json");
    fs.writeFileSync(fixturePath, "{\"schemaVersion\":1,");
    try {
      execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      });
      assert.fail("expected corrupt fixture to fail");
    } catch (error) {
      assert.equal(error.status, 1);
      assert.match(error.stderr, /model-routing evidence check failed: .* contains invalid JSON/);
      assert.doesNotMatch(error.stderr, /SyntaxError|at JSON\.parse/);
    }
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("rejects a non-object evidence root without a raw type error", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-root-"));
  try {
    const fixturePath = path.join(tempDir, "root.json");
    fs.writeFileSync(fixturePath, "null");
    try {
      execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      });
      assert.fail("expected non-object fixture to fail");
    } catch (error) {
      assert.equal(error.status, 1);
      assert.match(error.stderr, /model routing evidence check failed: fixture must be a JSON object/);
      assert.doesNotMatch(error.stderr, /TypeError|at /);
    }
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("requires explicit provider identity for provider-declared costs", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-cost-provenance-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    const fixturePath = path.join(tempDir, "invalid-cost-provenance.json");
    fixture.provenance.costAttribution = "provider_declared";
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    assert.throws(() => execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
    fixture.provenance.costAttribution = "local_estimate";
    fixture.provenance.providerId = "unexpected-provider";
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    assert.throws(() => execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("rejects unequal arms and future live-run timestamps", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-evidence-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    fixture.evidenceClass = "approved_live_run";
    fixture.provenance.source = "approved_live_run";
    fixture.provenance.costAttribution = "provider_declared";
    fixture.provenance.providerId = "test-provider";
    fixture.promotionEligible = true;
    fixture.minimumSamples = 1;
    fixture.baseline.sampleCount = 2;
    fixture.candidate.sampleCount = 1;
    fixture.runId = "test-run";
    fixture.capturedAt = "2999-01-01T00:00:00Z";
    fixture.approvalReceipt = "test-receipt";
    const fixturePath = path.join(tempDir, "invalid.json");
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    assert.throws(() => execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("rejects stale or timezone-free live-run timestamps", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-evidence-time-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    fixture.evidenceClass = "approved_live_run";
    fixture.provenance.source = "approved_live_run";
    fixture.provenance.costAttribution = "provider_declared";
    fixture.provenance.providerId = "test-provider";
    fixture.promotionEligible = false;
    fixture.minimumSamples = 1;
    fixture.baseline.sampleCount = 1;
    fixture.candidate.sampleCount = 1;
    fixture.runId = "test-run";
    fixture.approvalReceipt = "test-receipt";
    const fixturePath = path.join(tempDir, "invalid-time.json");
    for (const capturedAt of ["2026-08-01T00:00:00.000Z", "2026-08-21T00:00:00.000"]) {
      fixture.capturedAt = capturedAt;
      fs.writeFileSync(fixturePath, JSON.stringify(fixture));
      assert.throws(() => execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
    }
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("recomputes a passing live run instead of trusting promotionEligible", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-evidence-pass-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    fixture.evidenceClass = "approved_live_run";
    fixture.provenance.source = "approved_live_run";
    fixture.provenance.costAttribution = "provider_declared";
    fixture.provenance.providerId = "test-provider";
    fixture.promotionEligible = true;
    fixture.minimumSamples = 100;
    fixture.baseline.sampleCount = 100;
    fixture.candidate.sampleCount = 100;
    fixture.baseline.successfulTaskCount = 98;
    fixture.candidate.successfulTaskCount = 98;
    fixture.baseline.successRateBps = 9_800;
    fixture.candidate.successRateBps = 9_800;
    fixture.baseline.qualityScoreBps = 9_800;
    fixture.candidate.qualityScoreBps = 9_800;
    fixture.baseline.p95LatencyMs = 800;
    fixture.candidate.p95LatencyMs = 820;
    fixture.baseline.successfulTaskCostMicros = 1_000;
    fixture.candidate.successfulTaskCostMicros = 700;
    fixture.candidate.followUpReworkRateBps = 300;
    fixture.runId = "test-run";
    fixture.capturedAt = "2026-08-20T00:00:00Z";
    fixture.approvalReceipt = "test-receipt";
    const fixturePath = path.join(tempDir, "passing.json");
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    const result = JSON.parse(execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
    assert.equal(result.eligibility.eligible, true);
    assert.equal(result.eligibility.costImprovementBps, 3_000);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("keeps large cost-improvement arithmetic bounded and exact enough for the gate", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-evidence-cost-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    fixture.evidenceClass = "approved_live_run";
    fixture.provenance.source = "approved_live_run";
    fixture.provenance.costAttribution = "provider_declared";
    fixture.provenance.providerId = "test-provider";
    fixture.promotionEligible = true;
    fixture.minimumSamples = 1;
    fixture.baseline.sampleCount = 100;
    fixture.candidate.sampleCount = 100;
    fixture.baseline.successfulTaskCount = 99;
    fixture.candidate.successfulTaskCount = 99;
    fixture.baseline.successRateBps = fixture.candidate.successRateBps = 9_900;
    fixture.baseline.qualityScoreBps = fixture.candidate.qualityScoreBps = 9_900;
    fixture.baseline.p95LatencyMs = fixture.candidate.p95LatencyMs = 800;
    fixture.baseline.successfulTaskCostMicros = Number.MAX_SAFE_INTEGER;
    fixture.candidate.successfulTaskCostMicros = 1;
    fixture.candidate.followUpReworkRateBps = 100;
    fixture.runId = "test-large-cost";
    fixture.capturedAt = "2026-08-20T00:00:00Z";
    fixture.approvalReceipt = "test-receipt";
    const fixturePath = path.join(tempDir, "large-cost.json");
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    const result = JSON.parse(execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
    assert.equal(result.eligibility.eligible, true);
    assert.ok(Number.isSafeInteger(result.eligibility.costImprovementBps));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("rejects a live fixture that claims eligibility while failing thresholds", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-evidence-fail-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    fixture.evidenceClass = "approved_live_run";
    fixture.provenance.source = "approved_live_run";
    fixture.provenance.costAttribution = "provider_declared";
    fixture.provenance.providerId = "test-provider";
    fixture.promotionEligible = true;
    fixture.minimumSamples = 100;
    fixture.baseline.sampleCount = 100;
    fixture.candidate.sampleCount = 100;
    fixture.baseline.successfulTaskCostMicros = 1_000;
    fixture.candidate.successfulTaskCostMicros = 1_000;
    fixture.runId = "test-run";
    fixture.capturedAt = "2026-08-20T00:00:00Z";
    fixture.approvalReceipt = "test-receipt";
    const fixturePath = path.join(tempDir, "failing.json");
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    assert.throws(() => execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("rejects approved live evidence with zero successful tasks", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-zero-success-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    fixture.evidenceClass = "approved_live_run";
    fixture.provenance.source = "approved_live_run";
    fixture.provenance.costAttribution = "provider_declared";
    fixture.provenance.providerId = "test-provider";
    fixture.promotionEligible = false;
    fixture.minimumSamples = 100;
    fixture.runId = "zero-success-run";
    fixture.capturedAt = "2026-08-20T00:00:00Z";
    fixture.approvalReceipt = "test-receipt";
    for (const arm of ["baseline", "candidate"]) {
      fixture[arm].sampleCount = 100;
      fixture[arm].successfulTaskCount = 0;
      fixture[arm].successRateBps = 0;
      fixture[arm].successfulTaskCostMicros = arm === "baseline" ? 1_000 : 0;
    }
    const fixturePath = path.join(tempDir, "zero-success.json");
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    assert.throws(() => execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("rejects success rates inconsistent with successful task counts", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-success-rate-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    fixture.evidenceClass = "approved_live_run";
    fixture.provenance.source = "approved_live_run";
    fixture.provenance.costAttribution = "provider_declared";
    fixture.provenance.providerId = "test-provider";
    fixture.promotionEligible = false;
    fixture.minimumSamples = 1;
    fixture.runId = "inconsistent-rate-run";
    fixture.capturedAt = "2026-08-20T00:00:00Z";
    fixture.approvalReceipt = "test-receipt";
    for (const arm of ["baseline", "candidate"]) {
      fixture[arm].sampleCount = 2;
      fixture[arm].successfulTaskCount = 1;
      fixture[arm].successRateBps = 2_500;
    }
    const fixturePath = path.join(tempDir, "inconsistent-rate.json");
    fs.writeFileSync(fixturePath, JSON.stringify(fixture));
    assert.throws(() => execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs", fixturePath], { encoding: "utf8" }));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});
