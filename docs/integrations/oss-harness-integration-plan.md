# OSS harness integration plan

Updated: 2026-08-23

This is the integration track for DeepSeek Harness, NVIDIA NeMo Switchyard,
and jcode. The app remains the owner of routing policy, credentials, release
trust, and user-approved configuration changes.

## Source-derived capabilities

| Source | Useful capability | Switchboard target | Current status |
|---|---|---|---|
| DeepSeek Harness | Plugin seams, typed session events, replay/fork boundaries, tool and approval pipelines | Read-only capability contracts and gated plugin adapters | Experimental adapter and Repo Intelligence context prototype shipped; stable upstream lifecycle unavailable |
| NVIDIA NeMo Switchyard | Protocol translation, strategy-based routing, stage/escalation/random routing, operational metrics | Optional interoperability profile and benchmark strategy fixtures | Evaluated as external optional tool; no embedded runtime dependency |
| jcode | Reconnectable sessions, provider profiles, adaptive context, session search, approvals | Session service contract, provider metadata contract, bounded context/replay harness | Reference evaluation added; implementation remains staged |

## Implementation phases

1. **Provenance and contracts** — pin source URLs/commits, record licenses,
   define content-free event, provider, route, approval, and replay schemas.
2. **Local replay harness** — replay redacted request/decision/response metadata
   without provider traffic; mark external-tool replay observational unless
   deterministic inputs and outputs are present.
3. **Routing strategy fixtures** — add deterministic random, stage, escalation,
   and fallback fixtures behind observe-only policy; reconcile every result with
   the existing model-routing evidence gate.
4. **Session event backbone** — derive UI/session summaries from bounded,
   redacted events; add fork-at-event and cancel/resume contracts before any
   semantic memory expansion.
5. **Provider/tool registries** — expose provider metadata and tool capability
   schemas without moving secrets or enabling new writes; approvals remain
   fail-closed.
6. **Optional external interoperability** — only after compatibility, rollback,
   attribution, and release evidence exists for a concrete Switchyard or dsh
   workflow.

## Safety rules

- Do not vendor unstable harness runtimes into the macOS app by default.
- Do not combine two independent routing decisions in one request path.
- Do not infer quality, cost, task success, or promotion from transport status.
- Do not persist prompts, tool outputs, authorization headers, or credentials in
  the replay/event harness.
- Do not enable automatic routing, semantic replay, remote execution, or native
  connector writes from fixtures alone.

## Current status

The deterministic strategy-fixture slice is now implemented in
`scripts/oss-harness-strategies.mjs`. Random, stage, escalation, and fallback
strategies produce bounded metadata-only observations with deterministic seeds,
health checks, fail-closed exhaustion, and `automaticPromotion: disabled`.
The fixtures are covered by `scripts/oss-harness-strategies.node-test.mjs` and
remain outside the live proxy route path.

The session-event backbone is implemented in `scripts/oss-session-events.mjs`.
It enforces bounded contiguous metadata events, redaction, attach/pause/resume/
cancel/complete lifecycle transitions, and deterministic fork-at-event IDs;
all outputs remain observe-only and local.

The provider/tool registry is implemented in `scripts/oss-provider-registry.mjs`.
It exposes only bounded provider labels, model families, context limits,
auth-source labels, and tool capabilities. Secrets are rejected, writes remain
disabled, and approval evaluation fails closed in metadata-only mode.
The typed Workbench capability projection is the sole frontend bridge for this
native registry. Addons and Workbench render the same metadata-only boundary
without importing provider credentials or runtimes; it cannot drive lifecycle
actions, installs, or provider requests.

## Remaining work and gate

The local replay, deterministic strategy fixtures, session-event ledger, and
provider/tool registry slices are complete. They consume redacted metadata
only, produce bounded deterministic summaries, and cannot promote automatic
routing. Their tests are included in `check:oss-harness-integrations`.

The remaining OSS work is optional external interoperability. It requires a
specific upstream workflow with pinned compatibility evidence, a documented
rollback path, attribution for provider-billed usage, and release-level
verification. Until those artifacts exist, no upstream runtime is vendored,
no network replay is enabled, and no provider/editor writes are promoted.
