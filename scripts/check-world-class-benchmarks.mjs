#!/usr/bin/env node
import fs from "node:fs";
import { validateWorldClassFixtures } from "./world-class-benchmark-contract.mjs";

const schemaPath = "benchmarks/schema.json";
const fixturesPath = "benchmarks/fixtures.json";

function fail(message) {
  console.error(`world-class benchmarks check failed: ${message}`);
  process.exit(1);
}

const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const fixtures = JSON.parse(fs.readFileSync(fixturesPath, "utf8"));

const failures = validateWorldClassFixtures(schema, fixtures);
if (failures.length) failures.forEach(fail);
const categories = new Set(fixtures.map((fixture) => fixture.category));

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
