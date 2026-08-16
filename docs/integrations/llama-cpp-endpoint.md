# llama.cpp external endpoint contract

Status: Phase 4 verified local-runtime profile. Switchboard does not install,
start, discover, update, or administer llama.cpp or model files.

## Pinned provenance

- Repository: `ggml-org/llama.cpp`
- Reviewed commit: `4df29be4f4c3673f428170fda944a5b19f743bb8`
- Commit date: 2026-08-16
- Release checked: `b10453` (published 2026-08-16)
- Primary sources: [server documentation](https://github.com/ggml-org/llama.cpp/blob/4df29be4f4c3673f428170fda944a5b19f743bb8/tools/server/README.md)
  and [server route registration](https://github.com/ggml-org/llama.cpp/blob/4df29be4f4c3673f428170fda944a5b19f743bb8/tools/server/server.cpp)

The reviewed server defines public `GET /health`, OpenAI-compatible
`GET /v1/models`, and read-only `GET /props`. `/props` includes `build_info`,
context and slot settings, the model path, and whether speculative decoding is
configured. Switchboard retains only the sanitized runtime identifier/version
evidence required by the registry; it does not persist the raw properties
response or model path.

## Enrollment and verification

The profile accepts loopback and private-LAN endpoints only. Public remote
hosts are rejected. Enrollment still requires exact URL approval and selection
remains manual.

Verification is bounded to:

1. `/health` returning 2xx;
2. `/v1/models` containing the configured model ID;
3. `/props` providing non-empty llama.cpp `build_info` identity evidence.

Quantization is optional user-supplied metadata. It is constrained to a short
safe label such as `Q4_K_M`; paths, whitespace, shell syntax, and secret-like
values are rejected. Runtime support is not treated as endpoint activation.

Diagnostics expose only the normalized base URL/host, model ID, location
classification, configured context, verification state, runtime kind, and
safe quantization label. No response body, header, credential, or model path is
retained.

## Reversal

Disable the profile. This deselects the route without stopping llama.cpp,
changing its configuration, or touching its models.
