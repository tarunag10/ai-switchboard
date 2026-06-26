# Mac AI Switchboard

**A local-first Mac menu bar switchboard for Headroom, RTK, Claude Code, Codex, and token-saving coding add-ons.**

[![Repository](https://img.shields.io/badge/GitHub-tarunag10%2Fmac--ai--switchboard-blue?style=for-the-badge&logo=github)](https://github.com/tarunag10/mac-ai-switchboard)
[![License](https://img.shields.io/badge/license-MIT-green?style=for-the-badge)](LICENSE)

## License, Branding, Contributions

The source code is MIT-licensed. Official app names, icons, signing identities, update endpoints, release artifacts, and distribution channels are not licensed for reuse. Forks should rename the app and use their own bundle identifier, signing identity, and update channel. See [TRADEMARKS.md](TRADEMARKS.md), [NOTICE](NOTICE), [CONTRIBUTING.md](CONTRIBUTING.md), and [SECURITY.md](SECURITY.md).

Mac AI Switchboard is a personal, open-source wrapper around the Headroom Desktop shell, reshaped into a privacy-first Mac utility. The goal is an on/off control panel for local coding-agent optimization:

- **Full optimization:** Headroom proxy routing plus RTK command-output compression.
- **Headroom only:** route supported clients through the local Headroom proxy.
- **RTK only:** keep command-output compression without routing LLM traffic through Headroom.
- **Off:** remove local routing hooks and leave coding clients behaving normally.

The app is **local-first**, not offline-only. Claude/OpenAI model calls still go to the normal remote APIs; switchboard state, routing, reversible client config edits, RTK hooks, Doctor/repair workflows, telemetry defaults, and Repo Intelligence metadata live on your Mac.

> Current status: active productization branch. The standalone repository is public, but release artifacts are not published yet. Build from source for now. For installation paths, first-run footprint, DMG expectations, and uninstall behavior, see [docs/install.md](docs/install.md).

## Supported Local Tools

| Tool | Role | Status |
| --- | --- | --- |
| Headroom | Local prompt/context optimization proxy | Core runtime |
| RTK | Token-optimized command output for shells and agents | Add-on / mode |
| Claude Code | Local client routing and RTK hook target | Supported |
| Codex | Local provider/base-url routing target | Supported |
| Gemini CLI | Detected in the switchboard; reversible routing adapter pending | Planned |
| OpenCode | Detected in the switchboard; reversible routing adapter pending | Planned |
| Cursor | Editor/agent config detection and guided routing adapter pending | Planned |
| Grok / xAI CLI | Provider/base-url adapter pending once a stable CLI surface is identified | Planned |
| Aider | Detected in the switchboard; reversible routing adapter pending | Planned |
| Continue | Detected in the switchboard; reversible routing adapter pending | Planned |
| Goose | Agent provider adapter pending; Repo Intelligence handoff available | Planned |
| Ponytail | Agent behavior nudge toward smaller changes | Add-on |
| MarkItDown | Document-to-Markdown preprocessing | Add-on |
| Repo Intelligence | Read-only local repo index, context packs, persisted summary, Doctor warnings, and clear/copy UI | Read-only foundation |

## Recommended Future Integrations

- **Repo Intelligence / Graphy-style repo graph:** the app now ships a read-only foundation: local indexing, file classification, rough token estimates, bounded context packs, persisted latest summary, Doctor stale/missing-index warnings, clear saved index, and copyable handoff packs. The CLI now exposes an agent-readable `--manifest` for pack discovery; remaining work is the full Graphy-style symbol graph, import/dependency graph, call graph, test relationship graph, and MCP context-pack API. See [docs/repo-intelligence-plan.md](docs/repo-intelligence-plan.md).
- **Agent connector expansion:** Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, Zed AI, and similar coding agents are planned local-first connectors. The app now tracks each planned tool's config surface, safe manual workflow, and automation gates. Build order stays detection first, guided setup second, reversible config adapters third, then Doctor repair/off-mode cleanup once each tool has a stable config surface.
- **Add-on hardening:** RTK, Ponytail, and MarkItDown are installable add-ons today. Next work should add deeper health checks, smoke-test actions, and clearer savings attribution per add-on.

## Local-First Defaults

For a personal build, create `.env` with:

```bash
HEADROOM_LOCAL_ONLY="1"
VITE_HEADROOM_LOCAL_ONLY="1"
VITE_HEADROOM_REMOTE_TELEMETRY="0"
```

In local-only mode, the app hides cloud upgrade/auth surfaces, disables Clarity/Sentry/Aptabase unless explicitly re-enabled, and keeps the Home view focused on Mac-side switchboard controls and Doctor repairs.

## Using Switchboard

The Home view is the Mac-side optimization control surface:

| Mode | What It Does | Typical Use |
| --- | --- | --- |
| Full optimization | Routes supported clients through Headroom and enables RTK shell-output compression. | Daily coding-agent work when you want the full local optimization layer. |
| Headroom only | Routes supported clients through the local Headroom proxy while leaving shell output unchanged. | LLM prompt/context optimization without shell command rewriting. |
| RTK only | Keeps LLM traffic direct and enables RTK shell-output compression. | When a client should bypass Headroom, or when an oversized request hits compression refusal. |
| Off | Removes local routing hooks and disables RTK integration. | Clean pass-through mode before debugging client config or comparing behavior. |

The app separates **requested mode** from **active mode**. If a mode is requested but a dependency is missing, Switchboard shows what is actually active and points you to Doctor. Doctor currently repairs:

- Headroom runtime reachability.
- Reversible client setup for supported installed tools.
- RTK install/enablement and RTK shell integration.
- Codex direct-bypass state after Headroom returns a `413 compression_refused` response for an oversized request.

For a real-world Codex error:

```text
unexpected status 413 Payload Too Large: compression_refused
```

The app lets Codex bypass Headroom temporarily so work can continue. After compacting the conversation or switching to **RTK only**, use Doctor to reset the bypass and route through Headroom again. If this happens with more than 2-3 active Codex chats or goals are open, see [Codex Compression Troubleshooting](docs/codex-compression-troubleshooting.md).

If Codex instead reports:

```text
The '' model is not supported when using Codex with a ChatGPT account.
```

That is a Codex model/provider configuration problem, not the usual Headroom `413` compression path. Doctor can re-apply the managed Codex provider block, localhost proxy URL, and backups; after that, choose a Codex-supported ChatGPT model before retrying.

## Upstream Foundation

This project started from the MIT-licensed Headroom Desktop shell. The current work keeps the useful Tauri/Rust + React + managed-runtime foundation while moving the product toward a standalone local Mac AI work switchboard.

The original Headroom Desktop documentation continues below until the standalone docs are fully rewritten.

---

![Headroom dashboard showing $204 saved across 66.8M tokens](docs/screenshot-1.png)

---

> **Note:** Headroom supports **Claude Code** and **Codex** (CLI and desktop app). Support for additional clients is planned.

Headroom is a local-first desktop tray app that routes your coding clients through a local optimization pipeline. The stable target is macOS; Linux builds are currently experimental. It installs and manages a self-contained Python runtime, bundles proven token-saving tools, and surfaces savings analytics — all without touching your system environment.

## How it works

Headroom sits in your menu bar and does three things:

1. **Installs a managed Python runtime** into Headroom-owned storage — isolated from your system Python, no `pip install --user` pollution.
2. **Chains token-saving tools** (`headroom` for prompt optimization, `rtk` for CLI output compression) between your client and the LLM API.
3. **Shows you the math** — daily and monthly savings charts, per-client token stats, and pipeline health.

The app ships as a slim Tauri shell (~a few MB). Heavy Python components are fetched on first launch and kept in `~/Library/Application Support/Headroom`.

Compatibility note: the packaged app is now **Mac AI Switchboard**, but the managed runtime/storage directory remains named `Headroom` until a dedicated state migration exists. This avoids orphaning existing runtimes, logs, receipts, backups, and cleanup paths.

## What Headroom changes on your system

Full disclosure of every location Headroom writes to, so you can decide before installing. The install screen in the app shows the same list, and the uninstall flow reverses every item.

**On install:**

- Downloads a self-contained Python runtime (~2 GB) under `~/Library/Application Support/Headroom`. Your system Python is untouched.
- Adds a `PreToolUse` hook to `~/.claude/settings.json` and a script at `~/.claude/hooks/headroom-rtk-rewrite.sh` so Claude Code routes through Headroom. A timestamped backup of `settings.json` is written before any edit.
- For Codex, adds a Headroom provider block to `~/.codex/config.toml` and an `OPENAI_BASE_URL` export to your managed shell block so the Codex CLI and desktop app route through the local proxy. The TOML block is fenced with `# >>> headroom:... >>>` markers, a backup is written before any edit, and existing Codex threads are retagged to the managed provider.
- Creates `~/Library/Application Support/Headroom` for logs, caches, and per-client setup state.
- Stores your Headroom session token in the macOS Keychain under services prefixed `com.extraheadroom.headroom`.
- If you opt into "launch at login," installs a LaunchAgent plist at `~/Library/LaunchAgents/`. Never added otherwise.
- Adds a managed block to your shell profile (`.zshrc`, `.zprofile`, etc.) that prepends Headroom's managed `bin` directory (under `~/Library/Application Support/Headroom`) to `PATH` so `rtk` is available in your terminals. Every managed block is fenced with `# >>> headroom:... >>>` markers and can be removed by hand if you prefer.

**On quit (or pause):** Headroom tears down everything that would intercept your clients — the Claude Code hook entry and hook script, the `ANTHROPIC_BASE_URL` and `OPENAI_BASE_URL` redirects, the Codex provider block in `~/.codex/config.toml`, and the managed shell blocks. Codex threads are retagged back to their native provider. Claude Code and Codex behave exactly as they did before Headroom was launched. The Python runtime, logs, and keychain entries stay on disk so the next launch is fast.

**On uninstall (Settings → Uninstall Headroom):** Everything listed above is removed, including the LaunchAgent plist, `~/Library/Preferences/com.extraheadroom.headroom*`, `~/Library/Caches/com.extraheadroom.headroom`, and the keychain entries. The uninstall dialog in the app shows the full list before you confirm.

If the proxy dies unexpectedly, a watchdog restarts it; after repeated failures it auto-pauses and strips interception so your clients keep working without intervention.

## Bundled tools

| Tool                                                  | What it does                                                                           | Default       |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------- | ------------- |
| [headroom](https://pypi.org/project/headroom-ai/)     | Prompt optimization pipeline (Python)                                                  | Required      |
| [rtk](https://github.com/gglucass/rtk)                | Rewrites Claude Code bash commands to strip noise before it reaches the context window | Opt-in add-on |
| [markitdown](https://github.com/microsoft/markitdown) | Converts PDFs and Office documents to clean Markdown before the agent reads them       | Opt-in add-on |
| ponytail                                              | Nudges the agent toward leaner, less over-engineered code                              | Opt-in add-on |

**Tool inclusion policy:** only tools that run entirely locally, inside Headroom-managed storage, with a stable CLI surface make it in. No cloud dependencies, no host profile mutations. See [`research/tool-compatibility-matrix.md`](research/tool-compatibility-matrix.md).

## Compression benchmarks

Numbers from the [headroom](https://github.com/chopratejas/headroom) open-source library that powers the optimization pipeline, summarized from the current published benchmarks page.

### Current benchmark summary

| Benchmark                      | What it tests                                                         | Result                                                             |
| ------------------------------ | --------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Scrapinghub article extraction | Extract article bodies from 181 HTML pages while removing boilerplate | 0.919 F1, 98.2% recall, **94.9% compression**                      |
| SmartCrusher JSON compression  | Find a critical error in 100 production log entries after compression | 4/4 correct, **87.6% compression**                                 |
| QA accuracy preservation       | Ask the same questions on raw HTML vs. extracted content              | 0.87 F1 vs. 0.85 baseline, 62% exact match vs. 60%                 |
| Multi-tool agent test          | 4-tool agent investigating a memory leak with compressed tool output  | 6,100 vs. 15,662 tokens sent, **76.3% compression**, same findings |

### Benchmark details

| Benchmark             | Setup                                                     | Accuracy                                | Compression |
| --------------------- | --------------------------------------------------------- | --------------------------------------- | ----------- |
| HTML extraction       | Scrapinghub article extraction benchmark, 181 pages       | 0.919 F1, 0.879 precision, 0.982 recall | 94.9%       |
| JSON compression      | 100 production log entries, critical error at position 67 | 4/4 correct answers                     | 87.6%       |
| QA preservation       | SQuAD v2 + HotpotQA on raw HTML vs. extracted content     | +0.02 F1, +2% exact match vs. raw HTML  | —           |
| Multi-tool agent test | Agno agent with 4 tools investigating a memory leak       | Same findings as baseline               | 76.3%       |

### What compresses well vs. what doesn't

| Content type                                         | Typical savings    | Notes                                                             |
| ---------------------------------------------------- | ------------------ | ----------------------------------------------------------------- |
| JSON arrays (search results, API responses, DB rows) | 86–100%            | Primary use case                                                  |
| Structured logs                                      | 82–95%             | Errors and anomalies always preserved                             |
| Agentic conversations (25–50 turns)                  | 56–81%             |                                                                   |
| Plain text / documentation                           | 43–46%             | Cost savings only, adds latency                                   |
| Source code                                          | Mostly passthrough | Code in active messages is protected by default — see limitations |

### Limitations worth knowing

- **Code compression is intentionally conservative.** Code in recent messages (last 4 by default) and any conversation where the user is asking about code (`analyze`, `debug`, `fix`, etc.) is never compressed. The savings from code come from dropping old, no-longer-relevant messages — not from stripping function bodies.
- **Short content is skipped.** Arrays under 5 items and content under 200 tokens pass through unchanged.
- **Text compression (LLMLingua) adds latency.** It requires a ~2 GB model download on first use and doesn't break even on fast models. Useful for cost reduction, not speed.
- **Plain-text RAG results pass through.** Compression targets tool outputs and JSON; plain text in user messages is not compressed.

Full methodology and reproducible benchmarks: [chopratejas/headroom benchmarks](https://chopratejas.github.io/headroom/benchmarks/) · [limitations](https://chopratejas.github.io/headroom/LIMITATIONS/)

## Interesting design decisions

- **Zero host pollution.** Headroom owns its entire dependency tree. Uninstalling the app leaves your shell, your Python, and your PATH exactly as they were (except for the optional `rtk` PATH addition, which is reversible).
- **Rust shell, Python brain.** The Tauri/Rust layer handles tray lifecycle, managed installs, client detection, and update delivery. The optimization work happens in Python, where the headroom ecosystem lives.
- **Client config with rollback.** When Headroom edits a supported client's config (e.g. Claude Code settings), it writes a backup first. Disabling or uninstalling restores the original.
- **Open source shell, private web.** The desktop app is MIT-licensed and open source. The marketing site and account backend live in a separate private repo — so contributors can build and run the full desktop experience without needing backend access.

## Project structure

```
src/              React + Tauri frontend (tray UI, onboarding, savings dashboard)
src-tauri/        Rust backend
  state.rs        Dashboard state and data shaping
  tool_manager.rs Bootstrap, Python runtime, and tool installation
  client_adapters.rs  Client detection and guided setup
  insights.rs     Daily local recommendation engine
research/         Tool vetting artifacts and compatibility matrix
docs/             Architecture notes, release process
```

## macOS release flow

Updates ship outside the App Store via Tauri's built-in updater. The app polls GitHub Releases in the background, prompts before installing, and requests a restart to finish. Both local DMG builds and the GitHub Actions workflow run `./scripts/verify-release.sh` — a failing test blocks the build before anything is published.

See [`docs/macos-release.md`](docs/macos-release.md) for the full release setup.

### Branching and versioning

Use `./scripts/bump-version.sh <version>` to update all four version files at once (`package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `Cargo.toml`). Accepts `X.Y.Z` or `X.Y.Z-rc.N` (leading `v` is stripped).

Two release channels are wired into CI:

- **`main`** — stable channel. Users on the default download get updates from here. Version must be plain `X.Y.Z`. Branch-protected: direct pushes are rejected; changes land via PR only.
- **`staging`** — release candidate channel. Installs via a separate build pointing at the rolling `staging` GitHub release. Version must be `X.Y.Z-rc.N`.

Work happens on `feature/*` branches, which merge into `staging` for testing. Stable promotions land on `main` via a release PR (see below).

**Release candidate flow:**

1. Merge work from a feature branch into `staging`.
2. Bump `package.json` + `src-tauri/tauri.conf.json` to `X.Y.Z-rc.N` (e.g. `0.2.44-rc.1`) and push. `.github/workflows/release-macos-staging.yml` publishes a versioned prerelease tag `vX.Y.Z-rc.N` and mirrors the artifacts to the rolling `staging` release.
3. The staging test machine auto-updates (it has both endpoints baked in and routes itself to the staging endpoint because its installed version has an `-rc` suffix).
4. If something is wrong, bump to `rc.2` and push again. Repeat until the build is good.

**Promoting to stable:**

`main` is branch-protected, so promotions go through a release PR. The merge **must** be a merge commit (not squash, not rebase) so the staging commits — including the rc tag's commit — remain in `main`'s history and the rc-ancestor check passes.

1. From the verified `staging` tip, cut a release branch: `git checkout -b release/X.Y.Z staging`.
2. On that branch, run `./scripts/bump-version.sh X.Y.Z` (strips `-rc.N`), commit, push.
3. Open a PR from `release/X.Y.Z` into `main`.
4. Merge with **"Create a merge commit"**. `.github/workflows/release-macos.yml` triggers on the push to `main` and publishes the stable release.
5. The main workflow **enforces** that a `vX.Y.Z-rc.N` prerelease exists whose commit is an ancestor of the stable (merge) commit. If not, the build fails. This guarantees stable only ships code that was tested via the staging channel.
6. After the stable build is published, the main workflow re-points the rolling `staging` release at the stable DMG. The staging machine receives that as an update, installs it, and — because the new version is plain `X.Y.Z` — automatically switches to the stable endpoint for all future checks.
7. Delete the `release/X.Y.Z` branch.

> Recommended: in **Settings → General → Pull Requests**, leave only "Allow merge commits" enabled and disable squash and rebase merges. The rc-ancestor check already rejects squashed/rebased promotions at build time, but disabling them at the repo level prevents an accidental click that lands a broken bump on `main`.

**Bypassing the rc check:**

For hotfixes where a staging cycle is impractical, include `[skip-rc-check]` in the **PR merge commit message** (the workflow reads the merge commit's message, not the bump commit's). Easiest path: put `[skip-rc-check]` in the PR title or first body line so GitHub includes it in the auto-generated merge commit. Use sparingly — the guard exists to prevent untested stable releases.

## Development

```bash
npm install
npm run tauri dev
```

For a local-first personal build, create `.env` with remote telemetry disabled:

```bash
HEADROOM_LOCAL_ONLY="1"
VITE_HEADROOM_LOCAL_ONLY="1"
VITE_HEADROOM_REMOTE_TELEMETRY="0"
```

For the live auth and pricing flow, add the public-service configuration:

```bash
HEADROOM_ACCOUNT_API_BASE_URL="https://extraheadroom.com/api/v1"
HEADROOM_APTABASE_APP_KEY="REPLACE_WITH_APTABASE_APP_KEY"
VITE_SENTRY_DSN="REPLACE_WITH_SENTRY_DSN"
VITE_CLARITY_PROJECT_ID="REPLACE_WITH_CLARITY_PROJECT_ID"
VITE_HEADROOM_SALES_CONTACT_URL="mailto:hello@extraheadroom.com"
VITE_HEADROOM_CONTACT_FORM_URL="https://extraheadroom.com/contact_request"
```

See [`.env.example`](.env.example) for the complete list, including the optional updater and macOS signing keys used for release builds. Set the same keys as GitHub Actions repository variables for production DMG builds.

Run tests:

```bash
npm run test:all          # frontend + Rust
cargo test --manifest-path src-tauri/Cargo.toml   # Rust only
```

Clean Rust build artifacts (the `src-tauri/target/` directory grows quickly):

```bash
cargo clean --manifest-path src-tauri/Cargo.toml
```

## Dependency pinning

`headroom-ai` is installed from a specific pinned wheel on first run. Automatic upgrades are disabled — the app ships with one known-good version and only changes what it installs when the release artifact itself is updated.

Three constants in [`src-tauri/src/tool_manager.rs`](src-tauri/src/tool_manager.rs) control the pin:

- `HEADROOM_PINNED_VERSION` — the version string (e.g. `"0.8.2"`). Must match the wheel URL.
- `HEADROOM_PINNED_WHEEL_URL` — the exact PyPI wheel URL to download.
- `HEADROOM_PINNED_SHA256` — the wheel's SHA-256, verified after download.

To bump `headroom-ai`: update all three constants together, run the build, and ship a new desktop release. Users pick up the new Python dependency as part of the desktop update flow — there is no separate PyPI check or background upgrade path.

Other bundled components (`rtk`, the Python standalone runtime, the vendor wheels index) are pinned the same way — one version, one checksum, per platform.
