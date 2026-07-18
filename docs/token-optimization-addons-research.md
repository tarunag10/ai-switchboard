# Token Optimization Add-ons Research

Research date: 2026-07-18

This note records the live research behind
[Token Optimization Add-ons Implementation Plan](token-optimization-addons-implementation-plan.md).
Repository claims are treated as hypotheses until Switchboard reproduces them
against its own fixtures.

## Executive decision

| Priority | Decision | Integration boundary |
|---|---|---|
| 1 | Evaluate leanctx first; keep raw LLMLingua-2 as a research fallback | Opt-in local Python/HTTP sidecar; Headroom remains the routing owner |
| 2 | Evaluate `thom-heinrich/chonkify` only for Repo Intelligence packs | Offline, pinned, provenance-preserving pack stage; not live proxy traffic |
| 3 | Use LiteLLM exact caching before semantic caching | Existing guided local gateway profile; semantic mode remains opt-in and bypasses unsafe agent requests |
| 4 | Keep pxpipe experimental and upstream-gated | A future Headroom `text_image` compressor; never a second proxy |
| Skip | Do not add small universal token proxies yet | Require sustained maintenance, security, compatibility, and reproducible evidence |

## 1. leanctx and LLMLingua-2

Sources:

- [leanctx repository](https://github.com/jia-gao/leanctx)
- [leanctx PyPI](https://pypi.org/project/leanctx/)
- [Microsoft LLMLingua](https://github.com/microsoft/LLMLingua)
- [LLMLingua-2 paper](https://aclanthology.org/2024.findings-acl.57/)

### Findings

- leanctx is the better integration candidate because it provides Anthropic,
  OpenAI, and Gemini wrappers plus a documented HTTP compression sidecar.
- Its Anthropic path is block-aware and has explicit passthrough/streaming
  behavior, but Switchboard must independently verify tool-call and cache
  semantics.
- Its optional Lingua model adds roughly 1.2 GB of first-use model weights;
  this is too heavy for silent installation and requires readiness, disk, and
  memory checks.
- leanctx is young/Alpha. Its public benchmark claims, including agent
  transcript reduction, are useful for fixture design but are not Switchboard
  measurements.
- Raw LLMLingua-2 is a research compressor rather than an Anthropic proxy. It
  requires the caller to isolate safe text, rebuild provider messages, and own
  structured-content protection.
- Both projects are MIT-licensed, but dependency and model licenses still
  require release review.

### Decision

Do not embed raw LLMLingua-2 in the Rust/Tauri gateway. First prototype leanctx
behind a managed local HTTP sidecar with:

- explicit install and model download consent;
- shadow mode;
- a narrow prose/log allowlist;
- byte-identical passthrough for code, JSON, errors, tool calls, tool results,
  identifiers, cache-controlled blocks, and multimodal content;
- timeout and fail-open behavior;
- Switchboard-owned before/after token receipts.

## 2. chonkify versus Chonkie

The plan’s intended project is [thom-heinrich/chonkify](https://github.com/thom-heinrich/chonkify),
not [feyninc/chonkie](https://github.com/feyninc/chonkie). Chonkie is a separate
RAG chunking library.

Sources:

- [chonkify repository](https://github.com/thom-heinrich/chonkify)
- [chonkify README](https://raw.githubusercontent.com/thom-heinrich/chonkify/main/README.md)
- [Chonkie repository](https://github.com/feyninc/chonkie)

### Findings

- chonkify is an extractive document compressor aimed at RAG and agent memory,
  with project-authored factual-recovery and token-reduction comparisons.
- The repository currently has a small community footprint and GitHub reports
  `NOASSERTION` for its license metadata; license compatibility must be
  confirmed before bundling or redistributing it.
- Its README advertises compiled Python 3.11 wheels for macOS arm64/x86_64,
  Windows, and Linux, but Switchboard must verify the exact supported matrix.
- It is a better conceptual fit for bounded repository packs than for live
  conversation rewriting.
- Pack provenance, byte/line spans, deterministic output, and no-network/no-
  model execution are not sufficient as an external guarantee and must be
  enforced by a Switchboard adapter.
- Chonkie is active and widely used, but its primary role is chunking rather
  than source-faithful compression; it should not replace chonkify in the
  current plan without a separate evaluation.

### Decision

Keep chonkify out of the core packer initially. Build an offline adapter only
if it emits and preserves:

- repository-relative path;
- byte and line offsets;
- source content hash;
- compressor/version/configuration;
- deterministic output hash;
- dependency/runtime versions;
- skipped-file and omitted-content reasons;
- no-network/no-model proof.

Until license and provenance gates pass, retain the existing deterministic
Repo Intelligence packer as the shipped path.

## 3. Semantic caching

Sources:

- [LiteLLM caching documentation](https://docs.litellm.ai/docs/completion/caching)
- [LiteLLM repository](https://github.com/BerriAI/litellm)
- [mimir repository](https://github.com/aqstack/mimir)
- [PromptCache repository](https://github.com/tase-nikol/promptcache)

### Findings

- LiteLLM has the broadest provider coverage and fits Switchboard’s existing
  guided gateway profile. It supports exact and semantic backends with local
  deployment options.
- mimir is attractive for a disposable local benchmark because it is a small
  Go proxy with local embedding support, but documented namespace,
  invalidation, streaming, and tool-call safety controls are incomplete.
- PromptCache has a clear semantic-cache concept but is too new and adapter-
  dependent for a production Switchboard dependency.
- Semantic cache hits are not token compression. They avoid upstream calls and
  must have a separate ledger category.
- Semantic replay is unsafe for streaming, tool/MCP calls, nondeterministic
  requests, high-temperature requests, sensitive data, and rapidly changing
  repository state.

### Decision

Implement in this order:

1. LiteLLM exact caching for deterministic, completed text requests.
2. LiteLLM semantic caching behind an opt-in local profile.
3. mimir as a disposable local comparison, not a production dependency.
4. Revisit PromptCache only after stronger maintenance and streaming/tool-call
   evidence.

Required cache key material: provider, model, account/workspace boundary,
system/tool schema version, request policy, and cache namespace version. Add
TTL, invalidation, namespace flush, hit reason, and a kill switch before any
semantic mode is promoted.

## 4. pxpipe and Headroom

Sources:

- [pxpipe repository](https://github.com/teamchong/pxpipe)
- [Headroom repository](https://github.com/headroomlabs-ai/headroom)
- [Switchboard pxpipe plan](pxpipe-headroom-integration-plan.md)

### Findings

- pxpipe’s relevant technique is rendering bulky text context into PNG image
  blocks; its README documents meaningful savings but also silent exact-string
  misreads that vary by model.
- Switchboard pins Headroom `0.27.0` as a managed wheel and routes through one
  local Headroom boundary.
- Current Headroom image compression handles image inputs; that is not the
  same feature as text-to-image context rendering.
- A supported upstream Headroom `text_image` capability is therefore required
  before Switchboard can safely enable pxpipe-style behavior.

### Decision

Keep pxpipe in shadow/design mode until Headroom provides:

- versioned capability/configuration;
- exact model allowlisting;
- image-token profitability gate;
- protected factsheet and native-text safety zone;
- fail-open fallback;
- visual savings and exact-recall telemetry.

## 5. Skip criteria for universal token proxies

Do not integrate a small universal proxy solely because its README claims a
large percentage reduction. Require all of the following:

- active maintenance and pinned releases;
- clear license and dependency provenance;
- provider/message-shape compatibility tests;
- streaming, tool-call, and cache-boundary tests;
- local-only mode and secret-handling review;
- timeout/fail-open behavior;
- reproducible benchmarks with task-quality results;
- rollback, Off cleanup, and a documented bypass.

## 6. Research-driven implementation changes

The implementation plan should be interpreted with these corrections:

- “Chonkify” means `thom-heinrich/chonkify`; “Chonkie” is a separate project.
- leanctx is a sidecar experiment, not a direct Rust dependency or a default
  Headroom transform.
- raw LLMLingua-2 remains a research/reference implementation.
- LiteLLM exact caching precedes semantic caching.
- chonkify requires license and source-provenance gates before distribution.
- pxpipe cannot ship in Switchboard until upstream Headroom support exists.
