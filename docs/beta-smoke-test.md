# Beta smoke test

After installing a new beta (`-rc.N`) build, paste this file into Claude Code and ask it to run the checks. Each check has a single expected signal — if any fail, stop and investigate before promoting to stable.

## Setup

These checks assume the installed bundle is `/Applications/Mac AI Switchboard.app`.

1. Quit and relaunch Mac AI Switchboard from Applications.
2. Confirm the tray icon appears in the menu bar.
3. Open the dashboard window once (so the proxy is fully booted).

## Switchboard checks

Run these from the tray Home view before the client-specific passes. These checks verify the local-first Mac AI Switchboard layer, not only the underlying Headroom proxy.

### S1. Local-only shell is focused on Mac controls

With `HEADROOM_LOCAL_ONLY=1` and `VITE_HEADROOM_LOCAL_ONLY=1`, open the tray window.

Expect: Home shows the Switchboard panel and Doctor area. Cloud upgrade/auth prompts are not reachable from navigation or notification clicks; remote services show as Off.

### S2. Mode buttons explain their effect

Click each mode button without leaving Home:

- Full optimization
- Headroom only
- RTK only
- Off

Expect: the mode effect line changes to describe exactly what will be routed or left alone. The selected mode badge matches the requested mode. The local footprint matrix changes with each mode and shows **Client routing**, **Shell output**, and **Repo packs** as On, Off, or Local.

### S3. Requested mode vs active mode is honest

Create a degraded setup by requesting Full optimization while either RTK or client routing is missing. The easiest safe path is to uninstall or disable RTK from Addons, then request Full optimization.

Expect: Switchboard still shows the requested mode, but the attention line reports the active mode and says to run Doctor. Doctor lists the missing dependency instead of leaving the mode change looking stuck.

### S4. Doctor repairs missing RTK

From the degraded state in S3, run the RTK Doctor repair.

Expect: Doctor offers **Install RTK** when RTK is missing or disabled. Doctor triage shows automatic and manual counts. After repair, RTK is installed/enabled, the Doctor issue clears or changes to a more specific RTK integration issue, and the Switchboard refreshes within a few seconds.

### S5. Off mode is a clean pass-through

Switch to Off.

Expect: client routing hooks are removed, RTK integration is disabled, Headroom stops intercepting, and the Switchboard active mode becomes Off after refresh. Supported clients should behave as they did before Headroom.

### S6. Oversized Codex compression refusal is recoverable

If Codex hits:

```text
unexpected status 413 Payload Too Large: compression_refused
```

Expect: Codex temporarily bypasses Headroom so work can continue. After compacting context or switching to **RTK only**, Doctor shows the Codex bypass issue and **Reset Codex** routes Codex through Headroom again. If this happens with several active Codex chats or goals, follow [Codex Compression Troubleshooting](codex-compression-troubleshooting.md).

### S7. Codex model/provider mismatch is repairable

If Codex hits:

```text
The '' model is not supported when using Codex with a ChatGPT account.
```

Expect: this is treated as a Codex routing/config problem, not as an RTK compression problem. Doctor should flag Codex routing config if the managed provider block or proxy URL is stale, and **Repair Codex** should re-apply the reversible Codex setup.

### S8. Planned connectors are visible but manual

Open Settings and inspect the coding tool connector list.

Expect: Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, and Goose all appear when detected or known to the connector registry. Each planned connector shows a **Planned** badge, setup phase, category, copyable manual setup guide, and disabled switch. Doctor may show **Planned coding tools detected**, but it must be a manual step with no **Repair all** action for those tools. If a report mixes repairable and manual items, Doctor says **Repair all will leave manual steps visible.** Launcher auto-setup and proxy verification should include only managed connectors such as Claude Code and Codex.

### S9. Repo Intelligence index health

Open Addons, enter a local repo path in the Repo Intelligence card, and click **Index**.

Expect: the card shows indexed signals, context packs, the repo path, and indexed timestamp. Restart the app and return to Addons.

Expect: the latest Repo Intelligence summary reloads from managed app storage. If the indexed repo folder is moved or deleted, Doctor shows a manual warning to re-index an available local repo and does not offer **Repair all** for that issue.

