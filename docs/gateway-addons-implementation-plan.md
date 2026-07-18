# Gateway Add-ons Implementation Plan

Status ledger: see [plan-status-ledger.md](plan-status-ledger.md) for the
current created/updated/left checklist.

The broader execution order for text compression, repository-pack compression,
semantic caching, and pxpipe-style visual compression is defined in
[Token Optimization Add-ons Implementation Plan](token-optimization-addons-implementation-plan.md).
This document remains the detailed lifecycle plan for gateway, cache, trace,
and enterprise add-ons.

## Current local implementation status (2026-07-13)

The credential-free local slice is implemented. `src/lib/gatewayProfiles.ts`
now carries an explicit seven-stage lifecycle contract (detect, preview,
backup, apply, verify, rollback, and Off cleanup) for every profile. Governance
validation rejects missing stages, unsafe ordering, missing evidence, or a
profile that claims managed automation without complete lifecycle evidence.
The Add-ons card renders the lifecycle gate summary, local intent receipts,
redacted environment readiness, and the opt-in loopback LiteLLM preflight.

LiteLLM, Langfuse, Cloudflare AI Gateway, and Kong remain guided/gated and do
not install services, write provider configuration, contact remote gateways,
or store secrets. Live cache, trace, passthrough, enterprise health, and
credential verification still require user-owned infrastructure and are not
represented as completed by local evidence.

This plan adds optional gateway and observability layers around Mac AI Switchboard without replacing Headroom. The app remains local-first by default: any remote gateway or trace export must be explicit, reversible, and clearly labelled.

## Goals

- Keep Headroom and RTK as the default local savings path.
- Add a shared gateway profile model for local cache, observability, and remote gateway integrations.
- Add LiteLLM semantic cache as the first optional local cache add-on.
- Add Langfuse as an opt-in observability export target.
- Add Cloudflare AI Gateway as an opt-in remote gateway profile.
- Treat Kong and similar enterprise gateways as guided integrations until their full reversible lifecycle is proven.
- Extend Doctor, Rollback Center, and release gates so every enabled integration can be verified and cleaned up.

## Non-Goals

- Do not silently reroute provider traffic through a third party.
- Do not make cloud gateway profiles default.
- Do not store API keys, trace secrets, or gateway tokens in repo files.
- Do not promote native config writes until preview, backup, apply, verify, rollback, and Off cleanup are covered.
- Do not mix measured savings with estimated savings without an explicit confidence label.

## Current Baseline

Switchboard already has managed routing for Claude Code, Codex, Gemini CLI, OpenCode, Windsurf, Zed AI, Goose, and Grok / xAI CLI. Goose's native writes are limited to documented OpenAI/Anthropic endpoint fields alongside its read-only Repo Memory MCP bridge; Grok's native write is limited to `endpoints.models_base_url`. Cursor, Aider, Continue, Qwen Code, and Amazon Q Developer CLI retain sidecar/readiness coverage while unsupported native/provider writes remain gated.

The existing roadmap already centers reversible routing, Doctor evidence, rollback safety, Repo Intelligence, durable savings attribution, and local-first disclosure. Gateway add-ons should plug into those surfaces rather than create a parallel settings system.

## Phase 1: Gateway Profile Model

Add a shared model for optional add-ons that can affect routing, cache behavior, traces, or gateway exports.

Suggested profile ids:

```ts
type GatewayProfileId =
  | "headroom-local"
  | "rtk-shell-output"
  | "litellm-local-cache"
  | "langfuse-export"
  | "cloudflare-ai-gateway"
  | "kong-enterprise-gateway";
```

Each profile should declare:

- display name
- category: local savings, local cache, observability, remote gateway, enterprise gateway
- local or remote traffic boundary
- whether it can see prompts and outputs
- whether it can modify provider routing
- whether it needs secrets
- supported clients
- managed status: managed, guided, detected, gated, unsupported
- Doctor checks
- rollback behavior
- Off-mode cleanup behavior
- savings evidence source: measured, estimated, external, or none

