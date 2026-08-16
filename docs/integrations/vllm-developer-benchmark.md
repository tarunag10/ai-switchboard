# vLLM developer benchmark adapter

Status: developer evidence adapter; not a desktop runtime dependency.

AI Switchboard can normalize JSON already collected by AIPerf or a vLLM-compatible runtime into schema version 1. The evidence record captures model/runtime/source-version identity, TTFT, optional ITL and TPOT, end-to-end latency, token throughput, optional prefix-cache hit rate, queue depth/time, and optional GPU utilization/memory.

The compatibility target is [NVIDIA AIPerf v0.12.0](https://github.com/ai-dynamo/aiperf/releases/tag/v0.12.0), released 2026-08-06. The official [profile-export documentation](https://docs.nvidia.com/aiperf/tutorials/metrics-analysis/working-with-profile-export-files) maps `metrics.time_to_first_token.value`, `metrics.request_latency.value`, `metrics.inter_token_latency.value`, and `metrics.output_token_throughput_per_user.value`; exported latency values use the accompanying `ms` unit. The importer also accepts explicit canonical `*_ms` fields for runtime-native evidence. AIPerf [server-metric reference](https://github.com/ai-dynamo/aiperf/blob/main/docs/server-metrics/server-metrics-reference.md) is a separate source: vLLM queue, KV/prefix-cache, throughput, and latency metrics are usable when supplied, while GPU metrics are optional. The normalized record stores the artifact's reported collector version; an unreported version remains `null`.

The adapter has strict boundaries:

- it requires an explicit developer-mode opt-in;
- it does not install or launch AIPerf, vLLM, Python, CUDA, or GPU tooling;
- it does not make endpoint or network requests;
- TTFT, end-to-end latency, and throughput are required;
- missing optional runtime metrics remain `null`, rather than being inferred;
- invalid negative measurements or out-of-range rates are rejected.

Deterministic input examples live in `benchmarks/fixtures/vllm-*-evidence.json`. They are synthetic evidence, not performance claims.