Click **Copy pack**.

Expect: bounded Markdown context pack is copied for agent handoff. It includes repo path, pack headings, estimated token counts, file lists, but no file contents.

Click **Copy agent manifest**.

Expect: JSON manifest is copied for external coding agents. It includes `mac_ai_switchboard.repo_intelligence_manifest`, implementation/verification/handoff pack ids, per-pack commands, token savings, and read-only safety flags.

Click **Clear** in the Repo Intelligence card.

Expect: the card returns to the read-only preview state, the saved repo path disappears, and Doctor no longer reports stale or missing Repo Intelligence index warnings.

### S10. Release readiness visible in Settings

Open Settings and find **Release readiness**.

Expect: card shows 9 checks across Environment, Signing, and Installed App Smoke. **Copy report command** copies `npm run release:report`. The card should not claim the app is releasable until signing/notarization variables and installed-app smoke are complete.

## Checks (Claude Code pass)

Run these from a Claude Code session and report PASS / FAIL with the observed value. Checks 1, 5, 8, 9, and 10 are client-agnostic — run them once in either client. Codex has very different wiring (no RTK, no `~/.claude/settings.json`, pay-per-token), so its equivalents of checks 6 and 7 live in the **Codex pass** below; run that whole section from a Codex session.

### 1. Version matches the new beta
```bash
ls /Applications/Mac\ AI\ Switchboard.app/Contents/Info.plist >/dev/null && \
  /usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" /Applications/Mac\ AI\ Switchboard.app/Contents/Info.plist
```
Expect: the `-rc.N` version you just installed.

### 2. Proxy is intercepting this conversation
Send a trivial prompt ("say hi"), then:
```bash
stat -f '%Sm' ~/Library/Application\ Support/Headroom/config/activity-facts.json
```
Expect: mtime within the last minute. `lastTransformation` inside the file is a "Recent large compression" tile pick (gated on >=1000 tokens saved and >20% savings, see `activity_facts.rs`), not a per-request heartbeat — don't use it as a liveness signal.

### 3. RTK is on PATH and reports savings (Claude Code only — RTK does not rewrite Codex)
```bash
zsh -lc 'rtk --version && rtk gain | head -5'
```
Expect: a version line and a gain summary, no "command not found". The `zsh -lc` wrapper is required: `rtk` is added to PATH by the `headroom:managed_rtk` block in `~/.zprofile`, which only a login shell sources. Claude Code's Bash tool (and Codex's shell tool) spawn a non-login, non-interactive shell that does *not* source it, so a bare `rtk` here reports `command not found` on a perfectly healthy install. A login shell exercises the same PATH wiring a real terminal gets, so this confirms both that the managed block is intact and that the binary runs.

### 4. MCP retrieve tool is available (Claude Code only; only if memory tools are enabled)
First check whether the proxy was started with memory tools:
```bash
ls ~/Library/Application\ Support/Headroom/headroom/logs/ | grep -E 'no-memory-tools' >/dev/null && echo 'memory tools DISABLED — skip this check' || echo 'memory tools enabled — run check'
```
If enabled, have Claude call `mcp__headroom__headroom_retrieve` with any small query and expect a tool result (not "No such tool available").

### 5. Tray → Dashboard renders
Click the tray icon, open the dashboard. Expect savings chart and per-client stats render without a blank/error state.

### 6. Pause / resume cleanly strips and restores interception
In Settings, toggle Pause then Resume. After Pause, `cat ~/.claude/settings.json | grep -c headroom-rtk-rewrite` should return `0`; after Resume it should return `1`. This verifies the Claude Code config only — Pause clears *all* clients, so check C4 in the Codex pass confirms Codex's config is stripped and restored too.

