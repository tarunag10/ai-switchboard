import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

test("connector lifecycle evidence resolves to approved Rust tests", () => {
  const output = execFileSync(process.execPath, ["scripts/check-connector-lifecycle-matrix.mjs"], { encoding: "utf8" });
  const result = JSON.parse(output);
  assert.equal(result.ok, true);
  assert.equal(result.approvedTestFile, "src-tauri/src/client_adapters_tests.rs");
  assert.ok(result.evidenceLinks.length >= 70);
  assert.ok(result.evidenceLinks.every((link) => link.test.length > 0));
});
