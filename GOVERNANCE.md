# Governance

Mac AI Switchboard is a free, local-first, open-source desktop app maintained by
Tarun Agarwal.

This document explains how decisions, pull requests, releases, and official
distribution work for this repository.

## Maintainer Authority

Tarun Agarwal is the project owner and final approver for this repository.

No pull request from another person, bot, fork, dependency-update service, or
external contributor should be merged into a protected branch unless Tarun
Agarwal has explicitly approved it.

Approval must be visible in GitHub as one of:

- A GitHub pull request approval from `@tarunag10`.
- A maintainer comment from `@tarunag10` that clearly says the change is
  approved for merge.
- A direct commit by `@tarunag10` to a maintainer-owned branch when branch
  protection permits that workflow.

Silence, passing CI, a green review from another contributor, or an automated
dependency update is not approval.

## Branch Policy

- `main` is the stable release branch and should be branch-protected.
- `staging` is the release-candidate branch and should be branch-protected.
- Feature work should land through pull requests or maintainer-owned working
  branches.
- Direct pushes to protected branches should be disabled except for emergency
  maintainer recovery.
- Release promotions should follow the release process in `docs/macos-release.md`.

Recommended GitHub branch protection settings:

- Require a pull request before merging.
- Require at least one approving review.
- Require review from Code Owners.
- Dismiss stale approvals when new commits are pushed.
- Require status checks to pass before merging.
- Require branches to be up to date before merging.
- Restrict who can push to protected branches.
- Disable bypasses for external collaborators.

See `docs/repository-settings.md` for the maintainer settings runbook.

## Code Owners

The repository uses `.github/CODEOWNERS` to make `@tarunag10` the required owner
for all paths. GitHub branch protection must enable "Require review from Code
Owners" for this to be enforced by GitHub.

## Decision Rules

The project optimizes for:

- Local-first behavior.
- Reversible client configuration edits.
- No surprise telemetry, account flows, pricing gates, or hosted dependencies in
  the free public app.
- Clear user control over Headroom, RTK, add-ons, and connector state.
- Conservative release gates for signed/notarized macOS distribution.

Large changes should start with an issue or draft PR before implementation.
Examples include new cloud services, updater changes, signing changes, pricing
or account code, security-sensitive proxy behavior, new connectors, and legal or
branding changes.

## Release Authority

Only the maintainer may publish official releases, official DMG artifacts,
official update feeds, signing identities, notarized builds, release notes, or
project distribution channels.

Forks may build and redistribute under the MIT License, but they must follow
`TRADEMARKS.md` and use their own app name, bundle identifier, signing identity,
icon, update channel, and support channel unless written permission is granted.

## Security and Secrets

Security reports must follow `SECURITY.md`.

Contributors must never commit:

- API keys or model-provider tokens.
- Apple Developer credentials.
- Notarization credentials.
- Tauri updater private keys.
- Private endpoints.
- Personal keychain data.
- Private repository contents.

If a secret appears in a PR, the maintainer should treat it as compromised,
remove it from the branch, rotate it outside GitHub, and avoid discussing the
secret value in public comments.

## Policy Changes

Changes to `GOVERNANCE.md`, `MAINTAINERS.md`, `SECURITY.md`, `PRIVACY.md`,
`TERMS.md`, `TRADEMARKS.md`, `NOTICE`, `LICENSE`, `.github/CODEOWNERS`, or
release workflow files require explicit maintainer approval.
