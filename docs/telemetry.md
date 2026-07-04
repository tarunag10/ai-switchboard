# Telemetry and Message Logging

Mac AI Switchboard is local-first. Telemetry-style app data should use bounded
metadata such as status, feature name, token counts, error categories, and
health state. Raw prompts, request bodies, compressed messages, provider
responses, source snippets, authorization headers, and secret-looking strings
must not be sent to analytics or Sentry.

## Full Message Logging

Full message logging is off by default. The local setting is:

```json
{
  "fullMessageLogging": false,
  "fullMessageLoggingExpiresAt": null,
  "messageLogRetentionHours": 24
}
```

When enabled, the setting must include an expiry. The backend treats expired
settings as disabled and starts the proxy without `--log-messages`.

## Redaction Boundary

Message dumps are redacted before display/export for:

- `sk-ant-...`
- `sk-proj-...`
- `ghp_...`
- `github_pat_...`
- `BEGIN PRIVATE KEY`
- `AWS_SECRET_ACCESS_KEY`
- `ANTHROPIC_API_KEY`
- `OPENAI_API_KEY`
- `Authorization: Bearer ...`
- `.p8`, `.pem`, and `.p12` snippets

Redaction should happen before a message payload reaches UI rendering,
clipboard export, analytics, or Sentry.

## Purge

Use the `purge_message_logs` command to remove persisted Activity feed facts
that may contain historical message payloads. After disabling full message
logging, restart the runtime so the proxy is relaunched without raw message
capture.