Implementation notes:

- Put the canonical profile registry in a shared TypeScript/Rust-compatible shape if practical.
- Mirror only the fields needed by the Tauri backend; keep UI-only copy out of backend models.
- Use existing connector status language from `docs/connectors.md`.
- Include remote-disclosure text in the profile, not scattered across components.

Acceptance criteria:

- Settings can render profile cards from the registry.
- Doctor can list enabled profiles and report basic status.
- Governance checks fail if a remote profile lacks disclosure text.
- Tests cover local, remote, observability-only, and gated profiles.

## Phase 2: RTK Update And Session Check

Add a small Doctor slice before larger gateway work.

Doctor should report:

- bundled RTK path
- whether RTK exists and is executable
- version or build identity when available
- whether this shell/session appears to be using RTK instructions
- whether the active repo has Headroom RTK guidance in `AGENTS.md`
- whether an update appears available, if an upstream version can be checked safely

User-facing caveat:

RTK compresses shell command output. It does not automatically compress every built-in agent file-read or search tool, so handoffs should still prefer explicit `rtk read`, `rtk grep`, `rtk find`, and `rtk test` commands when appropriate.

Acceptance criteria:

- Missing RTK produces a repairable Doctor finding.
- Stale or unknown version produces a non-fatal warning.
- Paths with spaces are handled correctly.
- Off mode does not remove user-owned RTK binaries.
- Tests cover missing binary, bad permissions, valid binary, and unknown version.

## Phase 3: LiteLLM Semantic Cache - Guided Local Add-on

Start with guided setup and verification. Do not mutate provider configs yet.

User value:

- Reuse responses for semantically similar repeated prompts.
- Reduce repeated upstream calls in agent loops.
- Add a new savings source beside Headroom and RTK.

Initial scope:

- Detect a local LiteLLM proxy.
- Detect configured cache backend when possible.
- Provide copyable environment/config snippets.
- Run a safe health check.
- Record cache status in Doctor.
- Add estimated savings attribution only when enough evidence exists.

UI:

- Card title: Semantic Cache
- States: Off, Guided, Running, Cache hits detected, Needs repair
- Actions: Configure, Verify, Copy env, Disable guide
- Disclosure: similar prompts may reuse previous responses; disable for highly sensitive or rapidly changing work.

Doctor checks:

- local proxy port reachable
- cache backend configured
- upstream provider reachable through LiteLLM
- no repo-local secrets detected in generated snippets
- Off-mode instructions available

Acceptance criteria:

- Guided profile can be enabled without writing native provider configs.
- Doctor reports missing proxy and missing cache backend separately.
- Savings ledger can record `semantic_cache_hit` as estimated or external.
- Governance check enforces local/remote and prompt-visibility disclosure.

## Phase 4: LiteLLM Managed Lifecycle

Promote LiteLLM only after the guided version is stable.

Managed lifecycle:

- preview target files and env changes
- create sibling backup or app-owned backup dossier
- apply only Switchboard-owned blocks
- verify local proxy and cache backend
- add Rollback Center row
- restore backup or remove managed block
- clean up in Off mode

Design decision:

LiteLLM can sit beside Headroom or behind Headroom depending on provider compatibility. The first managed path should support one clearly documented topology before adding chains.

Acceptance criteria:

- Backend preview requires explicit confirmation before writes.
- Rollback Center restores or removes only Switchboard-owned changes.
- Doctor verifies active routing chain without relying only on saved settings.
- Tests cover apply, verify, rollback, Off cleanup, and unmanaged config boundaries.

## Phase 5: Langfuse Observability Export

Add Langfuse as trace export, not routing.

User value:

- Prove model calls, latency, token use, and failure modes.
- Correlate Headroom, RTK, LiteLLM, and provider events.
- Debug compression failures and routing mismatches.
- Support release-readiness evidence.

Initial scope:

- Disabled by default.
- Support self-hosted endpoint first.
- Store keys in app-owned secure storage.
- Send a test trace.
- Attach trace ids to savings ledger entries when available.

