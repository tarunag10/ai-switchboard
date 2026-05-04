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
rtk proxy curl -s http://127.0.0.1:6767/stats | jq '.summary.compression.requests_compressed, .summary.compression.total_tokens_removed'
```
Expect: `requests_compressed` increased by at least 1 between the two captures, and `total_tokens_removed` is strictly greater. A bumped mtime on `activity-facts.json` is not enough — interception without compression would still touch that file.

### 8. Bundled runtime is healthy
The desktop ships its own Python venv and `headroom` CLI; if either is broken, the proxy can't start cleanly on a fresh install.
```bash
~/Library/Application\ Support/Headroom/headroom/runtime/venv/bin/headroom --version && \
  ~/Library/Application\ Support/Headroom/headroom/runtime/venv/bin/python3 -c "import headroom; print(headroom.__file__)"
```
Expect: a `headroom, version X.Y.Z` line and a path under `.../runtime/venv/lib/python3.12/site-packages/headroom/__init__.py`. No `ModuleNotFoundError`, no `pydantic-core` mismatch traceback (see `extract_required_pydantic_core_version` in `tool_manager.rs` for the exact failure mode).

### 9. Backend port fallback when 6768 is held
The desktop's internal proxy port (default `6768`) can be claimed by other macOS processes — most often `rapportd` at login. The desktop should scan `6769..=6790` and pick a free one instead of failing.

First, confirm the live port and verify the proxy answers there:
```bash
lsof -iTCP -sTCP:LISTEN -nP 2>/dev/null | awk '$1 ~ /(headroom|python)/ && $9 ~ /:(67[6-9][0-9]|6790)/ { print $9 }'
curl -sS -o /dev/null -w '%{http_code}\n' "http://127.0.0.1:6767/livez"
```
Expect: at least one `127.0.0.1:67XX` line in the 6768-6790 range, and the curl returns `200`.

Then, force a fallback. Quit Headroom, hold 6768 with a Python blocker (`nc -l` exits after one connection, so the proxy's first probe frees the port before fallback can trigger), relaunch, and confirm the proxy comes up on a different port. The proxy on a fallback port boots cold (memory tools / model load), so poll `/livez` for up to 90s instead of a fixed sleep:
```bash
osascript -e 'quit app "Headroom"' 2>/dev/null; sleep 2
python3 -c "import socket,time; s=socket.socket(); s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1); s.bind(('127.0.0.1',6768)); s.listen(16); time.sleep(180)" &
BLOCK_PID=$!
sleep 1
open -a Headroom
for _ in $(seq 1 90); do
  code=$(curl -sS -o /dev/null -w '%{http_code}' "http://127.0.0.1:6767/livez" 2>/dev/null)
  [ "$code" = "200" ] && break
  sleep 1
done
echo "livez=$code"
lsof -iTCP -sTCP:LISTEN -nP 2>/dev/null | awk -v IGNORECASE=1 '$1 ~ /(headroom|python)/ && $9 ~ /:(67[6-9][0-9]|6790)/ { print $9 }'
kill $BLOCK_PID 2>/dev/null
```
Expect: `livez=200`, a `127.0.0.1:67XX` line where `XX` is NOT `68` (the fallback worked). After the test, quit + relaunch Headroom so the next session goes back to 6768.

If the fallback is missing, check `~/Library/Application Support/Headroom/headroom/logs/` for a `[backend_port]` warning line that names the occupant and the chosen fallback port.

### 10. Auth / pricing state is intact
The session token lives in the macOS keychain under service `com.extraheadroom.headroom.account`, account `session-token`; the local pricing state lives next to `activity-facts.json`.
```bash
security find-generic-password -s com.extraheadroom.headroom.account -a session-token >/dev/null 2>&1 && echo 'signed in' || echo 'not signed in'
test -f ~/Library/Application\ Support/Headroom/config/headroom-pricing-state.json && jq -e '.first_seen_at' ~/Library/Application\ Support/Headroom/config/headroom-pricing-state.json
```
Expect: if the build is supposed to be signed in, line 1 reports `signed in`; line 2 prints a non-null `first_seen_at` timestamp. A signed-in build that flips to `not signed in` after relaunch is a regression — keychain access is broken or the token was wiped.

## Inspecting the proxy directly

When inspecting the running proxy by hand (e.g. checking `/stats`), wrap `curl` with `rtk proxy` to bypass RTK's output filtering — otherwise large JSON responses get summarized into a type-shape view that looks like a broken endpoint:

```bash
rtk proxy curl -s http://127.0.0.1:6767/stats | jq .summary
```

## When something fails

- Proxy log silent → check `~/Library/Application Support/Headroom/headroom/logs/` for a newer log file or a crash file.
- RTK missing → check the managed block in `~/.zshrc` / `~/.zprofile` is intact and the shell has been reloaded.
- MCP tool missing → restart Claude Code; the MCP server registration happens at session start.
