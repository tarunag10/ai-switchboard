import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { canonicalLifecycleStages, validateLifecycleSchema } from "./connector-lifecycle-contract.mjs";

test("connector lifecycle evidence resolves to approved Rust tests", () => {
  const output = execFileSync(process.execPath, ["scripts/check-connector-lifecycle-matrix.mjs"], { encoding: "utf8" });
  const result = JSON.parse(output);
  assert.equal(result.ok, true);
  assert.equal(result.approvedTestFile, "src-tauri/src/client_adapters_tests.rs");
  assert.ok(result.evidenceLinks.length >= 70);
  assert.ok(result.evidenceLinks.every((link) => link.test.length > 0));
});

test("rejects duplicate IDs and unknown lifecycle stages", () => {
  const manifest = [{ id: "cursor", support_status: "gated" }, { id: "cursor", support_status: "gated" }];
  const fixtures = {
    requiredStages: [...canonicalLifecycleStages],
    connectors: [
      { id: "cursor", stages: { detect: null, mystery: "bad" } },
      { id: "cursor", stages: { detect: null } },
    ],
  };
  const failures = validateLifecycleSchema(manifest, fixtures);
  assert.match(failures.join("\n"), /duplicate manifest connector ID/);
  assert.match(failures.join("\n"), /duplicate lifecycle fixture ID/);
  assert.match(failures.join("\n"), /unknown lifecycle stage mystery/);
});

test("rejects reordered or duplicated required stages", () => {
  const failures = validateLifecycleSchema([], { requiredStages: ["detect", "detect"], connectors: [] });
  assert.match(failures.join("\n"), /requiredStages must be a unique array/);
});