UI:

- Section: Observability
- Modes: Disabled, Self-hosted Langfuse
- Fields: endpoint, public key, secret key
- Actions: Send test trace, Disable export
- Disclosure: traces may include prompts, outputs, metadata, model names, and timing.

Doctor checks:

- endpoint reachable
- auth accepted
- test trace accepted
- export disabled means no traces are sent
- stored secret is not present in repo files

Acceptance criteria:

- Langfuse export is opt-in and disabled by default.
- Test trace works without changing provider routing.
- Doctor reports auth, network, and disabled states clearly.
- Savings ledger can carry trace correlation ids without requiring Langfuse.

## Phase 6: Cloudflare AI Gateway Profile

Add as guided remote gateway first.

User value:

- Centralized rate limits.
- Gateway analytics.
- Retries and fallback options.
- Optional gateway caching.

Trust boundary:

Requests route through Cloudflare before the upstream provider. The UI must label this as remote routing and explain that prompts and outputs may be visible to that gateway depending on configuration.

Initial scope:

- Guided/copyable config.
- Verify gateway endpoint.
- Verify provider passthrough with a harmless request where possible.
- No native config mutation until the full lifecycle is proven.

Doctor checks:

- gateway URL present
- endpoint reachable
- auth present without exposing token value
- provider passthrough succeeds or gives actionable failure
- Off-mode guidance exists

Acceptance criteria:

- Remote disclosure is visible before configuration.
- Config snippets redact secrets.
- No provider config writes are performed in the guided phase.
- Release gates classify Cloudflare as remote and opt-in.

## Phase 7: Kong And Enterprise Gateway Dossiers

Treat Kong and similar tools as enterprise guided integrations.

Scope:

- Documentation and detection only.
- Explain where Kong overlaps with Headroom, LiteLLM, and Cloudflare.
- Provide manual setup checklist.
- Add connector/readiness dossier if there is a known local config surface.

Do not bundle or manage Kong until:

- local/remote boundary is clear
- install footprint is acceptable
- rollback strategy is proven
- Doctor can verify health without enterprise-only assumptions

Acceptance criteria:

- Docs explain enterprise use cases and tradeoffs.
- Profile status is guided or gated, not managed.
- Governance checks prevent accidental promotion to managed.

## Phase 8: Unified Evidence And Release Gates

Extend release-readiness checks for gateway profiles.

Checks should verify:

- every profile declares local or remote boundary
- every profile declares prompt/output visibility
- every remote profile has explicit disclosure copy
- every managed profile has preview/apply/verify/rollback/Off cleanup evidence
- every secret-bearing profile stores secrets outside repo files
- every savings source has measured, estimated, external, or none
- every enabled profile has at least one Doctor check

Commands to extend or add coverage to:

- `npm run check:connectors`
- `npm run check:governance`
- `npm run smoke:preflight`
- `npm run release:ready -- --json`

Acceptance criteria:

- CI fails when a profile lacks required disclosure or lifecycle metadata.
- Release readiness reports gateway profile status.
- Doctor and Rollback Center evidence appear in release JSON for managed profiles.

## Recommended Slice Order

1. Add `GatewayProfile` registry, docs, and governance validation.
2. Add RTK update and active-session Doctor checks.
3. Add LiteLLM semantic cache as guided local add-on.
4. Promote LiteLLM to managed lifecycle after guided checks are stable.
5. Add Langfuse self-hosted observability export.
6. Add Cloudflare AI Gateway as guided remote profile.
7. Add Kong enterprise docs and gated profile.
8. Extend release-readiness evidence across all profiles.

## Done Definition

Each shipped integration must have:

- local or remote label
- prompt/output visibility disclosure
- no silent provider rerouting
- no repo-stored secrets
- Doctor verification
- rollback or explicit gated status
- Off-mode cleanup or explicit manual cleanup guide
- savings attribution caveat
- tests for happy path and failure path
- release-readiness evidence
