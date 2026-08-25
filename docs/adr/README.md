# Architecture Decision Records

These lightweight ADRs capture the Phase 0 architecture constraints. Accepted
records apply to new work. Changing one requires a superseding ADR that names
the old record and includes migration and rollback evidence.

| ADR | Decision | Status |
| --- | --- | --- |
| [0001](0001-stable-intercept-ownership.md) | AI Switchboard owns the stable local intercept | Accepted |
| [0002](0002-coding-client-adapter-lifecycle.md) | Managed coding clients share one lifecycle contract | Accepted |
| [0003](0003-headroom-first-optimization-engine.md) | Headroom is the first `OptimizationEngine` | Accepted |
| [0004](0004-inference-endpoint-boundary.md) | Serving destinations implement the `InferenceEndpoint` boundary | Accepted |
| [0005](0005-cache-taxonomy.md) | Response, prompt, KV, and semantic caches remain distinct | Accepted |
| [0006](0006-hybrid-mac-window-menu-bar-ux.md) | The Mac product uses a full window and menu-bar companion | Accepted |
| [0007](0007-observe-only-model-routing.md) | Model routing remains observe-only until benchmark promotion | Accepted |
| [0008](0008-response-cache-body-at-rest-protection.md) | Response cache bodies stay plaintext at rest behind disclosure until encryption has a threat driver | Accepted |
