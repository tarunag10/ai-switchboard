import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { buildSummary } from "./lib/class-c-tasks.mjs";

const root = process.cwd();

function makeTempRoot() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-class-c-"));
  const tasksDir = path.join(tempDir, "benchmarks/tasks");
  fs.mkdirSync(tasksDir, { recursive: true });
  fs.copyFileSync(
    path.join(root, "benchmarks/tasks/schema.json"),
    path.join(tasksDir, "schema.json"),
  );
  return tempDir;
}

function writeTask(tempDir, name, value) {
  fs.writeFileSync(path.join(tempDir, "benchmarks/tasks", name), JSON.stringify(value));
}

function writeResults(tempDir, name, documents) {
  const resultsDir = path.join(tempDir, "benchmarks/tasks/results");
  fs.mkdirSync(resultsDir, { recursive: true });
  fs.writeFileSync(path.join(resultsDir, name), JSON.stringify(documents));
}

function sampleRun(overrides = {}) {
  return {
    result_id: overrides.result_id ?? "run-0001",
    task_id: overrides.task_id ?? "formatting-trim",
    arm: overrides.arm ?? "baseline",
    test_result: overrides.test_result ?? "pass",
    input_tokens: overrides.input_tokens ?? 120,
    output_tokens: overrides.output_tokens ?? 40,
    files_changed: overrides.files_changed ?? ["src/example/greeting.ts"],
    lines_changed: overrides.lines_changed ?? 2,
    tool_calls: overrides.tool_calls ?? 3,
    retries: overrides.retries ?? 0,
    elapsed_ms: overrides.elapsed_ms ?? 900,
    selected_model: overrides.selected_model ?? "frontier",
    selected_endpoint: overrides.selected_endpoint ?? "local-gateway",
    optimization_profile: overrides.optimization_profile ?? "observe-only",
    provider_cache_read_tokens: overrides.provider_cache_read_tokens ?? 0,
    captured_at: overrides.captured_at,
  };
}

const formattingTask = {
  task_id: "formatting-trim",
  task_class: "formatting",
  repo_fixture: {
    kind: "inline_synthetic",
    synthetic: true,
    files: [{ path: "src/example/greeting.ts", content: "export const x = 1;   \n" }],
  },
  task: "Trim trailing whitespace.",
  success_command: "grep -rqE ' +$' src/example/greeting.ts && exit 1 || exit 0",
  allowed_files: ["src/example/greeting.ts"],
  quality_assertions: ["only the fixture file changes"],
  expected_risk: "low",
};

test("seeded repository definitions validate and include the formatting class", () => {
  const summary = buildSummary(root);
  assert.equal(summary.taskCount, 4);
  assert.ok(summary.taskIds.includes("formatting-trim-trailing-whitespace"));
  assert.ok(summary.perClass.formatting);
  assert.equal(summary.runResults.claimsLiveProviderRuns, false);
});

test("runner output is byte-identical across reruns", () => {
  const first = execFileSync(process.execPath, ["scripts/run-class-c-tasks.mjs", "--no-write"], { encoding: "utf8" });
  const second = execFileSync(process.execPath, ["scripts/run-class-c-tasks.mjs", "--no-write"], { encoding: "utf8" });
  assert.equal(first, second);
});

