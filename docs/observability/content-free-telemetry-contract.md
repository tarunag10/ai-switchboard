# Content-Free Telemetry Contract

Phase 5 defines a local export contract for standard observability adapters. It
does not enable a network exporter or change the local-only product default.

Each event contains only bounded identifiers and measurements:

- request ID;
- client adapter, optimization profile, engine, and action;
- endpoint and model identifiers;
- before, after, and cache-read token counts;
- optimizer, TTFT, inter-token, and end-to-end latency;
- cache and compression results;
- success status or a typed failure-reason enum;
- an optional quality-outcome reference.

There are deliberately no prompt, response, message, tool payload, HTTP header,
credential, arbitrary exception, or free-form error fields. Identifiers are
length-bounded and reject control characters and common secret-like patterns.

The OpenTelemetry-style projection uses a span name plus typed attributes. The
Prometheus-style projection emits counters and histogram observations. Request
IDs appear only as metric exemplars, not high-cardinality Prometheus labels.
The module provides data projections only; a later, separately authorized
integration must decide whether and where to export them.

The checked-in fixture under
`benchmarks/fixtures/content-free-observability-event.json` proves deterministic
serialization and coverage of the Phase 5 field set without content.
