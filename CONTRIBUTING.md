# Contributing

Thanks for helping improve Mac AI Switchboard.

## License

By contributing to this repository, you agree that your contribution is licensed under the repository's MIT License.

## Contribution Guidelines

- Keep changes local-first by default.
- Preserve reversible setup and cleanup for every client adapter.
- Do not add remote telemetry, hosted services, or account flows without an explicit local-only guard.
- Do not commit secrets, signing identities, update keys, tokens, or private endpoints.
- Add or update tests for user-visible behavior, repair workflows, config editing, and release gates.
- Keep branding rules in `TRADEMARKS.md` intact.

## Pull Request Checklist

- `npm run build`
- `npm test -- --run --pool=threads`
- `npm run check:colors`
- `git diff --check`
- `npm run test:desktop` when Rust/Cargo is available locally

## Connector Work

For new coding-agent connectors, start with read-only detection. Add reversible config edits only after the setup, backup, off-mode cleanup, and Doctor repair behavior are tested.
