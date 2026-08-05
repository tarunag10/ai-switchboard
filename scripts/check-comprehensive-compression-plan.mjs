#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const planDir = path.join(root, "docs/world-class-token-savings");
const comprehensivePlan = path.join(
  planDir,
  "COMPREHENSIVE-TOKEN-COMPRESSION-IMPLEMENTATION-PLAN.md",
);
const designDoc = path.join(planDir, "COMPREHENSIVE-TOKEN-COMPRESSION-DESIGN.md");
const sliceStatusPath = path.join(planDir, "slice-status.json");
const benchmarkSchemaPath = path.join(root, "benchmarks/schema.json");
const benchmarkFixturesPath = path.join(root, "benchmarks/fixtures.json");

const requiredGateScripts = [
  "scripts/check-chonkify-promotion-gate.mjs",
  "scripts/check-leanctx-promotion-gate.mjs",
  "scripts/check-pxpipe-promotion-gate.mjs",
  "scripts/check-semantic-cache-v2-gate.mjs",
];

function fail(message) {
  console.error(`comprehensive compression plan check failed: ${message}`);
  process.exit(1);
}

for (const file of [comprehensivePlan, designDoc, sliceStatusPath]) {
  if (!fs.existsSync(file)) {
    fail(`missing ${path.relative(root, file)}`);
  }
}

for (const script of requiredGateScripts) {
  if (!fs.existsSync(path.join(root, script))) {
    fail(`missing gate script ${script}`);
  }
}

const sliceStatus = JSON.parse(fs.readFileSync(sliceStatusPath, "utf8"));
for (const phaseId of ["C0", "C1", "C2", "C3", "C4", "C5"]) {
  if (!sliceStatus.phases?.[phaseId]) {
    fail(`slice-status.json missing phase ${phaseId}`);
  }
}

const schema = JSON.parse(fs.readFileSync(benchmarkSchemaPath, "utf8"));
const fixtures = JSON.parse(fs.readFileSync(benchmarkFixturesPath, "utf8"));
const minimumFixtures = Math.max(schema.minimumFixtures ?? 8, 12);
if (!Array.isArray(fixtures) || fixtures.length < minimumFixtures) {
  fail(`expected at least ${minimumFixtures} benchmark fixtures, found ${fixtures.length}`);
}

const categories = new Set(fixtures.map((fixture) => fixture.category));
if (categories.size < 6) {
  fail(`expected at least 6 benchmark categories, found ${categories.size}`);
}

for (const fixture of fixtures) {
  const omissions =
    fixture.relevantFacts?.length === 0
      ? 0
      : (fixture.wrongOmissions?.length ?? 0) / fixture.relevantFacts.length;
  if (omissions > (schema.qualityGates?.maximumWrongOmissionRatePct ?? 0) / 100) {
    fail(`fixture "${fixture.name}" exceeds wrong-omission gate`);
  }
}

for (const script of requiredGateScripts) {
  const result = spawnSync("node", [path.join(root, script)], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    fail(`${script} failed:\n${result.stderr || result.stdout}`);
  }
}

console.log(
  JSON.stringify(
    {
      ok: true,
      fixtureCount: fixtures.length,
      categories: [...categories].sort(),
      compressionPhases: ["C0", "C1", "C2", "C3", "C4", "C5"],
      gateScripts: requiredGateScripts,
    },
    null,
    2,
  ),
);
