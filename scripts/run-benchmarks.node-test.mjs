import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import {
  buildManifest,
  compareWithBaseline,
  readJson,
  renderMarkdown,
} from "./benchmark-manifest.mjs";

const root = path.resolve(import.meta.dirname, "..");
const fixtures = readJson(path.join(root, "benchmarks/fixtures.json"));
const schema = readJson(path.join(root, "benchmarks/schema.json"));
const baseline = readJson(path.join(root, "benchmarks/baseline.json"));
const deterministicEnv = {
  SWITCHBOARD_BENCHMARK_COMMIT: "0123456789abcdef",
  SWITCHBOARD_BENCHMARK_PLATFORM: "test-platform",
  SWITCHBOARD_BENCHMARK_HEADROOM_VERSION: "headroom-test",
  SWITCHBOARD_BENCHMARK_RTK_VERSION: "rtk-test",
};

test("builds the complete deterministic offline manifest", () => {
  const first = buildManifest({ root, fixtures, schema, baseline, env: deterministicEnv });
  const second = buildManifest({ root, fixtures, schema, baseline, env: deterministicEnv });

  assert.deepEqual(first, second);
  assert.equal(first.results.length, fixtures.length);
  assert.equal(first.baselineComparison.status, "pass");
  for (const result of first.results) {
    for (const field of [
      "originalTokens",
      "optimizedTokens",
      "savedTokens",
      "savedPct",
      "latencyOverheadMs",
      "relevantFactRetentionPct",
      "wrongOmissionRatePct",
      "agentSuccessProxy",
      "qualityLabel",
    ]) {
      assert.ok(field in result, `${result.name} lacks ${field}`);
    }
  }
});

test("reports threshold violations against the stored baseline", () => {
  const manifest = buildManifest({ root, fixtures, schema, baseline, env: deterministicEnv });
  const degraded = structuredClone(manifest);
  degraded.aggregate.savedPct = baseline.aggregate.savedPct - 3;

  const comparison = compareWithBaseline(
    degraded,
    baseline,
    schema.regressionThresholds,
  );

  assert.equal(comparison.status, "regression");
  assert.match(comparison.violations.join("\n"), /aggregate saved percent dropped/);
});

test("renders a human-readable summary with thresholds and all fixture rows", () => {
  const manifest = buildManifest({ root, fixtures, schema, baseline, env: deterministicEnv });
  const markdown = renderMarkdown(manifest);

  assert.match(markdown, /^# Offline benchmark summary/m);
  assert.match(markdown, /## Regression thresholds/);
  assert.match(markdown, /offline_static_fixture/);
  assert.equal(
    markdown.split("\n").filter((line) => line.startsWith("| ") && line.includes("offline_static_fixture")).length,
    fixtures.length,
  );
});

test("CLI writes paired JSON and Markdown artifacts while keeping stdout machine-readable", () => {
  const outputDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-benchmarks-"));
  const run = spawnSync(
    process.execPath,
    ["scripts/run-benchmarks.mjs", "--check", "--output-dir", outputDirectory],
    { cwd: root, encoding: "utf8", env: { ...process.env, ...deterministicEnv } },
  );

  assert.equal(run.status, 0, run.stderr);
  const stdoutManifest = JSON.parse(run.stdout);
  const artifactManifest = readJson(path.join(outputDirectory, "manifest.json"));
  assert.deepEqual(stdoutManifest, artifactManifest);
  assert.match(
    fs.readFileSync(path.join(outputDirectory, "summary.md"), "utf8"),
    /Baseline comparison: \*\*pass\*\*/,
  );
});
