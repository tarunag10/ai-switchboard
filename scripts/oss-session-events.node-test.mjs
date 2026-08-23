import test from "node:test";
import assert from "node:assert/strict";
import { buildSessionEventLedger, transitionSession, forkSessionAtEvent } from "./oss-session-events.mjs";

const base = { sessionId: "session-1", events: [
  { eventId: "e0", sessionId: "session-1", sequence: 0, kind: "started" },
  { eventId: "e1", sessionId: "session-1", sequence: 1, kind: "checkpoint" },
] };

test("builds a bounded metadata-only session ledger", () => {
  const ledger = buildSessionEventLedger(base);
  assert.equal(ledger.status, "active");
  assert.equal(ledger.executionMode, "observe_only");
  assert.equal(ledger.eventCount, 2);
  assert.equal(ledger.events[1].kind, "checkpoint");
});

test("enforces lifecycle transitions and terminal states", () => {
  const paused = transitionSession(buildSessionEventLedger(base), "pause");
  const resumed = transitionSession(paused, "resume");
  assert.equal(resumed.status, "active");
  const cancelled = transitionSession(resumed, "cancel");
  assert.equal(cancelled.status, "cancelled");
  assert.throws(() => transitionSession(cancelled, "resume"), /not allowed/);
});

test("forks only at a known event with deterministic identity", () => {
  const ledger = buildSessionEventLedger(base);
  const first = forkSessionAtEvent(ledger, "e1");
  const second = forkSessionAtEvent(ledger, "e1");
  assert.deepEqual(first, second);
  assert.equal(first.executionMode, "observe_only");
  assert.throws(() => forkSessionAtEvent(ledger, "missing"), /not in the session ledger/);
});

test("rejects sensitive fields, gaps, duplicates, and cross-session events", () => {
  assert.throws(() => buildSessionEventLedger({ ...base, events: [{ ...base.events[0], output: "secret" }] }), /forbidden/);
  assert.throws(() => buildSessionEventLedger({ ...base, events: [{ ...base.events[0], sequence: 1 }] }), /contiguous/);
  assert.throws(() => buildSessionEventLedger({ ...base, events: [base.events[0], base.events[0]] }), /duplicate/);
  assert.throws(() => buildSessionEventLedger({ ...base, events: [{ ...base.events[0], sessionId: "other" }] }), /another session/);
});

test("replays lifecycle order instead of trusting the last event kind", () => {
  assert.throws(
    () => buildSessionEventLedger({
      ...base,
      events: [
        ...base.events,
        { eventId: "e2", sessionId: "session-1", sequence: 2, kind: "completed" },
        { eventId: "e3", sessionId: "session-1", sequence: 3, kind: "resumed" },
      ],
    }),
    /not allowed after completed/,
  );
  assert.throws(
    () => buildSessionEventLedger({
      ...base,
      events: [{ ...base.events[0], kind: "paused" }, base.events[1]],
    }),
    /must start with a started event/,
  );
  assert.throws(
    () => buildSessionEventLedger({
      ...base,
      events: [...base.events, { eventId: "e2", sessionId: "session-1", sequence: 2, kind: "resumed" }],
    }),
    /requires a paused session/,
  );
});
