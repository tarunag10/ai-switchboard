#!/usr/bin/env node
import fs from "node:fs";

const schemaPath = "benchmarks/schema.json";
const fixturesPath = "benchmarks/fixtures.json";

function fail(message) {
  console.error(`world-class benchmarks check failed: ${message}`);
  process.exit(1);
}

const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const fixtures = JSON.parse(fs.readFileSync(fixturesPath, "utf8"));

if (!Array.isArray(fixtures)) {
  fail(`${fixturesPath} must be an array`);
}

if (fixtures.length < schema.minimumFixtures) {
  fail(`expected at least ${schema.minimumFixtures} fixtures, found ${fixtures.length}`);
}

const categories = new Set(fixtures.map((fixture) => fixture.category));
if (categories.size < schema.minimumCategories) {
  fail(`expected at least ${schema.minimumCategories} categories, found ${categories.size}`);
}

for (const requiredCategory of schema.requiredCategories) {
  if (!categories.has(requiredCategory)) {
    fail(`missing required category: ${requiredCategory}`);
  }
}

for (const fixture of fixtures) {
  for (const field of schema.requiredFields) {
    if (!(field in fixture)) {
      fail(`fixture "${fixture.name ?? "unknown"}" missing field ${field}`);
    }
  }
  const retention =
    fixture.relevantFacts.length === 0
      ? 100
      : (fixture.optimizedFacts.filter((fact) => fixture.relevantFacts.includes(fact))
          .length /
          fixture.relevantFacts.length) *
        100;
  if (retention < schema.qualityGates.minimumRelevantFactRetentionPct) {
    fail(`fixture "${fixture.name}" retention ${retention}% below gate`);
  }
  const omissionRate =
    fixture.relevantFacts.length === 0
      ? 0
      : (fixture.wrongOmissions.length / fixture.relevantFacts.length) * 100;
  if (omissionRate > schema.qualityGates.maximumWrongOmissionRatePct) {
    fail(`fixture "${fixture.name}" wrong omission rate ${omissionRate}% above gate`);
  }
}

console.log(
  JSON.stringify(
    {
      ok: true,
      fixtureCount: fixtures.length,
      categories: [...categories].sort(),
      schemaVersion: schema.schemaVersion,
    },
    null,
    2,
  ),
);
