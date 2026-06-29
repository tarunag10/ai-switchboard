# Remote Destination Registry

Mac AI Switchboard is local-first, not offline-only. This registry lists the
remote destinations that are intentionally allowed when a build or user action
enables remote services. Local-only mode should avoid account, pricing,
telemetry, support, analytics, and update-network calls unless an operator of a
fork or build explicitly enables them.

Mac AI Switchboard does not include a remote account, billing, checkout, or paid
pricing API.

This file is a release gate. Add or update a row before adding a new app-owned
remote destination.

## Local-Only Boundary

Local-only mode is enabled when `HEADROOM_LOCAL_ONLY=1` or
`VITE_HEADROOM_LOCAL_ONLY=1` is set. The frontend also treats remote services as
disabled unless `VITE_HEADROOM_REMOTE_SERVICES=1` is set. Remote telemetry stays
off unless both remote services are enabled and `VITE_HEADROOM_REMOTE_TELEMETRY`
is truthy.

Local loopback URLs such as `http://127.0.0.1:6767` and the selected internal
backend port (`http://127.0.0.1:6768` or fallback `6769..=6790`) are local
runtime endpoints, not remote destinations.

## App-Owned Remote Destinations

| Destination                | Configuration                                                                                     | Purpose                                                                                                                                                                                  | Local-only behavior                                          |
| -------------------------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Tauri updater feeds        | `HEADROOM_UPDATER_ENDPOINTS`, `HEADROOM_UPDATER_STAGING_ENDPOINTS`, `HEADROOM_UPDATER_PUBLIC_KEY` | Signed desktop update checks for release builds. Official feeds are expected to be GitHub Release `latest.json` endpoints for this repository unless a fork intentionally replaces them. | Do not configure updater endpoints for local-only builds.    |
| Sentry diagnostics         | `HEADROOM_SENTRY_DSN`, `VITE_SENTRY_DSN`                                                          | Optional backend/frontend crash and bootstrap failure diagnostics.                                                                                                                       | Disabled when local-only or remote telemetry is disabled.    |
| Aptabase analytics         | `HEADROOM_APTABASE_APP_KEY`                                                                       | Optional backend analytics events sent to `https://eu.aptabase.com/api/v0/events` or `https://us.aptabase.com/api/v0/events` depending on the configured key region.                     | Disabled when local-only.                                    |
| Microsoft Clarity          | `VITE_CLARITY_PROJECT_ID`                                                                         | Optional frontend product analytics when a build opts in.                                                                                                                                | Leave empty for local-only builds.                           |
| Support and external links | Bundled app/docs copy                                                                             | Optional user-initiated browser actions for this repository's GitHub Issues, legal, or release pages.                                                                                    | Hide, remove, or keep as explicit user-initiated links only. |

## Tool And Dependency Downloads

Managed helper installation can fetch pinned release artifacts when a user asks
the app to install or repair tools:

| Destination                                                                              | Purpose                                                                 | Controls                                                                 |
| ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `https://files.pythonhosted.org/...headroom_ai...whl`                                    | Pinned `headroom-ai` wheel install.                                     | Version and SHA-256 are pinned in `src-tauri/src/tool_manager.rs`.       |
| `https://github.com/gglucass/headroom-desktop/releases/expanded_assets/vendor-wheels-v1` | Vendor wheel index for packages that do not ship suitable macOS wheels. | Pinned by the desktop release.                                           |
| PyPI/simple index URLs                                                                   | Python dependency resolution from the bundled lock files.               | Lock files are bundled under `src-tauri/python/`.                        |
| GitHub release/project URLs for RTK, MarkItDown, Ponytail, and Caveman                   | User-visible source/provenance and managed install surfaces.            | Keep versions pinned or explicit before enabling automatic installation. |

## Provider Traffic

Provider traffic is not an app-owned analytics or account destination. When
Headroom engine routing or native client routing is enabled, the user's coding
tools may contact providers such as Anthropic, OpenAI, Gemini, xAI, Amazon, or
other configured model endpoints under the user's own accounts and the
provider's terms.

The app may read provider-auth evidence for local diagnostics only where that
flow already exists and is explicitly enabled. Do not add silent provider calls
in local-only mode.

## Change Control

Before adding a new app-owned remote URL:

- Add the destination, configuration, purpose, and local-only behavior here.
- Add a local-only guard or explain why the call is strictly user-initiated.
- Update privacy, release, and smoke-test docs if the destination affects users.
- Add a test or release gate that fails when the destination is missing from this
  registry.
