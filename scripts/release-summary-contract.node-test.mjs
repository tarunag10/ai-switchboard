import test from "node:test";
import assert from "node:assert/strict";
import { assessSummaryFreshness, extractGeneratedAt, validateSummaryGeneratedAt } from "./release-summary-contract.mjs";

const now = Date.parse("2026-08-21T12:00:00.000Z");

test("extracts the structured timestamp from a summary", () => {
  assert.equal(
    extractGeneratedAt("# Summary\nGenerated: 2026-08-21T11:00:00.000Z\n"),
    "2026-08-21T11:00:00.000Z",
  );
  assert.equal(extractGeneratedAt("# Summary\nNo timestamp\n"), null);
});

test("requires timestamps for present or passed summaries", () => {
  assert.equal(validateSummaryGeneratedAt(null, { now }).ok, true);
  assert.equal(validateSummaryGeneratedAt(null, { present: true, now }).ok, false);
  assert.equal(validateSummaryGeneratedAt(null, { passed: true, now }).ok, false);
});

test("validates fresh summary timestamps and rejects stale ones", () => {
  assert.equal(
    validateSummaryGeneratedAt("2026-08-21T11:00:00.000Z", { present: true, now }).ok,
    true,
  );
  const stale = validateSummaryGeneratedAt("2026-08-01T11:00:00.000Z", { present: true, now });
  assert.equal(stale.ok, false);
  assert.match(stale.reason, /stale/);
});

test("requires matching fresh JSON and Markdown lineage", () => {
  const fresh = assessSummaryFreshness({
    summaryGeneratedAt: "2026-08-21T11:00:00.000Z",
    jsonGeneratedAt: "2026-08-21T11:00:00.000Z",
    present: true,
    passed: true,
  }, { now });
  assert.equal(fresh.fresh, true);
  const mismatch = assessSummaryFreshness({
    summaryGeneratedAt: "2026-08-21T11:00:00.000Z",
    jsonGeneratedAt: "2026-08-21T11:01:00.000Z",
    present: true,
    passed: true,
  }, { now });
  assert.equal(mismatch.fresh, false);
  assert.match(mismatch.reason, /differs/);
});
