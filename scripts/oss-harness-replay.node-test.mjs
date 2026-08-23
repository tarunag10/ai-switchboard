import test from "node:test";
import assert from "node:assert/strict";
import { replayRedactedRouteEvents } from "./oss-harness-replay.mjs";

const events = [
  { eventId: "e-1", taskClass: "coding", route: "headroom", outcome: "success", latencyMs: 100 },
  { eventId: "e-2", taskClass: "coding", route: "switchyard_observe", outcome: "upstream_http_error", latencyMs: 200 },
  { eventId: "e-3", taskClass: "coding", route: "cache", outcome: "success", latencyMs: 50 },
];

test("replays redacted route metadata deterministically without provider traffic", () => {
  const first = replayRedactedRouteEvents({ schemaVersion: 1, events });
  const second = replayRedactedRouteEvents({ schemaVersion: 1, events: [...events] });
  assert.deepEqual(first, second);
  assert.equal(first.replayMode, "redacted_observe_only");
  assert.equal(first.providerTraffic, "none");
  assert.equal(first.routeCounts.switchyard_observe, 1);
  assert.equal(first.latency.p95Ms, 200);
});

test("rejects prompt, response, and credential-shaped fields", () => {
  for (const field of ["prompt", "response", "authorization", "apiKey"]) {
    assert.throws(
      () => replayRedactedRouteEvents({ schemaVersion: 1, events: [{ ...events[0], [field]: "secret" }] }),
      /sensitive field is not allowed/,
    );
  }
});

test("rejects unsupported routes and malformed latency", () => {
  assert.throws(() => replayRedactedRouteEvents({ schemaVersion: 1, events: [{ ...events[0], route: "automatic" }] }), /unsupported route/);
  assert.throws(() => replayRedactedRouteEvents({ schemaVersion: 1, events: [{ ...events[0], latencyMs: -1 }] }), /latencyMs/);
});

test("accepts ingress observations and rejects duplicate or oversized input", () => {
  const ingress = replayRedactedRouteEvents({
    schemaVersion: 1,
    events: [{ ...events[0], route: "ingress", outcome: "timeout" }],
  });
  assert.equal(ingress.routeCounts.ingress, 1);
  assert.throws(
    () => replayRedactedRouteEvents({ schemaVersion: 1, events: [events[0], events[0]] }),
    /duplicate eventId/,
  );
  assert.throws(
    () => replayRedactedRouteEvents({ schemaVersion: 1, events: Array.from({ length: 10_001 }, (_, index) => ({ ...events[0], eventId: `e-${index}` })) }),
    /exceeds 10000 events/,
  );
});

test("redaction checks sensitive keys case-insensitively", () => {
  assert.throws(
    () => replayRedactedRouteEvents({ schemaVersion: 1, events: [{ ...events[0], Authorization: "secret" }] }),
    /sensitive field is not allowed/,
  );
});
