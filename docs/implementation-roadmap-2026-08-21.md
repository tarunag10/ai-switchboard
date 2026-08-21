# AI Switchboard implementation roadmap — 2026-08-21

## Product direction

AI Switchboard should be the local control plane that prepares coding-agent
sessions, manages reversible client wiring, and proves what was saved or
changed. The near-term product goal is a trustworthy path from “I have a repo
and an agent” to “this session is prepared, optimized, and explainable.”

The repository remains local-first: provider requests still go to the user's
configured services, while Switchboard state, reversible configuration edits,
context packs, and evidence remain on the Mac.

## Delivery status

| Phase | Scope | Status | Acceptance evidence |
| --- | --- | --- | --- |
| 1 | Release/product truth contract | Shipped | `docs/release-truth.json`, `npm run check:release-truth`, 1 node test |
| 2 | Session Ready overview path | Shipped | `SessionReadyCard` tests, Home integration tests, production build |
| 3 | Safe app-owned Headroom dashboard action | Shipped | 3 focused Rust tests; public-link SSRF policy unchanged |
| 4 | Connector lifecycle behavioral matrix | Shipped | `npm run check:connector-behavior` runs the static matrix plus the temporary-home Rust adapter lifecycle suite |
| 5 | Benchmark-backed proof loop | In progress | Four-variant compression evidence is now validated and explicitly observe-only; successful-task latency/rework evidence remains |
| 6 | One bounded endpoint/runtime expansion | Planned | Generic OpenAI-compatible endpoint first; explicit approval, health check, streaming/tools/cancellation tests, rollback fixtures |

## Phase 4 — Connector lifecycle behavioral matrix

Build on `connectors/manifest.json`, `connectors/lifecycle-fixtures.json`, and
the existing Rust adapter fixtures. The matrix must exercise real temporary
home directories and must not touch the user's home.

The repeatable gate is `npm run check:connector-behavior`. It combines the
manifest/fixture matrix with the Rust `client_adapters_tests` module, whose
temporary-home fixtures in `client_adapters::tests` exercise the following stages:

1. detect the installed or absent configuration;
2. produce a redacted preview;
3. create a sibling backup before mutation;
4. apply only the allowlisted Switchboard fields;
5. verify bytes/state after the write;
6. roll back from the exact receipt;
7. run Off cleanup without removing unmanaged content;
8. repeat apply and Off to prove idempotence; and
9. reject malformed, conflicting, or secret-bearing fixtures safely.

Managed, sidecar, Guided, and Gated must remain distinct. A connector cannot
be promoted because metadata says it is supported; its lifecycle evidence must
be present and current.

## Phase 5 — Benchmark-backed proof loop

Make the first-run value visible without fabricating savings. A session proof
should expose the selected repo pack, optimization layers, measured or
estimated status, evidence timestamp, caveat, and rollback/disable action.

Automatic routing remains observe-only until successful-task quality, latency,
rework, and economics meet explicit thresholds. RTK, Headroom, Repo
Intelligence, add-ons, response cache, provider prompt cache, and runtime KV
cache remain separate attribution layers.

## Phase 6 — Bounded expansion wedge

Prioritize one endpoint/runtime boundary, not a broad integration sweep. Start
with generic OpenAI-compatible endpoints. Runtime-specific vLLM or DeepSeek
Harness behavior can follow only after URL validation, local/remote
classification, credential-free health diagnostics, streaming, tools,
cancellation, failure mapping, and rollback fixtures are proven.

## Explicit non-goals until the proof loop is green

- automatic model or endpoint routing;
- more native provider writes;
- semantic-cache infrastructure or enterprise gateways;
- claims of public installed-app, reboot, notarization, or live deployment proof
  without fresh external evidence; and
- deleting or staging unrelated workspace artifacts.
