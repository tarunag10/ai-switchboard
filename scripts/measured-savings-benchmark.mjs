#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const fixturePath = path.join(repoRoot, "fixtures", "savings-benchmarks.json");
const distDir = path.join(repoRoot, "dist");
const jsonPath = path.join(distDir, "measured-savings-benchmark.json");
const markdownPath = path.join(distDir, "measured-savings-benchmark.md");

function estimateTokens(text) {
  const normalized = String(text ?? "").trim();
  if (!normalized) return 0;
  const words = normalized.match(/[A-Za-z0-9_'-]+|[^\sA-Za-z0-9_'-]/g) ?? [];
  return Math.max(1, Math.ceil(words.length * 1.25));
}

function fail(message) {
  console.error(`Measured savings benchmark failed: ${message}`);
  process.exit(1);
}

const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
if (fixture.schemaVersion !== 1 || !Array.isArray(fixture.fixtures)) {
  fail("fixture schemaVersion 1 with fixtures[] is required");
}

const generatedAt = new Date().toISOString();
const rows = fixture.fixtures.map((item) => {
  const beforeTokens = estimateTokens(item.before);
  const afterTokens = estimateTokens(item.after);
  const savedTokens = beforeTokens - afterTokens;
  const savingsPct = beforeTokens > 0 ? (savedTokens / beforeTokens) * 100 : 0;
  if (!item.id || !item.source || !item.label) {
    fail("each fixture needs id, source, and label");
  }
  if (beforeTokens <= 0 || afterTokens <= 0) {
    fail(`${item.id} has empty before/after text`);
  }
  if (savedTokens <= 0) {
    fail(`${item.id} does not save tokens`);
  }
  return {
    id: item.id,
    source: item.source,
    label: item.label,
    confidence: item.confidence ?? "measured_fixture",
    beforeTokens,
    afterTokens,
    savedTokens,
    savingsPct: Number(savingsPct.toFixed(1)),
  };
});

const totals = rows.reduce(
  (acc, row) => {
    acc.beforeTokens += row.beforeTokens;
    acc.afterTokens += row.afterTokens;
    acc.savedTokens += row.savedTokens;
    return acc;
  },
  { beforeTokens: 0, afterTokens: 0, savedTokens: 0 },
);
totals.savingsPct = Number(
  ((totals.savedTokens / Math.max(1, totals.beforeTokens)) * 100).toFixed(1),
);

const payload = {
  schemaVersion: 1,
  generatedAt,
  estimator: "word-and-symbol-count-x1.25",
  caveat:
    "Fixture benchmark evidence for repeatable before/after accounting; provider billing tokens can differ.",
  totals,
  rows,
};

fs.mkdirSync(distDir, { recursive: true });
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

const markdown = [
  "# Measured Savings Benchmark",
  "",
  `Generated: ${generatedAt}`,
  "",
  `Estimator: \`${payload.estimator}\``,
  "",
  `Total: ${totals.savedTokens} tokens saved (${totals.savingsPct}%) from ${totals.beforeTokens} before to ${totals.afterTokens} after.`,
  "",
  "| Source | Fixture | Before | After | Saved | Savings | Confidence |",
  "| --- | --- | ---: | ---: | ---: | ---: | --- |",
  ...rows.map(
    (row) =>
      `| ${row.source} | ${row.label} | ${row.beforeTokens} | ${row.afterTokens} | ${row.savedTokens} | ${row.savingsPct}% | ${row.confidence} |`,
  ),
  "",
  `Caveat: ${payload.caveat}`,
  "",
].join("\n");

fs.writeFileSync(markdownPath, markdown);
console.log(`Measured savings benchmark written: ${markdownPath}`);
console.log(`Measured savings benchmark JSON written: ${jsonPath}`);
