# Privacy Notice

Mac AI Switchboard is a local-first developer tool for AI coding-tool routing, runtimes, shell-output compression, add-ons, and diagnostics.

## Local-First, Not Offline-Only

The app keeps routing state, mode state, diagnostics, RTK/Repo Intelligence metadata, Doctor evidence, managed-file receipts, and health checks on the local Mac by default. Local-first does not mean offline-only: Claude, OpenAI, Anthropic, and other model-provider traffic still goes to whichever provider-owned tool or endpoint the user configures.

## Local Data

The app may store savings history, health receipts, telemetry-style local counters, Headroom/RTK/Repo Intelligence pack metadata, logs, diagnostics paths, provider-routing diagnostics, and message-logging artifacts when those features are enabled.

Message logging and diagnostics are intended for debugging. They may include compressed-message paths, provider responses, and diagnostic state. Secret-like values are scrubbed defensively, including common API keys, GitHub tokens, auth headers, `.p8`, `.pem`, `.p12`, and similar filenames.

## Secrets

Provider API keys, signing tokens, and private credentials should not be pasted into issues, requests, logs, diagnostics, or screenshots. Provider-owned tools may expose provider-issued credentials to their own storage. Mac AI Switchboard should use macOS Keychain or provider-owned storage where appropriate.

## Remote Services

The free public build can be used without sign-in, checkout, pricing, telemetry, support, analytics, or update-network access. Local-only builds disable account, pricing, telemetry, support, analytics, and update-network surfaces.

Official signed updater feeds and optional Sentry, Aptabase, and Microsoft Clarity analytics are only used when configured for a build that opts into remote services.

Usage and pricing helpers may query provider-owned account usage endpoints when the user enables those features. Codex usage checks currently depend on the upstream ChatGPT usage endpoint at `https://chatgpt.com/backend-api/wham/usage`; treat it as unofficial provider surface, not app-owned telemetry. It should remain disabled in local-only builds and documented in remote-destination inventories.

## Repo Intelligence

Repo Intelligence is read-only by default. It indexes local repository metadata, estimates token counts, builds bounded context packs, and excludes secret-like paths and generated/vendor-heavy surfaces where configured. Sharing context packs is a user action.

## Children

Mac AI Switchboard is a developer tool and is not directed to children.

## Changes

This notice may evolve with the app. Material changes should be documented in release notes or product docs.
