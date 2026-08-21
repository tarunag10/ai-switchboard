import test from "node:test";
import assert from "node:assert/strict";
import { readJson } from "./benchmark-manifest.mjs";
import { validateWorldClassFixtures } from "./world-class-benchmark-contract.mjs";

const schema = readJson("benchmarks/schema.json");

test("rejects duplicate identities and malformed input shapes", () => {
  const fixture = {
    category: "shell_output",
    name: "same",
    original: "a",
    optimized: "a",
    latencyOverheadMs: 1,
    relevantFacts: [],
    optimizedFacts: [],
    wrongOmissions: [],
    agentSuccessProxy: "pass",
  };
  const failures = validateWorldClassFixtures({ ...schema, minimumFixtures: 1, minimumCategories: 1, requiredCategories: ["shell_output"] }, [fixture, { ...fixture }, { ...fixture, name: "bad", latencyOverheadMs: "slow" }]);
  assert.match(failures.join("\n"), /duplicate benchmark fixture identity/);
  assert.match(failures.join("\n"), /latencyOverheadMs must be a finite/);
});

test("current world-class fixtures satisfy the stronger shape contract", () => {
  assert.deepEqual(validateWorldClassFixtures(schema, readJson("benchmarks/fixtures.json")), []);
});
