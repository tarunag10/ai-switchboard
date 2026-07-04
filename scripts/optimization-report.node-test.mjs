import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { buildOptimizationReport, renderMarkdown } from "./optimization-report.mjs";

const safeSnapshot = {
  generatedAt: "2026-07-04T00:00:00.000Z",
  source: "fixture:safe-token-metrics",
  tokenXray: {
    promptTokens: 1200,
    completionTokens: 300,
    toolTokens: 500,
    contextWindow: 8000,
    segments: [
      { label: "repo map", tokens: 700 },
      { label: "test output digest", tokens: 400 },
    ],
  },
  redundancy: {
    duplicateBlocks: 2,
    duplicateTokens: 250,
    repeatedToolCalls: 1,
  },
  cacheEfficiency: {
    cacheReadTokens: 900,
    cacheWriteTokens: 100,
    totalInputTokens: 2000,
  },
  compaction: {
    beforeTokens: 6000,
    afterTokens: 2200,
    triggerTokens: 7000,
    summaryTokens: 650,
  },
  modelRouting: {
    fallbackCount: 1,
    routes: [
      { model: "gpt-5-codex", requests: 4, inputTokens: 1200, outputTokens: 300, reason: "code edits" },
      { model: "gpt-5-mini", requests: 2, inputTokens: 500, outputTokens: 120, reason: "classification" },
    ],
  },
  rtkPresets: {
    mode: "auto",
    command: "npm test",
    files: ["src/app.test.ts"],
    output: "FAIL src/app.test.ts\nAssertionError: expected safe metric\n",
  },
};

test("buildOptimizationReport returns all optimization sections from safe metrics", () => {
  const report = buildOptimizationReport(safeSnapshot, {});
  assert.equal(report.tokenXray.totalTokens, 2000);
  assert.equal(report.redundancy.duplicatePct, "12.5%");
  assert.equal(report.cacheEfficiency.hitRate, "45.0%");
  assert.equal(report.compaction.savedTokens, 3800);
  assert.equal(report.modelRouting.primaryModel, "gpt-5-codex");
  assert.ok(report.rtkPresets.available.includes(report.rtkPresets.selectedPreset));
});

test("renderMarkdown omits raw prompt fields and includes CLI sections", () => {
  const markdown = renderMarkdown(buildOptimizationReport(safeSnapshot, {}));
  assert.match(markdown, /## Token X-ray/);
  assert.match(markdown, /## Redundancy/);
  assert.match(markdown, /## Cache Efficiency/);
  assert.match(markdown, /## Compaction/);
  assert.match(markdown, /## Model Routing/);
  assert.match(markdown, /## RTK Presets/);
  assert.doesNotMatch(markdown, /do not persist raw prompt text/i);
});

test("CLI writes JSON from a safe snapshot fixture", () => {
  const dir = mkdtempSync(path.join(tmpdir(), "optimization-report-"));
  const input = path.join(dir, "snapshot.json");
  const output = path.join(dir, "report.json");
  writeFileSync(input, `${JSON.stringify(safeSnapshot, null, 2)}\n`);
  const result = spawnSync(process.execPath, ["scripts/optimization-report.mjs", "--input", input, "--json", "--output", output], {
    cwd: process.cwd(),
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  const report = JSON.parse(readFileSync(output, "utf8"));
  assert.equal(report.source, "fixture:safe-token-metrics");
  assert.ok(report.rtkPresets.available.includes(report.rtkPresets.selectedPreset));
});

test("CLI rejects raw prompt-bearing JSON fields", () => {
  const dir = mkdtempSync(path.join(tmpdir(), "optimization-report-"));
  const input = path.join(dir, "unsafe.json");
  writeFileSync(input, JSON.stringify({ tokenXray: { prompt: "do not persist raw prompt text" } }));
  const result = spawnSync(process.execPath, ["scripts/optimization-report.mjs", "--input", input, "--json"], {
    cwd: process.cwd(),
    encoding: "utf8",
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unsafe raw prompt field/);
});

test("CLI keeps rtk preset JSON payload compatible", () => {
  const result = spawnSync(process.execPath, [
    "scripts/optimization-report.mjs",
    "--rtk-presets",
    "--json",
    "--command",
    "npm test",
    "--preset-output",
    "FAIL src/app.test.ts\nAssertionError: expected safe metric\n",
  ], {
    cwd: process.cwd(),
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  const payload = JSON.parse(result.stdout);
  assert.ok(payload.available.includes(payload.selectedPreset));
  assert.ok(Array.isArray(payload.available));
});
