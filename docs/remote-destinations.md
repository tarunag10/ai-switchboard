## App-Owned Remote Destinations

## Provider Traffic

## Change Control

## Local-Only Boundary

[
  {
    "Destination": "Tauri updater feeds",
    "Configuration": "`HEADROOM_UPDATER_ENDPOINTS`, `HEADROOM_UPDATER_STAGING_ENDPOINTS`, `HEADROOM_UPDATER_PUBLIC_KEY`",
    "Purpose": "Signed desktop update checks for release builds. Official feeds are expected to be GitHub Release `latest.json` endpoints for this repository unless a fork intentionally replaces them.",
    "Local-only behavior": "Do not configure updater endpoints for local-only builds."
  },
  {
    "Destination": "Sentry diagnostics",
    "Configuration": "`HEADROOM_SENTRY_DSN`, `VITE_SENTRY_DSN`",
    "Purpose": "Optional backend/frontend crash and bootstrap failure diagnostics.",
    "Local-only behavior": "Disabled when local-only or remote telemetry is disabled."
  },
  {
    "Destination": "Aptabase analytics",
    "Configuration": "`HEADROOM_APTABASE_APP_KEY`",
    "Purpose": "Optional backend analytics events sent to `https://eu.aptabase.com/api/v0/events` or `https://us.aptabase.com/api/v0/events` depending on the configured key region.",
    "Local-only behavior": "Disabled when local-only."
  },
  {
    "Destination": "Microsoft Clarity",
    "Configuration": "`VITE_CLARITY_PROJECT_ID`",
    "Purpose": "Optional frontend product analytics when a build opts in.",
    "Local-only behavior": "Leave empty for local-only builds."
  },
  {
    "Destination": "ChatGPT Codex usage endpoint",
    "Configuration": "User-enabled usage/pricing helper in `src-tauri/src/pricing.rs`.",
    "Purpose": "Reads provider-owned Codex usage from `https://chatgpt.com/backend-api/wham/usage`; this is an unofficial upstream provider surface, not app-owned telemetry.",
    "Local-only behavior": "Disable usage/pricing helpers for local-only builds."
  },
  {
    "Destination": "Support and external links",
    "Configuration": "Bundled app/docs copy",
    "Purpose": "Optional user-initiated browser actions for this repository GitHub Issues, legal, or release pages.",
    "Local-only behavior": "Hide, remove, or keep as explicit user-initiated links only."
  },
  {
    "Destination": "`https://files.pythonhosted.org/...headroom_ai...whl`",
    "Configuration": "Pinned `headroom-ai` wheel install.",
    "Purpose": "Version and SHA-256 are pinned in `src-tauri/src/tool_manager.rs`.",
    "Local-only behavior": "Do not install managed runtime packages in local-only/no-network validation."
  },
  {
    "Destination": "`https://github.com/gglucass/headroom-desktop/releases/expanded_assets/vendor-wheels-v1`",
    "Configuration": "Vendor wheel index for packages that do not ship suitable macOS wheels.",
    "Purpose": "Pinned by the desktop release.",
    "Local-only behavior": "Do not fetch vendor wheels in local-only/no-network validation."
  },
  {
    "Destination": "PyPI/simple index URLs",
    "Configuration": "Python dependency resolution from bundled lock files.",
    "Purpose": "Lock files are bundled under `src-tauri/python/`.",
    "Local-only behavior": "Use already-bundled/runtime-local packages only."
  },
  {
    "Destination": "GitHub release/project URLs for RTK, MarkItDown, Ponytail, and Caveman",
    "Configuration": "User-visible source/provenance and managed install surfaces.",
    "Purpose": "Keep versions pinned or explicit before enabling automatic installation.",
    "Local-only behavior": "Keep as documentation/provenance links unless the user explicitly installs add-ons."
  }
]
