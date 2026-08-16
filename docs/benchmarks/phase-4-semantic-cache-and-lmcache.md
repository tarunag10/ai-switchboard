# Phase 4 Semantic Cache and LMCache Gates

## Semantic-cache experiment

This phase adds an experiment contract, not a live response cache. A semantic
candidate must use the same verified representation provider, implementation
fingerprint, versioned encoder, and explicit quantized embedding,
meet a similarity threshold of at least 0.90, and match workspace, account,
model, task family, intent, repository revision, and dependency fingerprints.
Changing repositories and stale code are misses.

Tool turns, high-risk actions, arbitrary code generation, non-deterministic
requests, and temperatures above 0.2 are hard rejections. The quality gate
requires at least 100 measured samples, at least 98% hit precision, at least
98% successful cached tasks, and zero unsafe hits. The checked-in fixture is
deterministic evidence for the contract and does not enable runtime caching.
Promotion readiness remains false without a real, verified representation
provider even when the quality fixture passes.

## LMCache benchmark-only promotion

Official LMCache documentation describes a persistent, tiered KV-cache layer
integrated with serving engines through connectors. Reused KV chunks can avoid
repeated prefill work, with TTFT and GPU-cycle reduction as the relevant
benefits. These claims and the actively evolving multiprocess interface are why
Switchboard keeps LMCache benchmark-only in this phase.

Provenance is pinned to official `LMCache/LMCache` dev commit
`e8f938189d42875abf469f25a34765659e0f9c2d` from 2026-08-16:

- <https://github.com/LMCache/LMCache/commit/e8f938189d42875abf469f25a34765659e0f9c2d>
- <https://docs.lmcache.ai/developer_guide/integration.html>
- <https://docs.lmcache.ai/mp/quickstart.html>

The paired promotion gate compares native prefix caching against native prefix
caching plus LMCache. Each arm requires at least 100 measured requests. LMCache
must improve median TTFT by 15%, GPU prefill work by 10%, and cost per
successful task by 10%, while limiting task-success regression to 0.5
percentage points. Operational-complexity score must remain at most 50 and may
increase by at most 20 points. Passing produces only a benchmark-candidate
decision; live promotion is always false, and this phase installs nothing.
