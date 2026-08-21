import test from "node:test";
import assert from "node:assert/strict";
import { validateReleaseEvidenceTimestamp } from "./release-evidence-time.mjs";

const now = Date.parse("2026-08-21T12:00:00.000Z");

test("accepts a current RFC3339 timestamp with timezone", () => {
  const result = validateReleaseEvidenceTimestamp(
    "2026-08-21T11:59:59.000Z",
    { now },
  );
  assert.equal(result.ok, true);
});

test("rejects timestamps without an explicit timezone", () => {
  const result = validateReleaseEvidenceTimestamp(
    "2026-08-21T11:59:59.000",
    { now },
  );
  assert.equal(result.ok, false);
  assert.match(result.reason, /timezone/);
});

test("rejects malformed timestamps", () => {
  const result = validateReleaseEvidenceTimestamp("not-a-date", { now });
  assert.equal(result.ok, false);
  assert.match(result.reason, /valid ISO/);
});

test("rejects stale evidence", () => {
  const result = validateReleaseEvidenceTimestamp(
    "2026-08-13T11:59:59.000Z",
    { now, maxAgeMs: 7 * 24 * 60 * 60 * 1000 },
  );
  assert.equal(result.ok, false);
  assert.match(result.reason, /stale/);
});

test("rejects evidence too far in the future but allows clock skew", () => {
  const future = validateReleaseEvidenceTimestamp(
    "2026-08-21T12:06:00.000Z",
    { now },
  );
  assert.equal(future.ok, false);
  assert.match(future.reason, /future/);

  const skewed = validateReleaseEvidenceTimestamp(
    "2026-08-21T12:04:00.000Z",
    { now },
  );
  assert.equal(skewed.ok, true);
});
