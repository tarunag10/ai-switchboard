# Codex plan: mac-ai-switchboard

Status: portfolio planning baseline, 2026-09-26

## Local focus

Product/experiment; define the user, canonical status, minimum viable evidence, and archive/maintenance decision before new features.

## Near-term slice

- Confirm the active branch, remote, product identity, and current release/deployment state.
- Inspect existing tests and workflows before adding abstractions; preserve unrelated work.
- Add the smallest missing quality gate: focused test, typecheck/build, accessibility check, or release-truth evidence.
- Keep user data local by default and make consent, export, deletion, and failure states explicit where data is sensitive.

## Acceptance gate

- Changed files are intentional and free of credentials, caches, build output, and editor metadata.
- A focused verification command or read-only inspection is recorded in the commit/PR description.
- External distribution, App Store, notarisation, deployment, or legal-review claims remain separately labelled.

## Later slices

1. Stabilise core behavior and test fixtures.
2. Improve onboarding, accessibility, privacy, and error recovery.
3. Add release evidence and automate only the verified checks.
4. Decide whether this remains canonical, becomes a satellite, or is archived after reviewing usage and duplication.

Global coordination: [portfolio-global-plan.md](https://github.com/tarunag10/ai-switchboard/blob/main/docs/portfolio-global-plan.md)