### 7. Proxy is actively optimizing this conversation (not just a heartbeat)
First check which mode the proxy is in — the right signal differs:
```bash
rtk proxy curl -s http://127.0.0.1:6767/stats | jq -r '.summary.mode'
```
A Claude Code subscription/OAuth session (the normal desktop case) reports `cache`. The proxy deliberately stays in cache mode for this traffic because it's billed on the cache-weighted meter, where token mode's prefix rewrites bust the cache and inflate usage — so `requests_compressed` will *never* move here (see the `HEADROOM_MODE` comment in `tool_manager.rs`). The intercept only flips to `token` mode for pay-per-token API-key traffic. Pick the matching sub-check.

Timing matters either way: a `Read` result becomes part of Claude's *next* outgoing prompt, not the one currently being composed. So the baseline capture, the large Read, and the re-check cannot all happen in one turn — the re-check will still show the old numbers.

**If mode is `cache`** (normal desktop / Claude Code subscription):
1. Capture the baseline:
   ```bash
   rtk proxy curl -s http://127.0.0.1:6767/stats | jq '{prefix_frozen: .summary.uncompressed_requests.prefix_frozen, cache_savings_usd: .summary.cost.breakdown.cache_savings_usd, total_tokens_before: .summary.compression.total_tokens_before_with_cli_filtering}'
   ```
2. End the turn with a large Read in flight — e.g. ask Claude to read a long file like `src-tauri/src/lib.rs` with as large an offset/limit window as the Read tool allows (the 25k-token cap means you cannot read it whole; ~1300-1500 lines is plenty).
3. On the *next* turn, re-run the same `jq` command.

Expect: `cache_savings_usd` is strictly greater, `prefix_frozen` increased by at least 1, and `total_tokens_before` jumped by roughly the size of the Read. A bumped mtime on `activity-facts.json` is not enough — interception alone would still touch that file without delivering cache savings.

**If mode is `token`** (pay-per-token API-key traffic — this is also the branch Codex hits; the Codex pass below adds a Codex-attributed version):
1. Capture the baseline:
   ```bash
   rtk proxy curl -s http://127.0.0.1:6767/stats | jq '.summary.compression.requests_compressed, .summary.compression.total_tokens_removed'
   ```
2. End the turn with the same large Read in flight (~1300-1500 lines clears the compression threshold).
3. On the *next* turn, re-run the same `jq` command.

Expect: `requests_compressed` increased by at least 1, and `total_tokens_removed` is strictly greater.

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
Expect: `livez=200`, a `127.0.0.1:67XX` line where `XX` is NOT `68` (the fallback worked). After the test, quit + relaunch Mac AI Switchboard so the next session goes back to 6768.

If the fallback is missing, check `~/Library/Application Support/Headroom/headroom/logs/` for a `[backend_port]` warning line that names the occupant and the chosen fallback port.

### 10. Auth / pricing state is intact
The session token lives in the macOS keychain under service `com.extraheadroom.headroom.account`, account `session-token`; the local pricing state lives next to `activity-facts.json`.
```bash
security find-generic-password -s com.extraheadroom.headroom.account -a session-token >/dev/null 2>&1 && echo 'signed in' || echo 'not signed in'
test -f ~/Library/Application\ Support/Headroom/config/headroom-pricing-state.json && jq -e '.first_seen_at' ~/Library/Application\ Support/Headroom/config/headroom-pricing-state.json
```
Expect: if the build is supposed to be signed in, line 1 reports `signed in`; line 2 prints a non-null `first_seen_at` timestamp. A signed-in build that flips to `not signed in` after relaunch is a regression — keychain access is broken or the token was wiped.

## Codex checks (Codex pass)

Run these from a Codex CLI session (or with Codex configured and at least one Codex prompt sent this session). Codex routes through Headroom via an `OPENAI_BASE_URL` shell export plus a managed provider block in `~/.codex/config.toml` — not `~/.claude/settings.json` and not RTK — and its traffic is pay-per-token, so the proxy runs it in `token` mode.

