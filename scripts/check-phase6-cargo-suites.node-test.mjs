import test from "node:test";
import assert from "node:assert/strict";

import { isPassingSuiteResult } from "./check-phase6-cargo-suites.mjs";

test("phase-6 suite timeout never counts as a pass", () => {
  assert.equal(isPassingSuiteResult({ status: 0, timedOut: true }), false);
  assert.equal(isPassingSuiteResult({ status: 1, timedOut: false }), false);
  assert.equal(isPassingSuiteResult({ status: 0, timedOut: false }), true);
});

test("phase-6 launcher errors never count as a pass", () => {
  assert.equal(isPassingSuiteResult({ status: 0, timedOut: false, error: new Error("spawn") }), false);
});
