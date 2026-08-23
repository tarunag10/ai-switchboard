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

## Current next slice

The first local replay slice is `scripts/oss-harness-replay.mjs`. It consumes
redacted route metadata only, produces deterministic counts and latency
summaries, and remains separate from provider-billed quality evidence. It does
not issue network requests and its output cannot promote automatic routing.