test("evidence stays omitted below minimum samples and stays observe-only above them", () => {
  const tempDir = makeTempRoot();
  try {
    writeTask(tempDir, "task.json", formattingTask);
    writeResults(tempDir, "runs.json", [
      ...Array.from({ length: 2 }, (_, index) => sampleRun({ result_id: `b-${index}`, arm: "baseline", elapsed_ms: 800 + index })),
      ...Array.from({ length: 2 }, (_, index) => sampleRun({ result_id: `c-${index}`, arm: "candidate", selected_model: "fast-local", elapsed_ms: 700 + index })),
    ]);
    const below = buildSummary(tempDir);
    assert.equal(below.perClass.formatting.evidence, null);
    assert.match(below.perClass.formatting.evidenceOmittedReason, /minimum of 100 samples/);

    const above = buildSummary(tempDir, { minimumSamples: 1 });
    const evidence = above.perClass.formatting.evidence?.evidence;
    assert.ok(evidence, "expected an emitted evidence block");
    assert.equal(evidence.evidenceClass, "local_runtime_observation");
    assert.equal(evidence.promotionEligible, false);
    assert.equal(evidence.provenance.baselineModel, "frontier");
    assert.equal(evidence.provenance.candidateModel, "fast-local");
    assert.equal(evidence.provenance.capturedAt, "operator-timestamps-not-provided");

    const withTimestamps = buildSummary(tempDir, { minimumSamples: 1 });
    assert.equal(withTimestamps.perClass.formatting.evidence.evidence.provenance.capturedAt, "operator-timestamps-not-provided");

    fs.writeFileSync(
      path.join(tempDir, "benchmarks/tasks/results/runs.json"),
      JSON.stringify([
        ...Array.from({ length: 2 }, (_, index) => sampleRun({ result_id: `b-${index}`, arm: "baseline", captured_at: "2026-08-20T00:00:00Z" })),
        ...Array.from({ length: 2 }, (_, index) => sampleRun({ result_id: `c-${index}`, arm: "candidate", selected_model: "fast-local", captured_at: "2026-08-21T00:00:00Z" })),
      ]),
    );
    const stamped = buildSummary(tempDir, { minimumSamples: 1 });
    assert.equal(stamped.perClass.formatting.evidence.evidence.provenance.capturedAt, "2026-08-21T00:00:00Z");
    assert.deepEqual(stamped.runResults.capturedAtValues, ["2026-08-20T00:00:00Z", "2026-08-21T00:00:00Z"]);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("computed arm metrics match the routing-evidence formulas", () => {
  const tempDir = makeTempRoot();
  try {
    writeTask(tempDir, "task.json", formattingTask);
    writeResults(tempDir, "runs.json", [
      sampleRun({ result_id: "b-0", arm: "baseline", test_result: "fail" }),
      sampleRun({ result_id: "b-1", arm: "baseline", elapsed_ms: 1000 }),
      sampleRun({ result_id: "b-2", arm: "baseline", retries: 1 }),
      ...Array.from({ length: 3 }, (_, index) => sampleRun({
        result_id: `c-${index}`,
        arm: "candidate",
        selected_model: "fast-local",
        files_changed: ["test/escapee.ts"],
      })),
    ]);
    const summary = buildSummary(tempDir);
    const baseline = summary.perClass.formatting.baseline;
    const candidate = summary.perClass.formatting.candidate;
    assert.equal(baseline.sampleCount, 3);
    assert.equal(baseline.successfulTaskCount, 2);
    assert.equal(baseline.successRateBps, 6666);
    assert.equal(baseline.qualityScoreBps, 10000);
    assert.equal(baseline.followUpReworkRateBps, 5000);
    assert.equal(baseline.p95LatencyMs, 1000);
    assert.equal(candidate.sampleCount, 3);
    assert.equal(candidate.successfulTaskCount, 3);
    assert.equal(candidate.successRateBps, 10000);
    assert.equal(candidate.qualityScoreBps, 0);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("rejects unknown task ids, duplicate result ids, and schema violations", () => {
  const tempDir = makeTempRoot();
  try {
    writeTask(tempDir, "task.json", formattingTask);
    writeResults(tempDir, "runs.json", [sampleRun({ task_id: "does-not-exist" })]);
    assert.throws(() => buildSummary(tempDir), /references unknown task_id 'does-not-exist'/);

    writeResults(tempDir, "runs.json", [sampleRun(), sampleRun()]);
    assert.throws(() => buildSummary(tempDir), /duplicate result_id/);

    const rogue = sampleRun();
    rogue.not_in_schema = true;
    writeResults(tempDir, "runs.json", [rogue]);
    assert.throws(() => buildSummary(tempDir), /unexpected property 'not_in_schema'/);

    const badTask = { ...formattingTask };
    delete badTask.success_command;
    writeTask(tempDir, "task.json", badTask);
    fs.rmSync(path.join(tempDir, "benchmarks/tasks/results"), { recursive: true, force: true });
    assert.throws(() => buildSummary(tempDir), /fails benchmarks\/tasks\/schema\.json.*success_command/);

    const badFixture = { ...formattingTask, repo_fixture: { kind: "repo_path", synthetic: true, path: "../escape.json" } };
    writeTask(tempDir, "task.json", badFixture);
    assert.throws(() => buildSummary(tempDir), /repo_path fixtures must set repo_fixture.path and synthetic=false/);

    badFixture.repo_fixture.synthetic = false;
    writeTask(tempDir, "task.json", badFixture);
    assert.throws(() => buildSummary(tempDir), /does not exist in the repository/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("emitted evidence blocks align field names with the canonical routing-evidence fixture", () => {
  const tempDir = makeTempRoot();
  try {
    writeTask(tempDir, "task.json", formattingTask);
    writeResults(tempDir, "runs.json", [
      ...Array.from({ length: 1 }, (_, index) => sampleRun({ result_id: `b-${index}`, arm: "baseline" })),
      ...Array.from({ length: 1 }, (_, index) => sampleRun({ result_id: `c-${index}`, arm: "candidate", selected_model: "fast-local" })),
    ]);
    const evidence = buildSummary(tempDir, { minimumSamples: 1 }).perClass.formatting.evidence.evidence;
    const canonical = JSON.parse(fs.readFileSync(path.join(root, "benchmarks/fixtures/model-routing-quality-evidence.json"), "utf8"));
    assert.deepEqual(Object.keys(evidence).filter((key) => key !== "note"), Object.keys(canonical).filter((key) => key !== "note"));
    assert.deepEqual(Object.keys(canonical.provenance), Object.keys(canonical.provenance).filter((key) => key in evidence.provenance));
    for (const arm of ["baseline", "candidate"]) {
      assert.deepEqual(Object.keys(evidence[arm]), Object.keys(canonical[arm]));
    }
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("gate passes on a freshly regenerated repo summary and fails on drift or tampering", () => {
  execFileSync(process.execPath, ["scripts/run-class-c-tasks.mjs"], { cwd: root, stdio: "ignore" });
  const passing = execFileSync(process.execPath, ["scripts/check-class-c-tasks.mjs"], { encoding: "utf8" });
  assert.equal(JSON.parse(passing).ok, true);

  const jsonPath = path.join(root, "benchmarks/results/class-c-summary.json");
  const original = fs.readFileSync(jsonPath, "utf8");
  try {
    const tampered = JSON.parse(original);
    const firstClass = Object.keys(tampered.perClass)[0];
    if (!tampered.perClass[firstClass].evidence) {
      tampered.perClass[firstClass].evidence = {
        evidence: {
          schemaVersion: 1,
          evidenceClass: "local_runtime_observation",
          promotionEligible: true,
          minimumSamples: 100,
          provenance: { taskClass: firstClass },
          baseline: { sampleCount: 2, successfulTaskCount: 2, successRateBps: 10000, qualityScoreBps: 10000, p95LatencyMs: 10, successfulTaskCostMicros: 500, followUpReworkRateBps: 0 },
          candidate: { sampleCount: 2, successfulTaskCount: 2, successRateBps: 10000, qualityScoreBps: 10000, p95LatencyMs: 10, successfulTaskCostMicros: 400, followUpReworkRateBps: 0 },
        },
      };
    } else {
      tampered.perClass[firstClass].evidence.evidence.promotionEligible = true;
    }
    fs.writeFileSync(jsonPath, `${JSON.stringify(tampered, null, 2)}\n`);
    try {
      execFileSync(process.execPath, ["scripts/check-class-c-tasks.mjs"], { stdio: ["ignore", "pipe", "pipe"] });
      assert.fail("expected tampered summary to fail the gate");
    } catch (error) {
      assert.match(String(error.stderr ?? error.message), /promotionEligible does not match recomputed|drifted from deterministic rebuild/);
    }

    fs.writeFileSync(jsonPath, original.replace(/\n$/, ""));
    try {
      execFileSync(process.execPath, ["scripts/check-class-c-tasks.mjs"], { stdio: ["ignore", "pipe", "pipe"] });
      assert.fail("expected drifted summary to fail the gate");
    } catch (error) {
      assert.match(String(error.stderr ?? error.message), /drifted from deterministic rebuild/);
    }
  } finally {
    fs.writeFileSync(jsonPath, original);
    execFileSync(process.execPath, ["scripts/run-class-c-tasks.mjs"], { cwd: root, stdio: "ignore" });
  }
});

