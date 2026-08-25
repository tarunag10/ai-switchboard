import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { CONNECTOR_LIFECYCLE_FIXTURE_VERSION, canonicalLifecycleStages, lifecycleIntentForTest, lifecycleIntentMarkerFailures, runtimeStageByFixtureStage, supportStatusFailures, validateLifecycleSchema } from "./connector-lifecycle-contract.mjs";

function fixtureCatalog(connectors = [], overrides = {}) {
  return {
    version: CONNECTOR_LIFECYCLE_FIXTURE_VERSION,
    requiredStages: [...canonicalLifecycleStages],
    connectors,
    ...overrides,
  };
}

test("connector lifecycle evidence resolves to approved Rust tests", () => {
  const output = execFileSync(process.execPath, ["scripts/check-connector-lifecycle-matrix.mjs"], { encoding: "utf8" });
  const result = JSON.parse(output);
  assert.equal(result.ok, true);
  assert.equal(result.fixtureVersion, CONNECTOR_LIFECYCLE_FIXTURE_VERSION);
  assert.equal(result.approvedTestFile, "src-tauri/src/client_adapters_tests.rs");
  assert.ok(result.evidenceLinks.length >= 70);
  assert.ok(result.evidenceLinks.every((link) => link.test.length > 0));
});

test("rejects duplicate IDs and unknown lifecycle stages", () => {
  const manifest = [{ id: "cursor", support_status: "planned" }, { id: "cursor", support_status: "planned" }];
  const fixtures = fixtureCatalog([
      { id: "cursor", stages: { detect: null, mystery: "bad" } },
      { id: "cursor", stages: { detect: null } },
  ]);
  const failures = validateLifecycleSchema(manifest, fixtures);
  assert.match(failures.join("\n"), /duplicate manifest connector ID/);
  assert.match(failures.join("\n"), /duplicate lifecycle fixture ID/);
  assert.match(failures.join("\n"), /unknown lifecycle stage mystery/);
  assert.match(failures.join("\n"), /stages must declare every canonical stage exactly once in order/);
});

test("rejects unknown connector support statuses", () => {
  assert.deepEqual(
    supportStatusFailures([{ id: "example", support_status: "experimental" }]),
    ["example: unknown support_status"],
  );
});

test("rejects omitted required lifecycle stages", () => {
  const failures = validateLifecycleSchema(
    [{ id: "cursor", support_status: "gated" }],
    fixtureCatalog([{ id: "cursor", stages: { detect: null } }]),
  );
  assert.match(failures.join("\n"), /stages must declare every canonical stage exactly once in order/);
});

test("rejects reordered or duplicated required stages", () => {
  const duplicated = validateLifecycleSchema(
    [],
    fixtureCatalog([], { requiredStages: ["detect", "detect"] }),
  );
  assert.match(duplicated.join("\n"), /requiredStages must be a unique array/);

  for (const requiredStages of [
    [],
    [...canonicalLifecycleStages].reverse(),
    [...canonicalLifecycleStages.slice(0, -1), "unknown"],
  ]) {
    const failures = validateLifecycleSchema(
      [],
      fixtureCatalog([], { requiredStages }),
    );
    assert.match(failures.join("\n"), /requiredStages must equal/);
  }
});

test("requires lifecycle fixture version exactly one before schema indexing", () => {
  assert.match(
    validateLifecycleSchema([], null).join("\n"),
    /lifecycle fixtures must be an object/,
  );

  for (const version of [undefined, 0, 2, "1"]) {
    const fixture = fixtureCatalog([], { version });
    if (version === undefined) delete fixture.version;
    assert.match(
      validateLifecycleSchema([], fixture).join("\n"),
      /lifecycle fixture version must be 1/,
    );
  }
});

test("keeps fixture stages mapped to the runtime lifecycle vocabulary", () => {
  assert.equal(runtimeStageByFixtureStage.preview, "dryRunDiff");
  assert.equal(runtimeStageByFixtureStage.off, "offCleanup");
  assert.deepEqual(Object.keys(runtimeStageByFixtureStage), canonicalLifecycleStages);
});

test("requires explicit stage intent markers for linked tests", () => {
  const source = "// lifecycle-intent: preview,off\nfn combined() {}\n";
  assert.deepEqual(lifecycleIntentForTest(source, "combined"), ["preview", "off"]);
  assert.deepEqual(lifecycleIntentMarkerFailures(source), []);
  assert.match(
    lifecycleIntentMarkerFailures("// lifecycle-intent: mystery\nfn bad() {}\n").join("\n"),
    /unknown lifecycle intent stage: mystery/,
  );
  assert.equal(lifecycleIntentForTest("fn missing() {}\n", "missing"), null);
});
