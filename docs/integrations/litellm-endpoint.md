# LiteLLM externally owned endpoint contract

Status: Phase 4 endpoint profile built on the existing guided LiteLLM
readiness boundary. LiteLLM remains externally installed, configured, started,
secured, and upgraded.

## Pinned provenance

- Repository: `BerriAI/litellm`
- Reviewed commit: `bc6e7df05b018eefe6c7293790ca3f4de38709ac`
- Commit date: 2026-08-16
- Release checked: `v1.97.0` (published 2026-08-16)
- Primary sources: [health routes](https://github.com/BerriAI/litellm/blob/bc6e7df05b018eefe6c7293790ca3f4de38709ac/litellm/proxy/health_endpoints/_health_endpoints.py)
  and [proxy model-list route](https://github.com/BerriAI/litellm/blob/bc6e7df05b018eefe6c7293790ca3f4de38709ac/litellm/proxy/proxy_server.py)

The reviewed source defines unauthenticated `GET /health/liveliness`, which
returns a stable live-worker response and becomes unavailable during graceful
shutdown. It also defines authenticated `GET /v1/models`.

## Ownership and connectivity

Loopback enrollment keeps connectivity local. LAN and public HTTPS endpoints
require a separate explicit remote-connectivity opt-in in addition to exact
URL approval. The opt-in is persisted as policy evidence, not inferred from a
successful request.

Switchboard never stores a LiteLLM key. The endpoint profile stores only the
environment-variable name `LITELLM_API_KEY`. A verifier may read that variable
at request time and attach it to the model-list request, but must never return,
log, serialize, or interpolate its value into an error.

Verification is bounded to:

1. `/health/liveliness` returning 2xx and LiteLLM identity evidence;
2. `/v1/models` containing the configured model ID.

No deep `/health` provider checks are issued because those may contact
downstream providers and expose provider routing metadata. Normalized runtime
capabilities remain unknown unless separately configured or observed: a
LiteLLM gateway does not prove the abilities of its downstream model/runtime.

## Secret-free diagnostics and reversal

Diagnostics expose endpoint host/base URL, location, model ID, configured
context, runtime kind, external ownership, connectivity opt-in, and sanitized
verification state. They do not contain authorization headers, key values, or
raw proxy responses.

Disable the profile to deselect it. Switchboard leaves the external LiteLLM
process, configuration, keys, cache, and downstream providers untouched.
