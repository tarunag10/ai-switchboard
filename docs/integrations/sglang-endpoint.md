# SGLang external endpoint contract

Status: Phase 3 external-runtime integration. Switchboard does not install,
start, discover, or administer SGLang.

## Pinned provenance

- Upstream repository: `sgl-project/sglang`
- Reviewed commit: `d3589a7251e4df6710e14ac55071585e80ae62c7`
- Commit date: 2026-08-16
- Latest release checked: `v0.5.17` (published 2026-08-08)
- Primary source reviewed:
  [`python/sglang/srt/entrypoints/http_server.py` at the pinned commit](https://github.com/sgl-project/sglang/blob/d3589a7251e4df6710e14ac55071585e80ae62c7/python/sglang/srt/entrypoints/http_server.py)
- Capability references: [server arguments](https://docs.sglang.ai/advanced_features/server_arguments.html),
  [speculative decoding](https://docs.sglang.ai/advanced_features/speculative_decoding.html),
  and [PD disaggregation](https://docs.sglang.ai/backend/pd_disaggregation.html)

At that revision, the official server source defines an OpenAI-compatible
`POST /v1/chat/completions` surface, `GET /v1/models`, `GET /health`, and
`GET /server_info`. The server-info response includes the SGLang package
`version`. Switchboard pins those facts as compatibility provenance; a live
response is retained separately as endpoint evidence and is not proof that a
server runs the reviewed source revision.

## Enrollment and verification

SGLang uses the same `InferenceEndpoint`, allowlist, selection, and
`RouteTarget` boundary as vLLM and generic OpenAI-compatible endpoints. It
does not require a coding-client adapter change.

Enrollment requires explicit approval of the exact normalized base URL.
Verification sends bounded GET probes only to:

1. `/health` — must return a 2xx status;
2. `/v1/models` — must report the configured model ID;
3. `/server_info` — must return a non-empty SGLang version and be identified
   by the probe adapter as SGLang.

Selection remains manual and only succeeds after verification. Failed
verification disables and deselects the endpoint. No response body, header,
credential, or request URL is persisted in diagnostics.

## Normalized capabilities

The registry exposes the same normalized fields for vLLM and SGLang:

- prefix cache;
- speculative decoding;
- continuous batching;
- disaggregated prefill/decode;
- quantization;
- parallelism;
- maximum context;
- tool calling.

Each field has one evidence state: `supported`, `unsupported`, `unknown`,
`configured`, or `observed`. `supported` is deliberately limited to a
documented runtime ability. It does not claim that a feature is enabled on a
particular endpoint. User-supplied maximum context is therefore `configured`,
while a future telemetry-backed measurement may be `observed`. Missing
endpoint evidence remains `unknown` rather than being inferred.

## Reversal

Disable the profile through the endpoint registry. This clears selection and
leaves coding-client configuration unchanged. Removing SGLang support later
only requires removing its managed profile variant and probe parser; the
client adapter and route-target contract remain unchanged.
