# macOS Release and App Updates

Mac AI Switchboard is set up for outside-the-App-Store macOS distribution with:

- Tauri's official updater plugin
- signed updater artifacts
- user-confirmed install prompts
- Apple code signing and notarization

For the user-facing install path and first-run footprint, see [Installing Mac AI Switchboard](install.md).

## Build a signed DMG locally

If your Apple Developer access is ready on your Mac, the fastest local path is:

```bash
npm install
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export TAURI_SIGNING_PRIVATE_KEY="$(cat .secrets/tauri-updater/private.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your-updater-key-password"
export APPLE_API_ISSUER="your-app-store-connect-issuer-id"
export APPLE_API_KEY="your-app-store-connect-key-id"
export APPLE_API_KEY_PATH="$HOME/.private_keys/AuthKey_ABC123XYZ.p8"
export HEADROOM_UPDATER_PUBLIC_KEY="$(cat .secrets/tauri-updater/public.key)"
export HEADROOM_UPDATER_ENDPOINTS='["https://github.com/tarunag10/mac-ai-switchboard/releases/latest/download/latest.json"]'
npm run build:mac:dmg
```

This produces a signed `Mac-AI-Switchboard_<version>.dmg` in `src-tauri/target/release/bundle/dmg/`.
Run `npm run release:env` before publishing from a local Mac. It checks Node/npm, Rust/cargo/rustup, Xcode command-line tools, signing/notarization environment variables, and updater settings before the heavier release gate starts. Use `node scripts/check-release-env.mjs --json` or `npm --silent run release:env:json` when automation needs structured `ok`, `blockers`, and `warnings` output.
Run `npm run release:ready` when you want a sequential release-readiness check before packaging. It runs the branding guard, `release:report`, validates `dist/release-readiness-report.json`, and prints concrete next actions. The underlying `release:report` command writes `dist/release-readiness-report.md` and `dist/release-readiness-report.json` with environment blockers, warnings, Rust backend validation status, installed-app smoke status, and next steps. If Rust validation is blocked, the report includes the exact unblock sequence: check `rustup`, check `cargo`, add macOS Rust targets, then run `npm run fmt:desktop` and `npm run test:desktop`. Feature branches can also use the **Rust Tauri Validation** GitHub Actions workflow for backend formatting/tests without Apple signing secrets.

`npm run test:desktop` runs Rust tests serially because several backend tests intentionally exercise shared shell/env/config state. Keep that release gate serial unless those tests are isolated.

Release env checks reject copied template placeholders such as `REPLACE_WITH_*`, `your-*`, and `/absolute/path/...`. Copy `.env.example` into a private `.env.local`, replace every placeholder with a real local value, and keep `.env.local` out of git.
Run `npm run release:check` before publishing. The release gate runs release environment preflight, semantic color checks, a production frontend build, frontend coverage, Rust formatting, and desktop tests.
Run `npm run smoke:preflight` before handing a DMG to a tester; it confirms the installed-app smoke checklist covers Switchboard modes, degraded-mode Doctor guidance, managed connector automation gates, manual workflow, config creation plan, Gemini dry-run preview evidence, pause/resume, Repo Intelligence agent handoffs, connector readiness payload in agent handoffs, Savings calculator copyable summary, and Codex optimization.
Treat the build as blocked until `npm run release:ready -- --strict` is clear, the DMG is signed/notarized, `/Applications/Mac AI Switchboard.app/Contents/Info.plist` exists from the DMG install, the beta smoke checklist has been run on that installed app, and `npm run smoke:installed -- --confirm` has written `dist/installed-smoke-summary.md`. The installed smoke summary records the SHA-256 of `docs/beta-smoke-test.md`; rerun installed smoke whenever the checklist changes so release readiness does not accept stale evidence.

Stable GitHub releases upload `SHA256SUMS.txt` and `sbom-summary.json` after
the Tauri action publishes the DMG/updater assets. These integrity artifacts do
not replace Apple signing or notarization; they make release contents easier to
verify and audit.

For local unsigned/ad-hoc testing only, use `npm run build:mac:local-install`. It builds a local DMG, copies it to `dist/release-artifacts`, installs `/Applications/Mac AI Switchboard.app`, ad-hoc signs the installed app, then runs `npm run smoke:installed:local`. The local smoke command writes `dist/local-installed-smoke-summary.md` and JSON metadata for the bundle, checksum, local code signature, Gatekeeper status, and running process. When the installed app is running, the same local smoke also records whether the loopback app listener (`127.0.0.1:6767`) and Headroom engine proxy (`127.0.0.1:6768`) report healthy `/readyz` status. Do not use that local summary as public release evidence.

