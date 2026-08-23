import test from "node:test";
import assert from "node:assert/strict";
import { deterministicRandomStrategy, stagedStrategy, escalationStrategy, fallbackStrategy } from "./oss-harness-strategies.mjs";

const candidates = [{ id: "baseline" }, { id: "candidate" }, { id: "fallback" }];

test("random strategy is deterministic and observe-only", () => {
  const first = deterministicRandomStrategy({ candidates, seed: 42 });
  assert.deepEqual(first, deterministicRandomStrategy({ candidates: [...candidates], seed: 42 }));
  assert.equal(first.executionMode, "observe_only");
  assert.equal(first.automaticPromotion, "disabled");
});

test("stage strategy selects the first eligible stage", () => {
  const result = stagedStrategy({ candidates: [{ id: "a", healthy: false }, { id: "b" }, { id: "c" }], stages: [{ id: "primary", candidateIds: ["a"] }, { id: "secondary", candidateIds: ["b", "c"] }] });
  assert.equal(result.selectedCandidateId, "b");
  assert.equal(result.stageId, "secondary");
});

test("escalation is bounded and fails closed when exhausted", () => {
  assert.equal(escalationStrategy({ candidates, failureCount: 1 }).selectedCandidateId, "candidate");
  assert.equal(escalationStrategy({ candidates: [{ id: "a", healthy: false }], failureCount: 0 }).selectedCandidateId, null);
});

test("fallback uses only an allowlisted healthy fallback", () => {
  const result = fallbackStrategy({ primary: "baseline", fallback: "fallback", candidates: [{ id: "baseline", healthy: false }, { id: "fallback" }] });
  assert.equal(result.selectedCandidateId, "fallback");
  assert.equal(result.fallbackUsed, true);
  assert.equal(fallbackStrategy({ primary: "baseline", fallback: "fallback", candidates: [{ id: "baseline", healthy: false }, { id: "fallback", healthy: false }] }).selectedCandidateId, null);
});

test("strategy fixtures reject duplicate candidates and invalid seeds", () => {
  assert.throws(() => deterministicRandomStrategy({ candidates: [{ id: "a" }, { id: "a" }], seed: 1 }), /duplicate candidate/);
  assert.throws(() => deterministicRandomStrategy({ candidates, seed: -1 }), /seed/);
});
