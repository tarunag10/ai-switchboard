# Enterprise gateway external endpoint contract

Status: Phase 5 optional enterprise profile. Switchboard connects to an
already-operated Envoy AI Gateway deployment; it does not install gateway
components, apply Kubernetes resources, configure providers, rotate keys, or
manage cluster lifecycle.

## Pinned provenance

- Repository: `envoyproxy/ai-gateway`
- Reviewed commit: `06381c5195178b349fa5b77648179775f0b1d839`
- Commit date: 2026-08-15
- Release checked: `v1.0.0` (published 2026-06-23)
- Primary source: [supported LLM endpoints](https://github.com/envoyproxy/ai-gateway/blob/06381c5195178b349fa5b77648179775f0b1d839/site/docs/capabilities/llm-integrations/supported-endpoints.md)

The reviewed source documents OpenAI-compatible APIs including
`GET /v1/models`. This profile intentionally proves only gateway reachability
and the configured model's presence; a gateway response cannot prove which
downstream runtime or deployment features served it.

## Deployment and enrollment contract

The operator supplies an existing OpenAI-compatible base URL and model ID.
Loopback is local; LAN and public HTTPS URLs additionally require explicit
remote-connectivity opt-in. Every endpoint also requires approval matching the
exact normalized base URL. Public remote URLs must use HTTPS.

Switchboard stores only the credential environment-variable name
`AI_SWITCHBOARD_ENTERPRISE_GATEWAY_TOKEN`. A probe may read that variable at
request time and send a bearer token to `GET /v1/models`, but must never
persist, return, log, or interpolate the value into an error.

Verification is bounded to `GET /v1/models` returning 2xx and containing the
configured model ID. No Gateway API, Kubernetes, provider, route, telemetry,
admin, or mutation endpoint is called. Runtime capabilities remain unknown;
configured maximum context remains labeled configured rather than observed.

## Diagnostics and reversal

Diagnostics expose only normalized URL/host, location, model ID, configured
context, external ownership, connectivity opt-in, gateway/runtime kind, and
sanitized verification state. Disable or remove the profile to reverse local
routing; the external gateway and cluster are untouched.
