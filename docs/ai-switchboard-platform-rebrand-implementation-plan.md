# AI Switchboard Platform Rebrand Implementation Plan

Status: substantially complete for shipped copy, documentation, and compatibility slices; external release-proof gates remain
Owner: Switchboard
Scope: product naming, repo identity, cross-platform packaging, CLI visibility, documentation, migration safety, and release evidence

Detailed execution tracks: `docs/ai-switchboard-platform-rebrand-execution-tracks.md`

## Goal

Reposition the product from **Mac AI Switchboard** to **AI Switchboard**, with **Switchboard** as the short product name, so the project can grow beyond the macOS desktop app into Linux, Windows, server, and CLI workflows.

The rebrand must increase visibility without hiding upstream tooling. Headroom, RTK, Caveman, Ponytail, MarkItDown, and other integrated tools should remain accurately attributed where they are separate projects. The product value is the Switchboard layer: orchestration, installation, routing, optimization policy, token visibility, safety checks, and one-click workflows.

## Naming Decision

| Surface | Name |
| --- | --- |
| Parent product | AI Switchboard |
| Short name | Switchboard |
| macOS app | AI Switchboard for Mac |
| CLI command | `switchboard` |
| Repo/package slug | `ai-switchboard` where practical |
| Legacy references | Mac AI Switchboard, Headroom-derived paths, old bundle identifiers |

Do not rename technical storage, bundle, keychain, updater, or config identifiers until a migration exists and is tested.

## Non-Goals

- Do not claim ownership of third-party tools.
- Do not remove required license, attribution, or provenance notices.
- Do not break existing installed macOS users.
- Do not rename runtime paths blindly.
- Do not combine product rebrand work with unrelated feature implementation.

## End-to-End Goals

### Goal 1: Clear Brand Architecture

Create a durable brand model:

- AI Switchboard is the cross-platform product.
- Switchboard is the concise UI/CLI shorthand.
- AI Switchboard for Mac is the current desktop distribution.
- Headroom/RTK/Caveman/Ponytail/MarkItDown are integrated engines or adapters.
- User-facing copy emphasizes one-click optimization, model routing, token visibility, and session reliability.

Done when README, docs, app copy, website copy, release notes, and screenshots use the new hierarchy; attribution copy is explicit; and old Mac-only framing appears only in legacy/migration contexts.

### Goal 2: Safe Technical Identity Migration

Separate visible product naming from persistent technical identifiers.

Inventory before changing:

- Tauri product name and bundle identifier.
- macOS app bundle name.
- Application Support paths.
- LaunchAgent labels.
- keychain service names.
- updater endpoints and signing metadata.
- local database and receipt paths.
- GitHub repository URLs.
- website domain and download links.
- CLI binary/package names.

Done when a compatibility matrix states which identifiers are renamed now, which are aliased, and which remain legacy; existing installs continue to launch, repair, uninstall, and roll back; and the app can find legacy state intentionally.

### Goal 3: Cross-Platform Product Surfaces

Prepare AI Switchboard for non-macOS usage without pretending all native features already exist everywhere.

Platform tiers:

- Core proxy and optimization engine: macOS, Linux, Windows.
- CLI: macOS, Linux, Windows.
- Desktop shell: macOS first, then Windows/Linux when packaging is proven.
- Native repair/uninstall: platform-specific.
- Mac-only features: explicitly labeled when tied to LaunchAgents, keychain, app bundle, or macOS app state.

Done when docs and UI distinguish core, CLI, desktop, and macOS-only features; Linux and Windows roadmap docs exist with clear blockers; and platform feature flags prevent unavailable controls from appearing as broken.

### Goal 4: CLI-First Visibility

Make `switchboard` a first-class command surface.

Target commands:

- `switchboard status`
- `switchboard doctor`
- `switchboard proxy start`
- `switchboard proxy stop`
- `switchboard optimize --mode full|rtk-only|off`
- `switchboard session start`
- `switchboard xray`
- `switchboard cache report`
- `switchboard redundancy report`

Done when CLI docs exist for install, setup, health checks, and agent integration; the desktop app can point users to equivalent CLI commands; and CI validates CLI help text plus at least one smoke command.

### Goal 5: Website and Release Repositioning

Update public positioning from Mac utility to cross-platform AI workflow infrastructure.

Website structure:

- AI Switchboard overview.
- Download for Mac.
- CLI preview or install instructions.
- Supported integrations.
- Attribution and licenses.
- Privacy/local-first explanation.
- Roadmap for Linux and Windows.

Done when website hero and metadata use AI Switchboard; Mac download remains clear; legacy links redirect or explain the rename; and release notes include migration notes.

### Goal 6: Evidence and Rollback

Every slice must include proof and a rollback path.

Evidence types:

- local app smoke evidence.
- CLI smoke evidence.
- docs link check.
- workflow validation.
- screenshot refresh where visible UI changes.
- installer/update compatibility proof.
- uninstall/rollback proof for renamed paths.

Done when each slice has a commit, test evidence, and rollback notes; `docs/plan-status-ledger.md` tracks completion; and no user-facing surface is left half-renamed.

## Implementation Slices

| Slice | Purpose | Commit target |
| --- | --- | --- |
| 1 | Planning and audit | `Add AI Switchboard platform rebrand plan` |
| 2 | Public copy and documentation | `Rebrand public docs to AI Switchboard` |
| 3 | App copy and UI | `Update app copy for AI Switchboard` |
| 4 | Technical identifier compatibility | `Add legacy identity compatibility for AI Switchboard` |
| 5 | CLI brand and commands | `Make Switchboard CLI a first-class surface` |
| 6 | Cross-platform support matrix | `Document AI Switchboard platform support` |
| 7 | Website and download flow | `Reposition website as AI Switchboard` |
| 8 | Release evidence | `Record AI Switchboard rebrand release evidence` |

## File Size Guard

Keep new files small and split when needed:

- Planning docs should stay under 400 lines.
- UI components should stay under 300 lines unless already larger.
- New Rust modules should stay focused and avoid giant mixed-responsibility files.
- If a file crosses the threshold, split into helpers, fixtures, or docs subsections before committing.

## Risk Register

| Risk | Mitigation |
| --- | --- |
| Existing installs lose state | Keep legacy path discovery and migration tests |
| Users think Switchboard created upstream tools | Use explicit attribution copy |
| Mac-specific features look broken on Linux/Windows | Add platform tiers and feature flags |
| SEO becomes too generic with "Switchboard" only | Use AI Switchboard publicly, Switchboard as shorthand |
| Repo/package rename breaks automation | Stage repo/package rename separately after redirects and CI proof |
| Support docs become half-renamed | Gate with grep and docs review |

## Acceptance Criteria

The rebrand is complete when:

- User-visible product identity is AI Switchboard across app, docs, website, and release notes.
- The macOS distribution is clearly AI Switchboard for Mac.
- CLI docs and commands use Switchboard consistently.
- Third-party tools are attributed accurately.
- Legacy Mac AI Switchboard installs keep working.
- CI and local smoke evidence pass.
- `docs/plan-status-ledger.md` marks the rebrand complete with commit evidence.
