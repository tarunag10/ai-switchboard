# pxpipe-style context rendering in Headroom

Status: design-ready, Switchboard integration gated on an upstream Headroom
feature.

This is the pxpipe-specific workstream of the broader
[Token Optimization Add-ons Implementation Plan](token-optimization-addons-implementation-plan.md).

## Finding

Switchboard installs Headroom from a pinned wheel (`0.27.0`), so this checkout
does not contain the Headroom request transformer. The current upstream
Headroom project already has an image compressor for images supplied by the
user. That path is not the same as pxpipe: pxpipe renders eligible text
context into PNG image blocks before forwarding the request.

Switchboard must not add a second localhost proxy or set an undocumented
environment variable and claim that text-to-image compression is active.

## Reuse boundary

The implementation should be proposed upstream in Headroom and consumed by
Switchboard after a versioned release:

```text
client -> Switchboard-owned Headroom proxy
              |
              +-- existing text/code/log compressors
              +-- optional text-image context compressor
              +-- existing provider adapters and savings ledger
```

The pxpipe renderer should be reused only for page layout and PNG generation.
Headroom must own request eligibility, provider format conversion, protection
rules, cache policy, failure handling, and measurements.

## Required Headroom contract

Add a built-in, opt-in compressor (suggested name: `text_image`) with:

- `off` as the default; `shadow` mode that renders and measures but forwards
  the original request; `on` only for an explicit experimental profile.
- Exact model allowlisting. No model should receive image context merely
  because it advertises vision support.
- Profitability gating based on the resolved provider/model image-token cost.
- Eligibility limited initially to large, old `tool_result` and closed-history
  text blocks. Keep the system prompt, tools, current user turn, open tool
  calls, and recent turns native text.
- A recoverable adjacent factsheet containing protected identifiers and the
  original block metadata. Hashes, paths, secrets, JSON values, and other
  byte-sensitive spans must remain text.
- Fail-open behavior: renderer/model/dependency/size errors send the original
  text request and record the reason.
- Request-local metrics for original text tokens, image tokens, protected text
  tokens, eligible bytes, rendered pages, and fallback reason.
- No raw prompt logging by default. Debug message logging must retain the
  existing expiry/redaction controls.

The first upstream implementation should target Anthropic Messages only. It
should not attempt OpenAI Responses history rewriting until an equivalent
tool-call pairing and provider-token measurement contract exists.

## Switchboard integration after the Headroom release

1. Pin a Headroom version whose capability endpoint or startup receipt reports
   `text_image` support.
2. Add a Switchboard experimental profile rather than adding it to Full
   optimization.
3. Pass the documented Headroom configuration only when that profile is
   explicitly enabled; otherwise preserve the current startup environment
   byte-for-byte.
4. Display the local boundary, prompt visibility, lossiness warning, model
   scope, and exact-string caveat before enablement.
5. Add Doctor checks for capability, active mode, model allowlist, fallback
   count, and last measured savings.
6. Add an Off-mode cleanup check and a direct Headroom bypass/kill switch.
7. Attribute visual savings separately from Headroom text compression and RTK;
   never combine estimated and measured values.

## Acceptance tests

- Inactive profile leaves requests byte-identical.
- Shadow mode produces a render receipt but forwards original text.
- Protected identifiers remain native text.
- Recent/open history remains native text.
- Cacheable system/tool prefixes are not rewritten by default.
- Unsupported model, sparse prose, small block, renderer failure, and image
  token-cost regression all fail open.
- A Fable-quality fixture measures savings and exact-recall quality separately.
- Switchboard release evidence identifies the feature as experimental until
  the upstream benchmark and rollback gates pass.

## Current Switchboard action

No runtime code is enabled by this document. The existing Headroom proxy remains
the sole routing owner. The next implementation step is an upstream Headroom
change (or a local Headroom source checkout) that exposes the contract above;
then this repository can add the version pin, profile, Doctor evidence, and
tests without creating a competing proxy.
