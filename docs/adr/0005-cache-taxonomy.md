# ADR-0005: Keep response, prompt, KV, and semantic caches distinct

- Status: Accepted
- Date: 2026-08-16

## Context

The existing Switchboard cache replays responses for exact request identities.
Calling it a semantic cache implies approximate matching that it does not
perform and obscures different safety and ownership boundaries. Providers may
also cache prompt prefixes, while inference runtimes maintain KV caches.

## Decision

Use **response cache** for Switchboard-owned exact replay, with hashed keys,
workspace/account/model/request-variant namespace isolation, explicit bypass
rules, and invalidation. Use **prompt cache** for provider-owned prefix or prompt
caching and **KV cache** for inference-runtime state. Reserve **semantic cache**
for a future approximate-reuse subsystem that has embeddings or another
semantic representation, a similarity threshold, isolation, stale-code and
task-safety rules, and benchmark evidence. New code, UI, telemetry, and docs
must not call exact replay a semantic cache.

## Alternatives

- Keep **semantic cache** as a broad marketing label for exact replay.
- Use one generic **cache** term for every layer.
- Disable exact replay until approximate semantic reuse exists.

## Consequences

Metrics and controls can attribute hits to the correct owner, and users are not
misled about approximate reuse. Historical storage keys or serialized
identifiers may require compatibility aliases until a tested migration exists.
A true semantic cache is a separate, gated feature rather than an upgrade hidden
behind the response-cache name.

## Reversal strategy

If the response cache is unsafe, disable it and clear only its Switchboard-owned
namespace without affecting provider prompt caches or runtime KV caches. Any
terminology or storage migration must retain read/rollback compatibility for
one release and document the legacy identifier explicitly.
