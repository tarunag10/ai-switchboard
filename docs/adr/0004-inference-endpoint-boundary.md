# ADR-0004: Serving destinations use the InferenceEndpoint boundary

- Status: Accepted
- Date: 2026-08-16

## Context

Requests may terminate at a hosted provider or a self-hosted runtime. Coding
clients and optimization engines should not need destination-specific branches,
and a URL alone cannot express health, models, streaming, tools, authentication,
or safe routing capabilities.

## Decision

Represent each provider-hosted or self-hosted serving destination as an
`InferenceEndpoint`. Begin with the current remote provider and a generic
OpenAI-compatible endpoint. The boundary owns endpoint identity, validated URL
and protocol, capability discovery or declaration, health, model availability,
streaming and tool support, credential references without secret exposure, and
failure classification. Runtime-specific profiles extend this boundary rather
than changing coding-client adapters.

## Alternatives

- Pass untyped base URLs through the system.
- Add a separate routing path for every inference runtime.
- Put endpoint behavior in each `CodingClientAdapter` or `OptimizationEngine`.

## Consequences

Adding an endpoint does not require edits to every client adapter. Endpoint
capabilities become explicit and testable, but adapters and profiles must avoid
claiming unsupported protocol features. URL validation, remote-probe consent,
and secret handling are mandatory at the boundary.

## Reversal strategy

Keep the current provider representation as the compatibility implementation.
An endpoint profile can be disabled or removed without changing the stable
intercept or client configuration. Restore manual current-provider selection if
generic capability negotiation proves unreliable.
