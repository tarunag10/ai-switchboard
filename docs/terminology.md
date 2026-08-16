# Product Terminology

Status: canonical for new architecture documentation, code, UI copy, and
telemetry.

Use these terms consistently:

| Term | Meaning |
| --- | --- |
| **AI Switchboard** | The product and local-first control plane. **Switchboard** is acceptable shorthand after the full name is established. |
| **AI Switchboard for Mac** | The current macOS distribution. Legacy names may appear only when documenting compatibility paths, identifiers, or historical artifacts. |
| **Headroom** | The first optimization engine integrated with AI Switchboard. Headroom is not the product or the whole control plane. |
| **coding client / agent** | A developer-facing client that Switchboard detects or manages, such as Claude Code, Codex, or DeepSeek Harness. |
| **`CodingClientAdapter`** | The lifecycle boundary through which Switchboard detects, previews, configures, verifies, rolls back, and cleans up a coding-client integration. |
| **`OptimizationEngine`** | A request-optimization implementation selected behind Switchboard policy. Headroom is the first implementation. |
| **`InferenceEndpoint`** | A provider-hosted or self-hosted inference serving destination. It is distinct from a coding client and from an optimization engine. |
| **response cache** | Switchboard-owned exact replay of a response for an exact, safely namespaced request identity. |
| **prompt cache** | Provider-side prefix or prompt caching. Switchboard may preserve or optimize for it but does not own it. |
| **KV cache** | Key-value cache owned by an inference runtime during serving. |
| **semantic cache** | A possible future approximate-reuse subsystem using semantic representation and similarity. Do not use this term for exact replay. |

## Naming rules

- Do not describe AI Switchboard as a Headroom toggle or use **Headroom** as the
  application name.
- Use **response cache**, not **semantic cache**, for current exact replay
  behavior. Historical storage keys and migration notes may quote legacy
  identifiers when technically necessary.
- Use `InferenceEndpoint` for the serving destination. Do not use **provider**
  when the destination may also be a self-hosted runtime.
- Use stable nouns in telemetry names. Prefer nouns from this glossary plus a
  lifecycle action, such as `coding_client_adapter_verified` or
  `response_cache_hit`; do not encode a temporary UI label in an event name.
- Qualify macOS-only packaging or workflows as **AI Switchboard for Mac** while
  keeping **AI Switchboard** as the parent product.

The detailed decisions behind these boundaries are recorded in
[the ADR index](adr/README.md).
