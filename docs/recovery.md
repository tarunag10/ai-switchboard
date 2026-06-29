# Recovery and Uninstall

Mac AI Switchboard keeps uninstall and recovery scoped to app-owned state and
managed marker blocks. It does not delete user repositories, provider accounts,
AWS credentials, SSO cache, or unmanaged shell/profile content.

## Preview Cleanup

Use either preview path before removing the app:

```bash
mac-ai-switchboard --uninstall-dry-run
```

Or use **Settings -> Uninstall -> Copy dry-run** in the app. The dry-run report
is read-only and lists each target, whether it exists now, the planned action,
and whether explicit uninstall confirmation is required.

## Removed By Uninstall

Uninstall removes only Switchboard-owned surfaces:

- Managed Claude Code settings hooks and managed shell routing blocks.
- Managed Codex provider/routing blocks and AGENTS.md guidance blocks.
- Managed RTK, MarkItDown, Caveman, and Ponytail integration files or plugin
  entries.
- `~/Library/Application Support/Mac AI Switchboard`.
- Preserved legacy `~/Library/Application Support/Headroom` after explicit
  uninstall confirmation.
- `~/.headroom` managed runtime files.
- Current bundle data for `com.tarunagarwal.mac-ai-switchboard`: Preferences,
  Caches, WebKit data, HTTPStorages, saved application state, logs, and
  LaunchAgents.
- Legacy bundle data for `com.extraheadroom.headroom` plus `Headroom.plist`
  LaunchAgent.
- Switchboard-owned Keychain service entries by service/account name, without
  exporting or logging secret values.
- Managed backup siblings with `*.headroom-backup-*` or
  `*.nommer-backup-*` names.

## Preserved

Uninstall preserves:

- User repositories and source files.
- User-owned Claude, Codex, shell, and editor config outside managed marker
  blocks.
- Provider credentials and accounts not written by Switchboard.
- AWS credentials, SSO cache, and profiles.
- Any connector sidecar or app config that is only detected or guided unless it
  contains a Switchboard-managed marker block.

Use **Off** mode when you only want to stop routing. Off mode removes managed
routing hooks and provider overrides while leaving app storage, local evidence,
and reinstall state available.

## Message Log Purge

Full message logging is off by default. If it was temporarily enabled for
debugging, use the app's message-log purge action before sharing diagnostics.
The purge removes persisted Activity feed facts that may contain historical
request or compressed-message payloads. Restart the runtime afterward so the
proxy runs without raw message capture.

## Codex Thread DB Restore

Codex history retagging is opt-in. In `ask` or `disabled` mode, Switchboard does
not write Codex SQLite thread stores, even when Codex is routed through
Headroom. If retagging is enabled, every write first creates a sibling backup:

```text
~/.codex/sqlite/state_5.sqlite.switchboard-backup-20260629T120000Z
```

Restore from a Switchboard backup only when Codex is closed:

```bash
mac-ai-switchboard --restore-codex-thread-db-backup ~/.codex/sqlite/state_5.sqlite.switchboard-backup-20260629T120000Z
```

The restore copies the backup over the original `state_<N>.sqlite` file in the
same directory. Unknown Codex store versions are skipped until the schema is
verified.