### C1. Codex is configured to route through Headroom
```bash
grep -q 'model_provider = "headroom"' ~/.codex/config.toml && \
  grep -q 'openai_base_url = "http://127.0.0.1:6767/v1"' ~/.codex/config.toml && \
  grep -qF '[model_providers.headroom]' ~/.codex/config.toml && \
  grep -q 'export OPENAI_BASE_URL=http://127.0.0.1:6767/v1' ~/.zshrc ~/.zprofile 2>/dev/null && \
  echo PASS || echo FAIL
```
Expect: `PASS`. `~/.codex/config.toml` carries both managed marker blocks — `# >>> headroom:codex_cli >>>` with the root `model_provider`/`openai_base_url` keys, and `# >>> headroom:codex_cli_provider >>>` with the `[model_providers.headroom]` table — and a managed shell block exports `OPENAI_BASE_URL`. A `FAIL` means setup didn't write one of them (see `configure_codex_provider_block` / `configure_shell_block` in `client_adapters.rs`).

### C2. Codex traffic is actively optimized (token mode)
Codex is billed per token, so unlike a Claude Code subscription it runs in `token` mode and `requests_compressed` *does* move. Run this from inside Codex.
1. Capture the baseline:
   ```bash
   rtk proxy curl -s http://127.0.0.1:6767/stats | jq '{mode: .summary.mode, primary_model: .summary.primary_model, requests_compressed: .summary.compression.requests_compressed, total_tokens_removed: .summary.compression.total_tokens_removed}'
   ```
2. End the turn with a large file read in flight from Codex (~1300-1500 lines clears the compression threshold). As in check 7, the read lands in Codex's *next* prompt, so the re-check must be on a later turn.
3. On the next turn, re-run the same command.

Expect: `mode` is `token`, `primary_model` is a `gpt-*` model (confirms Codex — not Claude — is the traffic being measured), `requests_compressed` increased by at least 1, and `total_tokens_removed` is strictly greater. If `primary_model` is a `claude-*` model, the proxy is dominated by Claude traffic — confirm the prompt actually ran through Codex before trusting this check.

### C3. Codex savings are attributed on the dashboard
Open the dashboard and confirm a **Codex** group appears in the per-provider savings with non-zero values. Provider `openai` maps to the Codex group (`mergeProviderSavingsForDisplay` in `dashboardHelpers.ts`); a missing Codex group after Codex traffic means per-provider attribution isn't tagging OpenAI requests.

### C4. Pause / resume cleanly strips and restores Codex routing
The Claude equivalent is check 6; Pause clears *all* client setups, so it must remove Codex's config too. In Settings, toggle Pause then Resume (restore runs on a background thread, so give it a second), checking after each:
```bash
grep -c 'headroom:codex_cli' ~/.codex/config.toml
cat ~/.zshrc ~/.zprofile 2>/dev/null | grep -c 'OPENAI_BASE_URL=http://127.0.0.1:6767'
```
Expect: after Pause both print `0`; after Resume both are non-zero (config.toml back to `4` marker lines, shell back to one export per managed profile). Pause routes through `disable_codex_cli` — strips both TOML blocks, the `openai_base_url` root key, and the shell blocks; Resume re-applies them via `restore_client_setups`.

## Inspecting the proxy directly

When inspecting the running proxy by hand (e.g. checking `/stats`), wrap `curl` with `rtk proxy` to bypass RTK's output filtering — otherwise large JSON responses get summarized into a type-shape view that looks like a broken endpoint:

```bash
rtk proxy curl -s http://127.0.0.1:6767/stats | jq .summary
```

Every `rtk` invocation in this doc (checks 3, 7, C2, and above) has the same PATH caveat as check 3: when Claude Code or Codex runs them through their shell tool, `rtk` is not on PATH because the non-login shell never sources `~/.zprofile`. Either wrap the command in `zsh -lc '...'`, or call the binary by its managed path:

```bash
"$HOME/Library/Application Support/Headroom/headroom/bin/rtk" proxy curl -s http://127.0.0.1:6767/stats | jq .summary
```

## When something fails

- Proxy log silent → check `~/Library/Application Support/Headroom/headroom/logs/` for a newer log file or a crash file. This compatibility storage path remains named Headroom until a dedicated state migration is implemented.
- RTK missing → check the managed block in `~/.zshrc` / `~/.zprofile` is intact and the shell has been reloaded.
- MCP tool missing → restart Claude Code; the MCP server registration happens at session start.
