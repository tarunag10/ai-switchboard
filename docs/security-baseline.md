# Phase 0 security baseline

The Phase 0 gate is defined by `fixtures/security-baseline-v1.json` and checked by `node scripts/check-phase-0-security-baseline.mjs`. The checker requires named compiled tests plus implementation evidence for every control; it does not replace executing the Rust tests.

| Control | Current evidence |
|---|---|
| Remote gateway probes disabled by default | Gateway readiness is advisory and remote profiles cannot opt into connectivity probing. |
| Loopback-only local probe rules | Gateway/add-on probes reject non-loopback addresses; the intercept rejects non-loopback Host and browser Origin requests. |
| Log redaction | Nested fake credentials, private-key markers, and credential filenames are redacted. |
| Secret-like Repo Intelligence exclusions | Secret paths are generated/excluded and cannot enter default packs. |
| Cache namespace isolation | Account and workspace scopes hash/separate entries; unscoped namespaces fail closed. |
| Config backup permissions | Every Unix backup is forced to owner read/write (`0600`), even when the source is more permissive. |
| External URL validation | Unsupported schemes, credentials, line breaks, loopback/private/local hosts are rejected. |
| Updater signature verification | Update checks remain disabled without the updater public key, and the configured key is passed to Tauri's signed updater builder. |
| Managed-footprint exports free of secrets | Footprint reports inventory paths and keychain labels without reading or serializing values. |

Run the machine-readable coverage gate:

```bash
node scripts/check-phase-0-security-baseline.mjs
```

Then execute the Rust tests referenced by the fixture catalog (or the complete single-threaded desktop suite) before declaring the baseline green. This baseline is local source/test evidence; it does not claim public release signing, notarization, or live updater-feed readiness.
