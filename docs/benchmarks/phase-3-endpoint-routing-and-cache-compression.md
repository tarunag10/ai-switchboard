# Phase 3 Endpoint Routing and Cache-Compression Evidence

Endpoint routing and model routing are separate decisions. The endpoint policy
accepts one requested model and rejects endpoints that cannot serve that exact
model. It does not substitute a cheaper or more capable model. Health,
verification, required features, privacy, explicit cost limits, and explicit
queue-latency limits are hard gates. Preference, healthy versus degraded state,
locality, measured cost, and measured queue latency provide a deterministic,
explainable rank only after those gates pass.

The cache-compression recommendation compares exactly four variants:
`no_compression`, `normal`, `cache_safe`, and `aggressive`. The checked-in
fixture at
`benchmarks/fixtures/compression-four-variant-evidence.json` is a deterministic
replay evidence set, not a claim about every live provider. Automatic
recommendations require measured results with at least 30 samples, 98% agent
success, 99% relevant-fact retention, and at most 1% wrong omissions.

Provider-declared or sufficiently sampled prompt-cache hits prefer the
cache-safe profile when its measured gate passes. Unknown cache evidence uses
normal compression only when its gate passes. Aggressive compression requires
both explicit user opt-in and passing measured evidence. Missing, malformed,
unmeasured, or failing evidence falls back to no compression.

The deterministic evidence keeps aggressive compression gated: its additional
token savings coincide with lower agent success, lower fact retention, more
wrong omissions, and lower prompt-cache hit rate. This is intentional evidence
for the conservative default, not a promotion result.