If you want a universal build, install both Rust macOS targets first and then run:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
TARGET=universal-apple-darwin npm run build:mac:dmg
```

The local helper script sets `CI=true` for Tauri's DMG bundler, validates the required secrets, and supports either:

- `APPLE_API_KEY_PATH` for a local App Store Connect private key file
- `APPLE_API_PRIVATE_KEY_P8` if you prefer storing the key contents directly in an environment variable
- `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID` if you want Apple ID notarization instead

## What the app expects

This build reads two compile-time environment variables:

- `HEADROOM_UPDATER_PUBLIC_KEY`
  The public key for verifying Tauri updater signatures.
- `HEADROOM_UPDATER_ENDPOINTS`
  A JSON array or comma-separated list of HTTPS update feed URLs.

Example:

```bash
export HEADROOM_UPDATER_PUBLIC_KEY="$(cat .secrets/tauri-updater/public.key)"
export HEADROOM_UPDATER_ENDPOINTS='["https://github.com/tarunag10/mac-ai-switchboard/releases/latest/download/latest.json"]'
```

These values are compiled into the release build. If they are missing, Headroom still runs, but update checks stay disabled for that build.

## Environment variables to set

Required for a signed local DMG in this repo:

- `APPLE_SIGNING_IDENTITY`
  Your Developer ID Application certificate name from Keychain Access, for example `Developer ID Application: Your Name (TEAMID)`.
- `TAURI_SIGNING_PRIVATE_KEY`
  The private updater signing key contents because this repo builds updater artifacts alongside the DMG.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  The password for that updater signing key.

Required for notarization, choose one mode:

- App Store Connect API mode:
  `APPLE_API_ISSUER`, `APPLE_API_KEY`, and either `APPLE_API_KEY_PATH` or `APPLE_API_PRIVATE_KEY_P8`
- Apple ID mode:
  `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`

Recommended for production builds of Headroom so auto-update stays enabled:

- `HEADROOM_UPDATER_PUBLIC_KEY`
  The public half of the Tauri updater signing keypair.
- `HEADROOM_UPDATER_ENDPOINTS`
  A JSON array or comma-separated list of HTTPS update feed URLs.

Optional, usually only needed outside your own machine:

- `APPLE_CERTIFICATE`
  Base64-encoded `.p12` signing certificate export. Useful for CI or a clean machine without the certificate already installed in your login keychain.
- `APPLE_CERTIFICATE_PASSWORD`
  Password for the exported `.p12` certificate.

## Repository configuration

The GitHub Actions workflow expects these repository settings:

- Repository variable:
  `HEADROOM_UPDATER_PUBLIC_KEY`
- Repository secrets:
  `TAURI_SIGNING_PRIVATE_KEY`
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  `APPLE_CERTIFICATE`
  `APPLE_CERTIFICATE_PASSWORD`
  `APPLE_SIGNING_IDENTITY`

For notarization, configure one of these two sets:

- App Store Connect API:
  `APPLE_API_ISSUER`
  `APPLE_API_KEY`
  `APPLE_API_PRIVATE_KEY_P8`
- Apple ID:
  `APPLE_ID`
  `APPLE_PASSWORD`
  `APPLE_TEAM_ID`

## One-time updater key setup

Generate a Tauri updater keypair once and keep the private key in CI secrets:

```bash
npm run tauri signer generate -- -w ~/.tauri/headroom-desktop.key
```

Store:

- the generated public key in `HEADROOM_UPDATER_PUBLIC_KEY` during release builds
- the generated private key in CI as `TAURI_SIGNING_PRIVATE_KEY`
- the private-key password in CI as `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## Release pipeline

For each mac release:

1. Build with `HEADROOM_UPDATER_PUBLIC_KEY` and `HEADROOM_UPDATER_ENDPOINTS` set.
2. Code-sign the app with your Apple Developer ID Application certificate.
3. Notarize the build with Apple.
4. Publish the signed updater artifacts and `latest.json`.
5. Create or update the GitHub Release that hosts those files.

The app is already configured with `"createUpdaterArtifacts": true`, so Tauri will emit updater-friendly release artifacts during bundling.

## Apple signing and notarization

Use a Developer ID flow, not Mac App Store packaging.

Tauri's macOS distribution docs support two notarization paths:

- App Store Connect API credentials:
  `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`
- Apple ID credentials:
  `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`

