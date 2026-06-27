# Repository Settings Runbook

This runbook lists the GitHub settings that should be enabled for the public
Mac AI Switchboard repository.

The policy files in the repository document the rules. GitHub branch protection
enforces them.

## Required Maintainer Approval

No pull request from another person, bot, fork, dependency-update service, or
external contributor should be merged unless Tarun Agarwal (`@tarunag10`)
explicitly approves it.

Current live repository check on 2026-06-27: GitHub reported
`tarun/local-switchboard` as the only branch and the default branch, with no
branch protection enabled. Treat the settings below as the target before broad
public contribution or release intake.

The repository includes `.github/CODEOWNERS`:

```text
* @tarunag10
```

To enforce that file, enable these settings for `main` and `staging`.

## Branch Protection: `main`

Recommended settings:

- Require a pull request before merging.
- Require approvals: `1` or more.
- Require review from Code Owners.
- Dismiss stale pull request approvals when new commits are pushed.
- Require status checks to pass before merging.
- Require branches to be up to date before merging.
- Require conversation resolution before merging.
- Restrict who can push to matching branches.
- Allow only `@tarunag10` or a maintainer-owned team to push.
- Do not allow force pushes.
- Do not allow deletions.
- Do not allow bypassing the above settings for external collaborators.

Recommended merge methods:

- Enable merge commits.
- Disable squash merge for protected release promotion branches.
- Disable rebase merge for protected release promotion branches.

The release process expects merge commits so release-candidate ancestry remains
verifiable.

## Branch Protection: `staging`

Use the same settings as `main`.

`staging` may accept release-candidate work before `main`, but it still requires
maintainer approval and passing checks.

## Pull Request Settings

Recommended repository settings:

- Automatically delete head branches after merge.
- Require contributors to complete the pull request template.
- Keep dependency-update PRs open for review; do not auto-merge them.
- Do not grant write access to external contributors unless intentionally added
  as maintainers.

## Security Settings

Recommended repository settings:

- Enable private vulnerability reporting.
- Enable secret scanning.
- Enable push protection for secrets.
- Require status checks that include `npm run check:governance`.

## Release Secrets

Store release secrets only as GitHub Actions secrets or local maintainer secrets.
Never commit them to the repository.

Sensitive values include:

- Apple Developer credentials.
- Notarization credentials.
- Tauri updater private keys and passwords.
- API keys and model-provider tokens.
- Private endpoints.
