# TensorRT-LLM external endpoint contract

Status: Phase 5 verified endpoint profile. Switchboard connects to an existing
`trtllm-serve` deployment and never installs TensorRT-LLM, builds engines,
loads models, allocates GPUs, starts processes, or manages a cluster.

## Pinned provenance

- Repository: `NVIDIA/TensorRT-LLM`
- Reviewed commit: `210397bedcbec4305722942b49ddcb17c1cce3c1`
- Commit date: 2026-08-16
- Release checked: `v1.2.1` (published 2026-04-20)
- Primary source: [`trtllm-serve` command documentation](https://nvidia.github.io/TensorRT-LLM/commands/trtllm-serve.html)

The reviewed server contract exposes `GET /health`, `GET /version`, and
OpenAI-compatible `GET /v1/models`, `/v1/completions`, and
`/v1/chat/completions`. The documentation also describes inflight batching,
parallelism, disaggregated serving, speculative decoding, and quantization.
Those are normalized as runtime support, not proof that a particular enrolled
endpoint has enabled them.

## Enrollment and verification

Direct TensorRT-LLM enrollment is limited to loopback and private-LAN hosts.
Public remote access must be placed behind the separately approved enterprise
gateway profile. Enrollment requires approval matching the exact normalized
base URL; the runtime remains externally owned and uses no stored credential.

Verification is bounded to:

1. `GET /health` returning 2xx;
2. `GET /v1/models` containing the configured model ID;
3. `GET /version` providing non-empty TensorRT-LLM identity/version evidence.

Because vLLM also uses `/version`, the probe parser must use the configured
runtime-kind hint only to choose the expected response schema. The hint is not
observed evidence; successful parsing plus the response is still required.

Quantization is optional operator-supplied metadata constrained to a short,
safe label. Diagnostics contain only normalized URL/host, location, model ID,
configured context, runtime kind, safe quantization label, external ownership,
and sanitized verification state. They never retain raw bodies, headers,
credentials, model paths, engine paths, or topology.

Disable the profile to reverse routing. The TensorRT-LLM server, engines,
models, accelerators, and deployment remain untouched.
