# Privacy Notice

Mac AI Switchboard is a local-first desktop utility for managing AI coding-tool
routing, local helper runtimes, shell-output compression, add-ons, and related
diagnostics on your Mac.

This notice describes the public repository's intended privacy model. It is not
legal advice and may need review before public distribution in a specific
jurisdiction.

## Local-First, Not Offline-Only

Mac AI Switchboard is local-first. App state, routing mode, local diagnostics,
reversible client setup evidence, RTK settings, add-on state, and Repo
Intelligence metadata are intended to live on your Mac.

The app is not offline-only. When you use Claude, OpenAI, Anthropic, or another
model provider, your coding client may still send requests to that provider
under your account and that provider's terms.

## Local Data the App May Read

Depending on enabled features, the app may read local files such as:

- App configuration and runtime state.
- Shell startup files used for RTK integration.
- Claude Code, Codex, MCP, LaunchAgent, and supported client configuration
  files.
- Headroom-managed runtime files, receipts, logs, and health evidence.
- Repository files selected for read-only Repo Intelligence indexing.
- Local diagnostics needed for Doctor repair workflows.

Managed edits should be reversible where possible and visible through app
diagnostics.

## Local Data the App May Store

The app may store local state such as:

- Selected switchboard mode.
- Selected savings profile.
- Runtime health and repair receipts.
- Local telemetry-style savings records.
- Headroom and RTK installation status.
- Repo Intelligence summaries and context packs.
- Logs and diagnostic evidence.

Diagnostics can include local paths, command metadata, token counts, runtime
status, and provider-routing evidence. Review diagnostics before sharing them.

## Full Message Logging

Full message logging is off by default because request and compressed-message
payloads can contain prompts, source code, local paths, provider responses, and
secrets. When enabled for debugging, it must have an expiry and should be kept
short. The app stores the setting in local config as `fullMessageLogging`,
`fullMessageLoggingExpiresAt`, and `messageLogRetentionHours`.

Displayed and exported message dumps are redacted for common secret patterns
such as provider keys, GitHub tokens, bearer headers, private-key markers, and
`.p8`, `.pem`, or `.p12` filenames. Redaction is a last line of defense, not a
guarantee. Use the message-log purge action before sharing diagnostics.

## Secrets

Provider API keys, Apple signing credentials, updater private keys, and personal
tokens should not be pasted into issues, pull requests, logs, exported
diagnostics, or screenshots.

Secrets should remain in macOS Keychain or provider-owned tools when possible.
If a secret is exposed, rotate it immediately through the provider that issued
it.

## Remote Services

The free public app is intended to run without sign-in, checkout, pricing APIs,
or required hosted services.

Local-only mode should avoid account, pricing, telemetry, support, analytics,
and update-network calls unless a remote feature is explicitly enabled by the
operator of a fork or build.

Optional remote destinations may include:

- Model-provider APIs used by your coding clients.
- Tauri update feeds for official signed releases.
- Sentry diagnostics if configured.
- Microsoft Clarity analytics if configured.
- Aptabase analytics if configured.
- Support/contact links if configured.

Official public builds should disclose which destinations are enabled.

## Repo Intelligence

Repo Intelligence is intended to be local and read-only by default. It may index
selected repository files, estimate tokens, classify paths, and generate context
packs. Generated packs may contain source excerpts or metadata from the selected
repository, so review them before sharing.

## Children

Mac AI Switchboard is a developer tool and is not intended for children.

## Changes

Privacy behavior can change as the app evolves. Material privacy changes should
be documented in this file, release notes, or both.
