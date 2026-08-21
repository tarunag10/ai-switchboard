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

test("rejects unequal arms and future live-run timestamps", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-routing-evidence-"));
  try {
    const fixture = JSON.parse(fs.readFileSync("benchmarks/fixtures/model-routing-quality-evidence.json", "utf8"));
    fixture.evidenceClass = "approved_live_run";
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
