# NVIDIA Dynamo external endpoint contract and selection record

Status: Phase 5 selected enterprise serving integration. Switchboard enrolls
an existing Dynamo frontend only. It does not install Dynamo, create a cluster,
deploy workers, select GPUs, configure discovery, or administer Kubernetes.

## Pinned provenance

- Selected repository: `ai-dynamo/dynamo`
- Reviewed commit: `4ae1af02db404c6268c4560df1071c0225f88b36`
- Commit date: 2026-08-16
- Release checked: `v1.4.0` (published 2026-08-15)
- Alternative evaluated: `llm-d/llm-d` at
  `cc4fbfd736b9795cc39df7a0e0f9273c97b7346b` (2026-08-14), release `v0.8.1`
- Primary sources: [Dynamo frontend](https://docs.nvidia.com/dynamo/latest/components/frontend),
  [frontend configuration](https://docs.nvidia.com/dynamo/latest/components/frontend/configuration-reference),
  [KV-aware routing](https://docs.nvidia.com/dynamo/latest/user-guides/kv-cache-aware-routing),
  [disaggregated serving](https://docs.nvidia.com/dynamo/latest/user-guides/disaggregated-serving),
  and [llm-d repository](https://github.com/llm-d/llm-d/tree/cc4fbfd736b9795cc39df7a0e0f9273c97b7346b)

## Why Dynamo is the one first-release choice

Dynamo is selected because this phase also introduces TensorRT-LLM and targets
NVIDIA/datacenter deployments. Its official documentation directly covers the
OpenAI frontend, KV-aware routing, disaggregated serving, and vLLM, SGLang, and
TensorRT-LLM backends. llm-d is a strong Kubernetes-native alternative, but
adding it now would duplicate an enterprise orchestration surface without
improving the endpoint-only Switchboard contract. It is not implemented in
this release.

## Concrete deployment proof boundary

The operator supplies an already-running Dynamo frontend base URL, a model ID,
and any backend/worker deployment. Switchboard requires exact normalized URL
approval. LAN and public HTTPS endpoints also require explicit remote
connectivity opt-in. It stores only `AI_SWITCHBOARD_DYNAMO_TOKEN`, the name of
an environment variable whose value may be used transiently for the models
probe and never enters registry state or diagnostics.

Verification performs only:

1. `GET /health` returning 2xx;
2. `GET /v1/models` returning 2xx and containing the configured model ID.

No worker, discovery, router, scheduler, KV-cache, Kubernetes, metrics, admin,
or mutation API is called. The profile records supported deployment-level
capabilities from the pinned Dynamo contract, while endpoint-specific
activation and backend-dependent features remain configured/unknown rather
than falsely observed.

Disabling the profile reverses Switchboard routing only. It does not stop or
change the Dynamo frontend, workers, models, cluster, or credentials.
