# DeepSeek Harness plugin maturity audit

Audit date: 2026-08-16. Decision: **Experimental / Developer Preview**.
Promotion is blocked and the existing adapter remains pinned to
`0.1.0-rc.5` at `47f943859bef60e4160492346772ded9b24f765a`.

## Current official upstream evidence

- The audited `master` head is still the adapter's pinned commit,
  `47f943859bef60e4160492346772ded9b24f765a`.
- [`README.md`](https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/README.md)
  still labels Harness a developer preview, says it is iterating rapidly, and
  warns that compatibility-breaking changes will occur.
- [`apps/cli/package.json`](https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/apps/cli/package.json)
  remains `0.1.0-rc.5`.
- The official repository exposes no GitHub releases or tags as of the audit.
- The official [architecture](https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/docs/architecture.md),
  [agent lifecycle](https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/docs/agent-lifecycle.md),
  [tool pipeline](https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/docs/tool-execution-pipeline.md),
  [system-prompt subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/docs/subsystems/system-prompt.md),
  and [token-meter subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/docs/subsystems/token-meter.md)
  describe useful internal seams. Because the whole release is explicitly
  compatibility-unstable, their existence is source evidence rather than a
  stable third-party compatibility promise.

## Required promotion evidence and current gaps

| Surface | Current evidence | Promotion gap |
|---|---|---|
| Repo Intelligence injection | `agent.inject()` is documented; Switchboard has an internal read-only `before_agent_run` payload prototype. | No stable/versioned upstream injection lifecycle, upstream third-party fixture, or real-dsh Switchboard lifecycle/rollback fixture. |
| Request metadata | `agent/request` is documented as a live request interception point. | No stable metadata mutation schema, ownership/redaction contract, upstream fixture, or Switchboard end-to-end proof. |
| Tool-result optimization | `tools/*` events and an internal tool-result pruner exist. | No stable external optimizer contract defining ordering, truncation ownership, replay, failure fallback, and content-free proof. |
| Prompt-segment classification | Upstream assembles prompt sections and tool schemas. | No stable typed classification vocabulary or compatibility fixture that lets an external optimizer classify without altering model-visible replay. |
| Switchboard route decision | Request interception and replaceable LLM adapters exist; the current Switchboard adapter only manages the pinned `baseURL` patch. | No stable per-request route-decision schema covering selected target, fallback, refusal, cancellation, replay, and secret-free attribution. |
| Savings evidence | Upstream has token-meter projections; the local context prototype records estimates. | No stable end-to-end schema attributing before/after tokens to a specific Switchboard transformation and route without recording prompt/tool content. |

## Fail-closed promotion contract

Every surface must have all of the following at one newly reviewed upstream
commit: a documented seam, stable versioned contract, upstream third-party
compatibility fixture, real-dsh Switchboard end-to-end fixture, and
content-free evidence. Upstream must also leave developer preview, withdraw the
breaking-change warning, publish a stable release/tag, and publish a plugin
compatibility policy.

A changed dsh version or commit is unreviewed and therefore remains
experimental. Even complete machine-readable evidence only opens a separate
manual promotion review; it never changes adapter maturity automatically.

The audit performs no dsh configuration writes, imports no dsh dependency,
reads no credentials or prompts, and does not modify the core adapter.
