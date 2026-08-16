# Phase 6 release-hardening evidence boundary

This phase separates deterministic local contracts from evidence that can only
come from a signed installed application and a real macOS reboot. Source files,
unit tests, an ad-hoc-signed local build, and tester prose are never promoted to
current installed-app release proof by themselves.

## Verified locally in this implementation run

| Concern | Current local evidence | Claim boundary |
| --- | --- | --- |
| Updater recovery | Five focused updater tests pass, including failure-slot restoration and a fail-once/succeed-on-retry exercise using the same update object. Success still clears the slot. | Does not prove a published updater feed, signature, download, replacement, or relaunch. |
| Legacy storage migration | Three fixture tests pass: copy while preserving legacy storage, skip when destination exists, and failure leaves legacy storage intact. | Does not prove migration inside a signed installed app. |
| Launch-at-login cleanup | The focused managed-LaunchAgent cleanup test passes and preserves an unrelated plist. Enable/status commands and Tauri autostart wiring are present. | Does not prove login launch after a physical reboot. |
| Doctor | The local Doctor validation summary reports pass and labels `releaseGateEvidence: false`. | Not reboot or signed-installed-app proof. |
| Rollback | The local Rollback summary passes four checks/four cleanup domains and labels `releaseGateEvidence: false`. | Its fresh-process probe is relaunch evidence, not physical reboot evidence. |
| Uninstall | The local non-destructive uninstall summary passes two checks and labels `releaseGateEvidence: false`, `destructive: false`. | It inventories cleanup and does not prove an installed-app uninstall. |
| Reboot workflow guards | Four Node contract tests pass, including refusal to write a marker without a valid arm receipt and capture of a concrete boot-session baseline. | Tests do not reboot macOS or establish current app trust. |

Generated local summaries under `dist/` are intentionally ignored release
artifacts. They may be regenerated, but should not be committed as proof.

## External blockers in the current run

`scripts/check-phase6-release-hardening.mjs` currently reports release proof
blocked because both required external artifacts are missing or non-passing:

- `dist/reboot-level-installed-proof-summary.json` must be ready and must bind a
  current installed application to successful codesign, Gatekeeper, notarization
  stapler validation, and a marker from a different armed boot session.
- `dist/phase6-installed-operations-proof.json` must be recorded from the
  installed-app exercise and cover Doctor, rollback, uninstall cleanup, updater
  recovery, launch at login, and legacy-storage migration without recording
  prompt content or secrets.

The second artifact uses this minimum contract:

```json
{
  "schemaVersion": 1,
  "kind": "ai_switchboard.phase6_installed_operations_proof",
  "releaseGateEvidence": true,
  "contentOrSecretsRecorded": false,
  "bootTimeUnixSeconds": 0,
  "installedAppArtifactSha256": "64 lowercase hex characters",
  "operations": {
    "doctor": { "verified": true, "evidenceArtifact": "doctor.json", "evidenceSha256": "64 lowercase hex characters" },
    "rollback": { "verified": true, "evidenceArtifact": "rollback.json", "evidenceSha256": "64 lowercase hex characters" },
    "uninstallCleanup": { "verified": true, "evidenceArtifact": "uninstall.json", "evidenceSha256": "64 lowercase hex characters" },
    "updaterRecovery": { "verified": true, "evidenceArtifact": "updater.json", "evidenceSha256": "64 lowercase hex characters" },
    "launchAtLogin": { "verified": true, "evidenceArtifact": "login.json", "evidenceSha256": "64 lowercase hex characters" },
    "legacyStorageMigration": { "verified": true, "evidenceArtifact": "migration.json", "evidenceSha256": "64 lowercase hex characters" }
  }
}
```

An artifact that labels itself local-only, omits an operation, references a
file outside the evidence directory, lacks a referenced evidence file, includes
an invalid/mismatched SHA-256, or lacks the content/secrets boundary fails the checker.
The checker writes `dist/phase6-release-hardening-audit.json` and exits blocked
until both external artifacts validate.

## Gate integration

The root phase gate should run:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib install_pending_update
cargo test --manifest-path src-tauri/Cargo.toml --lib storage::tests
cargo test --manifest-path src-tauri/Cargo.toml --lib dedicated_cleanup_rollback_removes_managed_launch_agents_only
node --test scripts/reboot-level-installed-proof.node-test.mjs scripts/check-phase6-release-hardening.node-test.mjs
node scripts/check-phase6-release-hardening.mjs
```

The last command is expected to remain blocked on development machines without
the two real installed-app artifacts. CI may validate the checker unit tests and
implementation contracts without misreporting the external release gate as
complete.
