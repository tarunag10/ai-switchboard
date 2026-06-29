# Threat Model

Mac AI Switchboard is local-first, but local-first is not the same as
zero-risk. This document records the current trust boundaries for managed
routing and repo-context features.

## Local Proxy

The intercept proxy binds to `127.0.0.1:6767` and forwards to the managed
Headroom backend on `127.0.0.1:<selected-port>`. It must never bind to
`0.0.0.0`.

Current guardrails:

- Loopback bind only: `127.0.0.1`, not external interfaces.
- Request-shape validation rejects browser `Origin` requests and non-loopback
  `Host` values to reduce DNS-rebinding exposure.
- Backend proxy fallback ports are selected only from local loopback ports.
- Doctor reports the proxy bind address and authentication status.

Current limitation:

- Managed Claude Code and Codex routing does not yet include a per-session
  bearer token because the managed clients do not reliably support custom
  headers for every required route. Treat localhost as local-process trust, not
  a security boundary.

Future hardening:

- Prefer a user-owned Unix domain socket where client support allows it.
- Add a per-session bearer token for clients or shims that can inject headers.
- Keep local HTTP as a compatibility bridge only when a socket/header path is
  unavailable.

## Raw Messages

Full message logging is off by default, auto-expires when enabled, is redacted
before display/export, and can be purged from the app. Do not enable it unless
debugging requires raw request/compressed-message visibility.
