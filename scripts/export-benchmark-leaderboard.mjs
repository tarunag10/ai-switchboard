#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const fixturesPath = path.join(root, "benchmarks/fixtures.json");
const outputPath = path.join(root, "benchmarks/leaderboard.json");

const fixtures = JSON.parse(fs.readFileSync(fixturesPath, "utf8"));
if (!Array.isArray(fixtures)) {
  console.error("fixtures.json must be an array");
  process.exit(1);
}

const leaderboard = fixtures.map((fixture) => {
  const relevant = fixture.relevantFacts?.length ?? 0;
  const retained =
    relevant === 0
      ? 100
      : ((fixture.optimizedFacts ?? []).filter((fact) =>
          (fixture.relevantFacts ?? []).includes(fact),
        ).length /
          relevant) *
        100;
  const omissionRate =
    relevant === 0
      ? 0
      : ((fixture.wrongOmissions ?? []).length / relevant) * 100;
  return {
    name: fixture.name,
    category: fixture.category,
    relevantFactRetentionPct: Number(retained.toFixed(2)),
    wrongOmissionRatePct: Number(omissionRate.toFixed(2)),
    estimatedInputTokens: fixture.estimatedInputTokens ?? null,
    estimatedOutputTokens: fixture.estimatedOutputTokens ?? null,
    secretSafe: fixture.secretSafe !== false,
  };
});

const payload = {
  generatedAt: new Date().toISOString(),
  fixtureCount: leaderboard.length,
  categories: [...new Set(leaderboard.map((entry) => entry.category))].sort(),
  entries: leaderboard.sort((a, b) => b.relevantFactRetentionPct - a.relevantFactRetentionPct),
};

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log(
  JSON.stringify(
    {
      ok: true,
      outputPath: "benchmarks/leaderboard.json",
      fixtureCount: payload.fixtureCount,
      categories: payload.categories,
    },
    null,
    2,
  ),
);
