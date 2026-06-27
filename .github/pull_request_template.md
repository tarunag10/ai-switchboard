## Summary

Describe the change and why it is needed.

## Maintainer Approval

- [ ] I understand this PR must not be merged unless `@tarunag10` explicitly approves it.
- [ ] I understand passing CI is not maintainer approval.

## Local-First and Safety Checklist

- [ ] This change does not add required sign-in, checkout, pricing API, or hosted-service behavior.
- [ ] This change does not enable telemetry or analytics unless explicitly guarded and documented.
- [ ] This change does not commit secrets, signing credentials, updater keys, tokens, or private endpoints.
- [ ] Managed config edits are reversible, backed up where appropriate, and covered by Doctor/repair behavior.
- [ ] Fork/branding rules in `TRADEMARKS.md` are preserved.

## Validation

List commands run, for example:

- `npm run build`
- `npm test -- --run --pool=threads`
- `npm run check:governance`
- `npm run check:colors`
- `npm run test:desktop`

## Screenshots or Evidence

Add screenshots, logs, or release evidence when the change affects UI, install,
runtime repair, packaging, or release behavior. Remove secrets first.

