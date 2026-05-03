# Beta smoke test

After installing a new beta (`-rc.N`) build, paste this file into Claude Code and ask it to run the checks. Each check has a single expected signal — if any fail, stop and investigate before promoting to stable.

## Setup

1. Quit and relaunch Headroom from Applications.
2. Confirm the tray icon appears in the menu bar.
3. Open the dashboard window once (so the proxy is fully booted).

## Checks

Claude: run each block and report PASS / FAIL with the observed value.

### 1. Version matches the new beta
```bash
ls ~/Applications/Headroom.app/Contents/Info.plist >/dev/null && \
  /usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" /Applications/Headroom.app/Contents/Info.plist
```
Expect: the `-rc.N` version you just installed.

### 2. Proxy is intercepting this conversation
Send a trivial prompt ("say hi"), then:
```bash
stat -f '%Sm' ~/Library/Application\ Support/Headroom/config/activity-facts.json
```
Expect: mtime within the last minute. `lastTransformation` inside the file is a "Recent large compression" tile pick (gated on >=1000 tokens saved and >20% savings, see `activity_facts.rs`), not a per-request heartbeat — don't use it as a liveness signal.

### 3. RTK is on PATH and reports savings
```bash
rtk --version && rtk gain | head -5
```
Expect: a version line and a gain summary, no "command not found".

### 4. MCP retrieve tool is available (only if memory tools are enabled)
First check whether the proxy was started with memory tools:
```bash
ls ~/Library/Application\ Support/Headroom/headroom/logs/ | grep -E 'no-memory-tools' >/dev/null && echo 'memory tools DISABLED — skip this check' || echo 'memory tools enabled — run check'
```
If enabled, have Claude call `mcp__headroom__headroom_retrieve` with any small query and expect a tool result (not "No such tool available").

### 5. Tray → Dashboard renders
Click the tray icon, open the dashboard. Expect savings chart and per-client stats render without a blank/error state.

### 6. Pause / resume cleanly strips and restores interception
In Settings, toggle Pause then Resume. After Pause, `cat ~/.claude/settings.json | grep -c headroom-rtk-rewrite` should return `0`; after Resume it should return `1`.

### 7. Real compression event (not just a heartbeat)
Capture the compression counter, then trigger a large request (Claude: read a long file like `src-tauri/src/lib.rs` with no offset/limit so the prompt exceeds ~10k tokens), then re-check:
```bash
rtk proxy curl -s http://127.0.0.1:6768/stats | jq '.summary.compression.requests_compressed, .summary.compression.total_tokens_removed'
```
Expect: `requests_compressed` increased by at least 1 between the two captures, and `total_tokens_removed` is strictly greater. A bumped mtime on `activity-facts.json` is not enough — interception without compression would still touch that file.

### 8. Bundled runtime is healthy
The desktop ships its own Python venv and `headroom` CLI; if either is broken, the proxy can't start cleanly on a fresh install.
```bash
~/Library/Application\ Support/Headroom/headroom/runtime/venv/bin/headroom --version && \
  ~/Library/Application\ Support/Headroom/headroom/runtime/venv/bin/python3 -c "import headroom; print(headroom.__file__)"
```
Expect: a `headroom, version X.Y.Z` line and a path under `.../runtime/venv/lib/python3.12/site-packages/headroom/__init__.py`. No `ModuleNotFoundError`, no `pydantic-core` mismatch traceback (see `extract_required_pydantic_core_version` in `tool_manager.rs` for the exact failure mode).

### 9. Auth / pricing state is intact
The session token lives in the macOS keychain under service `com.extraheadroom.headroom.account`, account `session-token`; the local pricing state lives next to `activity-facts.json`.
```bash
security find-generic-password -s com.extraheadroom.headroom.account -a session-token >/dev/null 2>&1 && echo 'signed in' || echo 'not signed in'
test -f ~/Library/Application\ Support/Headroom/config/headroom-pricing-state.json && jq -e '.first_seen_at' ~/Library/Application\ Support/Headroom/config/headroom-pricing-state.json
```
Expect: if the build is supposed to be signed in, line 1 reports `signed in`; line 2 prints a non-null `first_seen_at` timestamp. A signed-in build that flips to `not signed in` after relaunch is a regression — keychain access is broken or the token was wiped.

## Inspecting the proxy directly

When inspecting the running proxy by hand (e.g. checking `/stats`), wrap `curl` with `rtk proxy` to bypass RTK's output filtering — otherwise large JSON responses get summarized into a type-shape view that looks like a broken endpoint:

```bash
rtk proxy curl -s http://127.0.0.1:6768/stats | jq .summary
```

## When something fails

- Proxy log silent → check `~/Library/Application Support/Headroom/headroom/logs/` for a newer log file or a crash file.
- RTK missing → check the managed block in `~/.zshrc` / `~/.zprofile` is intact and the shell has been reloaded.
- MCP tool missing → restart Claude Code; the MCP server registration happens at session start.