You also need the signing certificate material used by the macOS bundle build, typically:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`

## Recommended hosting

For a small app, the simplest setup is:

- GitHub Releases for DMG and updater artifacts
- a stable `latest.json` release asset URL

`latest.json` should follow Tauri's static updater format and include the macOS platform entry, the signed update bundle URL, and the bundle signature.

You can later move the updater feed to S3 or another CDN without changing app code, as long as the published endpoint URL stays valid and the signatures match the embedded public key.

## User experience in Headroom

Headroom does not auto-install updates silently.

Current behavior:

- checks for updates in the background after launch
- lets the user manually check from Settings
- prompts before download/install
- asks the user to restart after install completes
- production builds fall back to `https://github.com/tarunag10/mac-ai-switchboard/releases/latest/download/latest.json` when no explicit updater env vars are injected

## Recommended next step

Add a release workflow in CI that:

- builds `tauri build` for macOS
- injects the updater env vars above
- signs and notarizes the app
- uploads the updater artifacts plus `latest.json` to the release

Tauri's official GitHub release tooling can generate `latest.json` for you, which is the easiest way to keep the feed and artifacts aligned.

This repo now includes a workflow at `.github/workflows/release-macos.yml`.

It:

- runs on manual dispatch
- also runs automatically when a version bump to `package.json` / `src-tauri/tauri.conf.json` is pushed to `main`
- builds the Apple Silicon (`aarch64-apple-darwin`) release bundle
- signs and notarizes the app
- uploads updater artifacts and `latest.json` to the GitHub Release

## Release channels: stable and staging

There are two channels with separate GitHub Actions workflows:

| Channel | Branch | Workflow | Version format | Endpoint |
|---------|--------|----------|----------------|----------|
| Stable | `main` | `release-macos.yml` | `X.Y.Z` | `releases/latest/download/latest.json` |
| Staging | `staging` | `release-macos-staging.yml` | `X.Y.Z-rc.N` | `releases/download/staging-rolling/latest.json` |

### Branching model

- Feature work happens on `feature/*` branches.
- Feature branches merge into `staging` for release-candidate builds.
- `main` is branch-protected (no direct pushes). Stable promotion goes through a release PR from a `release/X.Y.Z` branch (cut from the verified `staging` tip, with the version bumped to plain `X.Y.Z`) into `main`. Merge with **"Create a merge commit"** — squash and rebase merges rewrite the staging SHAs and break the rc-ancestor check below. See the README for the full step-by-step.

### Staging workflow

On each push to `staging` that bumps the version to `X.Y.Z-rc.N`:

1. A versioned prerelease tag `vX.Y.Z-rc.N` is published with signed artifacts and `latest.json`.
2. The previous rolling `staging` release is deleted and recreated pointing at the new rc's artifacts. The staging endpoint URL stays stable.

The versioned tags give an auditable history of every rc; the rolling `staging` release is what the updater on the test machine actually polls.

### One binary, two channels

Both workflows bake **both** endpoints into every build via `HEADROOM_UPDATER_ENDPOINTS` (stable) and `HEADROOM_UPDATER_STAGING_ENDPOINTS` (staging). At runtime the app picks based on its own version: pre-release suffix (anything containing `-`, e.g. `0.2.44-rc.1`) → staging endpoint; plain `X.Y.Z` → stable endpoint. No separate build flavor required.

### Installing the staging build

Download the DMG attached to the rolling [`staging-rolling`](https://github.com/tarunag10/mac-ai-switchboard/releases/tag/staging-rolling) release on the clean test machine and install it once. Because its version has an `-rc.N` suffix, the app polls the staging endpoint from then on and self-updates as new rcs land.

### Promotion guard

The stable workflow refuses to run unless:

1. The version is plain `X.Y.Z` (no `-rc` suffix).
2. A `vX.Y.Z-rc.N` prerelease exists on GitHub whose tagged commit is an ancestor of the commit being released. This enforces that stable only ships code that was tested via the staging channel.

To bypass the guard for emergency hotfixes, include `[skip-rc-check]` in the **PR merge commit message** (the workflow reads the merge commit on `main`, not the bump commit on the release branch). Putting `[skip-rc-check]` in the PR title or first body line is the easiest way — GitHub copies it into the auto-generated merge commit.

### Final update-flow verification

After the stable workflow publishes `vX.Y.Z`, it re-points the rolling `staging` release at the stable artifacts. The staging test machine receives `X.Y.Z` as an update via the staging endpoint (since `X.Y.Z > X.Y.Z-rc.N` in semver). Once installed, its version is plain `X.Y.Z` and the app automatically switches to the stable endpoint for all future update checks.
