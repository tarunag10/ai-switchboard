# Mac AI Switchboard Implementation Plan

## Purpose

This plan turns the repo audit into a concrete implementation roadmap for making **Mac AI Switchboard** a trustworthy, local-first, open-source control plane for coding agents. The target product should let developers safely manage routing, compression, repo context, and add-ons across Claude Code, Codex, and future agentic coding tools, while preserving a one-click escape hatch and full visibility into every local change.

## Product North Star

**Mac AI Switchboard should become the local-first control plane for AI coding agents.**

It should be trusted because it:

1. Clearly shows what is active.
2. Clearly shows what local files it reads, writes, edits, or deletes.
3. Can undo every managed change.
4. Does not require telemetry, sign-in, hosted pricing, or cloud services for the public free build.
5. Gives agents better context at lower token cost without hiding important detail.
6. Supports Claude Code and Codex deeply before expanding to other tools.
7. Treats unpromoted connectors honestly as gated, guided, or detection-only until they are safe and reversible.

## Guiding Principles

### 1. Trust beats cleverness

Any feature that modifies shell profiles, agent config files, Codex databases, local logs, or repo files must be visible, reversible, and documented.

### 2. Local-first must be provable

It is not enough for telemetry to be disabled in normal flows. Public local builds should have automated checks proving they do not contain account, pricing, telemetry, or third-party analytics endpoints unless explicitly built with remote services enabled.

### 3. Off mode must be boring

Off mode must remove routing hooks, provider overrides, shell blocks, RTK hooks, MarkItDown hooks, and managed instruction blocks without disturbing user-owned configuration.

### 4. Gated support must not be marketed as managed support

Claude Code, Codex, Gemini CLI, OpenCode, Windsurf, and Zed AI are the current managed routing targets. Goose is managed only for the read-only Repo Memory MCP bridge. Cursor is gated with settings discovery and dry-run target/marker preview while native/provider writes remain blocked. Aider, Continue, Amazon Q, Qwen Code, Grok / xAI CLI, and similar tools should remain labelled as guided, detected, or gated until automatic provider setup and cleanup are implemented and tested.

### 5. Context quality is the long-term moat

Headroom and RTK save tokens. Repo Intelligence and Graphify-style context selection should become the reason users keep the app installed.

---

## Roadmap Overview

| Phase |  Timeframe | Theme                                         | Outcome                                                                    |
| ----- | ---------: | --------------------------------------------- | -------------------------------------------------------------------------- |
| P0    |   Days 0-7 | Trust and repo hygiene                        | Remove risky artifacts, lock local-only defaults, fix obvious trust issues |
| P1    |  Days 7-21 | Branding, migration, and public build clarity | App identity is coherent and public builds are clearly free/local          |
| P2    | Days 14-30 | CI, release, and security gates               | Every PR and release passes quality, dependency, and secret checks         |
| P3    | Days 21-45 | Runtime and connector safety                  | Claude/Codex flows are safer, more reversible, and easier to recover       |
| P4    | Days 30-60 | Repo Intelligence v2                          | Context packs become task-aware and relevance-ranked                       |
| P5    | Days 45-75 | Adapter platform                              | Gated connectors move toward manifest-based guided/managed support         |
| P6    | Days 60-90 | Distribution and polish                       | Signed releases, Homebrew, onboarding, docs, diagnostics, and benchmarks   |

The phases overlap intentionally. For example, signed releases can begin before Repo Intelligence v2 is done, but broad distribution should wait until P0-P3 are complete.

---

# P0: Trust and Repo Hygiene

## P0.1 Remove committed local database artifacts

### Problem

A root-level `headroom_memory.db` appears to be present in the repository. Even if empty, a committed database file creates concern because the app stores local state and memory-like artifacts.

### Files to inspect or change

- `headroom_memory.db`
- `.gitignore`
- `src-tauri/src/storage.rs`
- Any scripts that create local DB files

### Implementation steps

1. Inspect the DB before deleting it:

   ```bash
   sqlite3 headroom_memory.db ".tables"
   sqlite3 headroom_memory.db ".schema"
   sqlite3 headroom_memory.db "SELECT name FROM sqlite_master WHERE type='table';"
   ```

2. Check for sensitive local paths, prompts, source snippets, provider metadata, telemetry, auth identifiers, or test data:

   ```bash
   strings headroom_memory.db | head -200
   strings headroom_memory.db | grep -Ei "api|token|key|secret|anthropic|openai|claude|codex|Users/|PRIVATE|BEGIN"
   ```

3. If the DB contains any real local data, purge it from Git history and rotate anything exposed.

4. Remove the file from the working tree:

   ```bash
   git rm headroom_memory.db
   ```

5. Add ignore rules:

   ```gitignore
   *.db
   *.sqlite
   *.sqlite3
   *.sqlite-wal
   *.sqlite-shm
   headroom_memory.db
   memory.db
   ```

6. Add a CI check that fails if database files are committed outside approved fixture directories.

### Acceptance criteria

- `headroom_memory.db` is no longer in the repo.
- `.gitignore` blocks future accidental DB commits.
- CI fails on unexpected committed SQLite/database files.
- If sensitive data was found, Git history has been cleaned and the incident is documented privately.

### Tests

Add a script:

```bash
scripts/check-no-local-artifacts.sh
```

It should fail on:

- `*.db`
- `*.sqlite`
- `*.sqlite3`
- `*.sqlite-wal`
- `*.sqlite-shm`
- `.env`
- `.env.local`
- `.DS_Store`
- `*.log`

Allow test fixtures only under an explicit path such as `fixtures/` or `tests/fixtures/`.

---

## P0.2 Make local-only public builds provable

### Problem

The app has local-only guards, but the public repo still contains remote-service scaffolding: analytics dependencies, Clarity CSP entries, Sentry variables, Aptabase variables, old account API examples, and release workflow requirements.

### Files to inspect or change

- `.env.example`
- `src/lib/localMode.ts`
- `src/lib/analytics.ts`
- `src-tauri/src/local_mode.rs`
- `src-tauri/src/analytics.rs`
- `src-tauri/tauri.conf.json`
- `.github/workflows/release-macos.yml`
- `.github/workflows/release-macos-staging.yml`
- `scripts/check-deployment-readiness.mjs`
- `scripts/check-release-env.mjs`
- `package.json`

### Implementation steps

1. Split build modes:

   - `local-free`: default public build, no telemetry/account/pricing required.
   - `remote-services`: explicit operator/fork build with telemetry/support/account services enabled.

2. Add explicit environment variables:

   ```bash
   HEADROOM_BUILD_FLAVOR="local-free"
   VITE_HEADROOM_BUILD_FLAVOR="local-free"
   HEADROOM_REMOTE_SERVICES="0"
   VITE_HEADROOM_REMOTE_SERVICES="0"
   VITE_HEADROOM_REMOTE_TELEMETRY="0"
   ```

3. Change `.env.example` so local/free is the only default. Keep optional remote telemetry, update, and support keys documented without an account or paid pricing API.

4. Add a bundle scanner:

   ```bash
   npm run check:local-build-privacy
   ```

   It should search `dist/`, `src-tauri/target/release/bundle/`, and built app resources for forbidden strings in local-free builds:

   - `clarity.ms`
   - `aptabase`
   - `sentry.io`
   - inherited paid account endpoints
   - checkout hosts
   - paid pricing endpoint identifiers

5. Remove Clarity from the default CSP. If Clarity is used in a remote-services build, inject CSP conditionally at build time or maintain separate config overlays.

6. Decide whether Sentry/Clarity should remain dependencies in the default package. Prefer removing Clarity entirely from the local-free build path.

7. Add a visible UI panel:

   **Settings → Privacy and Network**

   It should show:

   - Build flavor
   - Local-only mode status
   - Remote services enabled: yes/no
   - Telemetry enabled: yes/no
   - Update checks enabled: yes/no
   - Configured remote destinations
   - Button: copy network/privacy diagnostics

### Acceptance criteria

- Public local-free build does not require account, pricing, Aptabase, Clarity, or Sentry variables.
- Public local-free bundle scan passes.
- Remote-service build still works when explicitly enabled.
- Settings UI shows build and telemetry status plainly.
- Docs explain both build flavors.

### Tests

- Unit tests for `localOnlyModeEnabled()`.
- Unit tests for Rust `local_mode::enabled()`.
- Bundle string scan in CI.
- Snapshot test for Privacy and Network panel.

---

## P0.3 Align README claims with actual support

### Problem

The project aims to support many agentic coding tools, but managed support currently exists for Claude Code and Codex. Other tools are detected, planned, or guided.

### Files to change

- `README.md`
- `docs/install.md`
- `docs/connectors.md`, new
- `src-tauri/src/client_adapters.rs`
- `src/lib/plannedConnectors.ts`
- Any website/marketing copy if present

### Implementation steps

1. Change the headline to:

   ```md
   A local-first Mac menu bar switchboard for Claude Code, Codex, Headroom, RTK, and copyable context packs for other coding agents.
   ```

2. Add a clear support matrix:

   | Tool        | Status         | Automatic routing | RTK support | Repo packs | Notes                                                 |
   | ----------- | -------------- | ----------------: | ----------: | ---------: | ----------------------------------------------------- |
   | Claude Code | Managed        |               Yes |         Yes |        Yes | Reversible config edits                               |
   | Codex       | Managed        |               Yes |     Partial |        Yes | Provider block and bypass handling                    |
   | Gemini CLI  | Managed        |               Yes |          No |        Yes | Managed shell/base-url routing and Off cleanup        |
   | OpenCode    | Managed        |               Yes |          No |        Yes | Managed provider config routing with rollback         |
   | Windsurf    | Managed        |               Yes |          No |        Yes | Managed editor settings routing with rollback         |
   | Zed AI      | Managed        |               Yes |          No |        Yes | Managed settings routing with rollback                |
   | Cursor      | Gated          |                No |          No |        Yes | Settings discovery and dry-run target/marker preview; native writes remain blocked |
   | Aider       | Guided         |                No |          No |        Yes | Sidecar/readiness lifecycle, native writes blocked    |
   | Continue    | Guided         |                No |          No |        Yes | Sidecar/readiness lifecycle, native writes blocked    |
   | Goose       | Managed MCP    |               Yes |          No |        Yes | Read-only Repo Memory MCP bridge, native writes blocked |
   | Grok / xAI  | Guided         |                No |          No |        Yes | Sidecar/readiness lifecycle, native writes blocked    |
   | Qwen Code   | Guided         |                No |          No |        Yes | Sidecar/readiness lifecycle, native writes blocked    |
   | Amazon Q    | Guided         |                No |          No |        Yes | Sidecar/readiness lifecycle, native writes blocked    |

3. Use consistent labels everywhere:

   - Managed
   - Guided
   - Detected
   - Planned
   - Unsupported

4. Add `docs/connectors.md` and link it from README.

5. Make the UI use the same labels.

### Acceptance criteria

- README and docs identify only Claude Code, Codex, Gemini CLI, OpenCode, Windsurf, and Zed AI as fully managed.
- Every gated connector has a safe manual workflow and automation gates.
- UI and docs use the same labels.

---

# P1: Branding, Migration, and Public Identity

## P1.1 Complete Headroom → Mac AI Switchboard product identity

### Problem

The repo and app show mixed identity:

- Product name: Mac AI Switchboard
- Rust package name: `headroom-desktop`
- Runtime storage: `Headroom`
- Managed markers: `headroom:`
- Some docs still include original Headroom text

This can confuse users and make the app look like a half-fork.

### Files to inspect or change

- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`
- `src-tauri/src/storage.rs`
- `src-tauri/src/client_adapters.rs`
- `src-tauri/src/keychain.rs`
- `src-tauri/src/lib.rs`
- `README.md`
- `docs/*.md`
- `.github/workflows/*.yml`
- `scripts/*.mjs`

### Implementation steps

1. Rename package metadata:

   - npm package name: `mac-ai-switchboard`
   - Rust package name: `mac-ai-switchboard`
   - Rust lib name: `mac_ai_switchboard_lib`
   - Description: `Local-first Mac switchboard for AI coding-agent routing, token savings, and repo context packs.`
   - Author: real maintainer/project name, not `Codex`

2. Create constants for legacy names:

   ```rust
   const LEGACY_STORAGE_DIR_NAME: &str = "Headroom";
   const APP_STORAGE_DIR_NAME: &str = "Mac AI Switchboard";
   const LEGACY_MARKER_PREFIX: &str = "headroom";
   const MARKER_PREFIX: &str = "switchboard";
   ```

3. Add migration state file:

   ```text
   ~/Library/Application Support/Mac AI Switchboard/config/migrations.json
   ```

4. Implement storage migration:

   - On first launch, if new storage does not exist and old Headroom storage exists, copy old to new temp path.
   - Verify critical files copied.
   - Rename temp path to final path atomically if possible.
   - Keep old storage for one release as backup.
   - Record migration metadata.
   - Allow rollback if startup fails.

5. Continue reading legacy storage as fallback for existing users until a future major version.

6. For managed config markers:

   - New writes use `# >>> switchboard:<id> >>>`.
   - Cleanup removes both `headroom:` and `switchboard:` markers.
   - Doctor reports legacy markers and offers migration.

7. Update docs:

   - Remove “original Headroom documentation continues below” from README.
   - Add “Headroom is the upstream optimization engine” section.
   - Add “Why some legacy paths may exist” migration note.

### Acceptance criteria

- New installs no longer create primary storage under `Headroom`, unless intentionally in compatibility mode.
- Existing users migrate without losing runtime receipts, logs, or backups.
- Cleanup handles both old and new markers.
- User-facing docs use Mac AI Switchboard consistently.
- Any remaining Headroom references are clearly about the upstream optimizer.

### Tests

- Unit test: fresh install chooses new storage path.
- Unit test: old Headroom storage migrates to new storage.
- Unit test: corrupted migration leaves old storage intact.
- Unit test: cleanup removes both legacy and new marker blocks.
- Smoke test: upgrade from old storage path to new path.

---

## P1.2 Update uninstall and cleanup paths

### Problem

Uninstall cleanup currently includes old Headroom bundle IDs and paths. Current app identifiers and macOS storage paths should also be removed.

### Files to change

- `src-tauri/src/client_adapters.rs`
- `src-tauri/src/lib.rs`
- `src/lib/uninstallDisclosure.ts`
- `docs/install.md`
- `docs/recovery.md`, new

### Implementation steps

1. Expand cleanup targets to include current app identifiers and any explicitly approved future official org bundle ID.

2. Cleanup should cover:

   - Preferences
   - Caches
   - WebKit data
   - HTTPStorages
   - Saved Application State
   - LaunchAgents
   - Keychain entries
   - Managed shell blocks
   - Claude settings/hooks
   - Codex provider blocks
   - MarkItDown/Caveman/Ponytail managed blocks
   - App support storage, with explicit user confirmation

3. Add “dry run uninstall”:

   ```bash
   mac-ai-switchboard --uninstall-dry-run
   ```

   Or a UI equivalent: **Settings → Uninstall → Preview files**.

4. Add exportable uninstall report.

### Acceptance criteria

- Uninstall disclosure exactly matches code cleanup targets.
- User can preview what will be removed.
- Cleanup covers legacy and current bundle IDs.
- No user-owned config is deleted except managed blocks/backups with explicit confirmation.

---

# P2: CI, Security, and Release Gates

## P2.1 Add PR CI workflow

### Problem

Release workflows are strong, but a normal PR/push CI workflow should catch issues before release branches.

### Files to add

- `.github/workflows/ci.yml`

### Implementation steps

Add CI with:

```yaml
name: CI

on:
  pull_request:
  push:
    branches:
      - main
      - staging
      - "feature/**"

permissions:
  contents: read

jobs:
  test:
    runs-on: macos-latest
    timeout-minutes: 35
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
      - uses: dtolnay/rust-toolchain@stable
      - run: npm ci
      - run: npm run build
      - run: npm run test:coverage
      - run: npm run fmt:desktop
      - run: cargo test --manifest-path src-tauri/Cargo.toml
      - run: cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
      - run: npm run check:connectors
      - run: npm run check:governance
      - run: npm run check:local-build-privacy
      - run: scripts/check-no-local-artifacts.sh
```

### Acceptance criteria

- Every PR runs frontend build/tests and Rust tests.
- CI blocks formatting, clippy, connector parity, local privacy, and local artifact failures.
- Branch protection requires CI before merge.

---

## P2.2 Add secret and dependency scanning

### Files to add or change

- `.github/workflows/security.yml`
- `deny.toml`
- `.gitleaks.toml`
- `package.json`

### Implementation steps

1. Add Gitleaks:

   ```yaml
   - uses: gitleaks/gitleaks-action@v2
   ```

2. Add cargo audit:

   ```bash
   cargo install cargo-audit
   cargo audit --manifest-path src-tauri/Cargo.toml
   ```

3. Add cargo deny:

   ```bash
   cargo install cargo-deny
   cargo deny --manifest-path src-tauri/Cargo.toml check
   ```

4. Add npm production audit:

   ```bash
   npm audit --omit=dev --audit-level=high
   ```

5. Add GitHub dependency review for PRs.

6. Add CodeQL for TypeScript and Rust.

### Acceptance criteria

- CI fails on committed secrets.
- CI fails on high-risk production dependency vulnerabilities unless explicitly waived.
- Dependency license policy is defined.
- Security checks run separately from release workflow.

---

## P2.3 Harden release artifacts

### Files to change

- `.github/workflows/release-macos.yml`
- `.github/workflows/release-macos-staging.yml`
- `scripts/verify-release.sh`
- `docs/macos-release.md`

### Implementation steps

1. Add checksum generation:

   ```bash
   shasum -a 256 *.dmg > SHA256SUMS.txt
   ```

2. Add SBOM generation:

   Options:

   - `cargo auditable`
   - `cyclonedx-npm`
   - `syft packages . -o cyclonedx-json`

3. Add GitHub artifact attestation if available:

   ```yaml
   permissions:
     id-token: write
     attestations: write
   ```

4. Upload:

   - DMG
   - `latest.json`
   - `SHA256SUMS.txt`
   - SBOM
   - release notes

5. Add a release-notes checklist:

   - Supported managed clients
   - Build flavor
   - Remote destinations enabled
   - Known issues
   - Upgrade path
   - Manual recovery path

### Acceptance criteria

- Every release has checksum and SBOM.
- Release notes show build flavor and network behavior.
- Update JSON is uploaded after artifacts.
- Notarization status is verified before release publication.

---

# P3: Runtime, Connector, and Logging Safety

## P3.1 Add managed footprint inventory

### Goal

Users should be able to see exactly what the app currently manages.

### Files to change

- `src-tauri/src/client_adapters.rs`
- `src-tauri/src/tool_manager.rs`
- `src-tauri/src/models.rs`
- `src-tauri/src/lib.rs`
- `src/components/SwitchboardDoctorPanel.tsx`
- `src/components/SwitchboardPanel.tsx`
- `src/lib/managedChanges.ts`

### Data model

Add:

```rust
pub struct ManagedFootprintItem {
    pub id: String,
    pub category: String,
    pub path: String,
    pub exists: bool,
    pub managed: bool,
    pub action: String,
    pub reversible: bool,
    pub backup_paths: Vec<String>,
    pub notes: Vec<String>,
}

pub struct ManagedFootprintReport {
    pub generated_at: DateTime<Utc>,
    pub items: Vec<ManagedFootprintItem>,
}
```

### Implementation steps

1. Add backend command:

   ```rust
   #[tauri::command]
   fn get_managed_footprint() -> Result<ManagedFootprintReport, String>
   ```

2. Include:

   - Shell profile blocks
   - Claude settings env
   - Claude hooks
   - Claude `CLAUDE.md` managed blocks
   - Codex config provider blocks
   - Codex `AGENTS.md` managed blocks
   - RTK binary and PATH block
   - MarkItDown hook/shim/cache
   - Caveman blocks
   - Ponytail config if installed
   - LaunchAgent
   - Keychain service names, without values
   - App support directories
   - Logs and local databases

3. Add UI:

   - **Doctor → Managed footprint**
   - Filter by category
   - Copy report
   - Open containing folder
   - Repair/remove action where safe

### Acceptance criteria

- The app can produce a complete footprint report.
- The report is redacted and safe to share.
- Uninstall disclosure is generated from or checked against the same inventory.

---

## P3.2 Make raw message logging explicitly dangerous and safely managed

### Problem

Raw request/compressed messages are useful but can contain prompts, source code, secrets, and paths.

### Files to change

- `src-tauri/src/models.rs`
- `src-tauri/src/proxy_intercept.rs`
- `src/components/ActivityFeed.tsx`
- `src/lib/trayHelpers.ts`
- `PRIVACY.md`
- `docs/telemetry.md`, new
- `docs/recovery.md`, new

### Implementation steps

1. Add a setting:

   ```json
   {
     "fullMessageLogging": false,
     "fullMessageLoggingExpiresAt": null,
     "messageLogRetentionHours": 24
   }
   ```

2. UI flow to enable:

   - Warning modal
   - List of data types that may be stored
   - Expiration selector: 15 minutes, 1 hour, 24 hours
   - Confirm checkbox

3. Redact before display/export:

   Patterns:

   - `sk-ant-...`
   - `sk-proj-...`
   - `ghp_...`
   - `github_pat_...`
   - `BEGIN PRIVATE KEY`
   - `AWS_SECRET_ACCESS_KEY`
   - `ANTHROPIC_API_KEY`
   - `OPENAI_API_KEY`
   - `Authorization: Bearer ...`
   - `.p8`, `.pem`, `.p12` content snippets

4. Add one-click purge:

   ```rust
   #[tauri::command]
   fn purge_message_logs() -> Result<PurgeResult, String>
   ```

5. Make analytics/Sentry paths reject raw message fields.

6. Add test fixtures containing fake secrets and verify redaction.

### Acceptance criteria

- Full message logging is off by default.
- It auto-expires.
- User can purge logs.
- Redaction is applied before display/export.
- No raw messages are sent to telemetry or Sentry.

---

## P3.3 Make Codex SQLite retagging opt-in and backed up

### Problem

The app updates Codex SQLite stores to keep history visible across provider changes. This is useful but risky because it modifies another tool’s private database.

### Files to change

- `src-tauri/src/client_adapters.rs`
- `src-tauri/src/models.rs`
- `src/components/SwitchboardDoctorPanel.tsx`
- `docs/codex-compression-troubleshooting.md`
- `docs/recovery.md`

### Implementation steps

1. Add setting:

   ```json
   {
     "codexThreadRetagging": "ask"
   }
   ```

   Values:

   - `ask`
   - `enabled`
   - `disabled`

2. On first retag need, show:

   - What database will be edited
   - Why retagging exists
   - Where backup will be written
   - How to restore

3. Backup before write:

   ```text
   ~/.codex/sqlite/state_5.sqlite.switchboard-backup-20260629T120000Z
   ```

4. Use transaction:

   ```sql
   BEGIN IMMEDIATE;
   UPDATE threads SET model_provider = ?2 WHERE model_provider = ?1;
   COMMIT;
   ```

5. Add schema guard:

   - Verify `threads` table exists.
   - Verify `model_provider` column exists.
   - Verify known version or require explicit override.

6. Add restore command:

   ```rust
   #[tauri::command]
   fn restore_codex_thread_db_backup(path: String) -> Result<(), String>
   ```

7. Add Doctor issue when retagging is disabled but history split is likely.

### Acceptance criteria

- No Codex DB write happens without opt-in or prior consent.
- Every write has a backup.
- Unknown schemas are skipped or require override.
- Doctor can restore from backup.

### Tests

- Fake DB with `threads` table retags correctly.
- Missing table is no-op.
- Missing column is no-op with warning.
- Locked DB fails safely.
- Unknown version prompts/skips.
- Restore returns DB to prior state.

---

## P3.4 Add proxy local-auth guard

### Problem

The local proxy listens on localhost. Other local processes can potentially send requests to it. Localhost is not a security boundary.

### Files to change

- `src-tauri/src/tool_manager.rs`
- `src-tauri/src/proxy_intercept.rs`
- Headroom proxy launch/config integration
- Client adapter env/config writes

### Implementation options

#### Option A: Per-session bearer token

1. Generate random token at proxy start.
2. Store in app memory or Keychain.
3. Configure managed clients with local proxy URL plus expected header if client supports it.
4. For clients that cannot set headers, use a provider-specific proxy shim.

#### Option B: Unix domain socket

1. Bind proxy to a user-owned Unix socket.
2. Expose HTTP bridge only where needed.
3. Prefer socket permissions over open localhost port.

#### Option C: Keep localhost but add scope controls

If upstream Headroom cannot support auth yet:

- Bind only `127.0.0.1`, never `0.0.0.0`.
- Add request-shape validation.
- Add warning in threat model.
- Add future upstream issue for auth.

### Acceptance criteria

- Threat model documents localhost proxy risk.
- Managed traffic is protected where technically possible.
- Doctor shows proxy bind address and auth status.

---

# P4: Repo Intelligence v2

## P4.1 Replace smallest-file pack selection with relevance ranking

### Problem

Current context packs sort candidate files by estimated tokens and truncate. That saves tokens, but may choose small irrelevant files over central files.

### Files to change

- `src-tauri/src/repo_intelligence.rs`
- `src-tauri/src/models.rs`
- `src/lib/repoIntelligence.ts`
- Repo Intelligence UI components

### New ranking inputs

For each file, compute:

- Role score: source/test/config/docs/lockfile
- Entrypoint score
- Dependency hub score
- Import fan-in
- Import fan-out
- Test proximity
- Directory centrality
- Recent Git activity
- File name match to task query
- Symbol match to task query
- Token cost penalty
- Secret/generated/binary exclusion

### Data model

```rust
pub struct RepoFileRank {
    pub path: String,
    pub score: f64,
    pub estimated_tokens: u64,
    pub reasons: Vec<String>,
    pub risks: Vec<String>,
}

pub struct RepoTaskContextPack {
    pub id: String,
    pub task: String,
    pub budget_tokens: u64,
    pub files: Vec<RepoFileRank>,
    pub tests: Vec<RepoFileRank>,
    pub commands: Vec<String>,
    pub omitted: Vec<RepoFileRank>,
}
```

### Implementation steps

1. Keep existing generic packs for backward compatibility.
2. Add task-aware pack generation:

   ```rust
   summarize_repo_for_task(repo_path, task_query, budget_tokens)
   ```

3. Add scoring function:

   ```rust
   fn rank_file(signal: &RepoFileSignal, graph: &RepoGraphSummary, task: &TaskQuery) -> RepoFileRank
   ```

4. Select files using score per token:

   - Always include direct matches and entrypoints above threshold.
   - Always include nearest tests for selected source files.
   - Respect total budget.
   - Avoid selecting tiny irrelevant files merely because they are cheap.

5. Add “why included” to every file.

6. Add UI:

   - Task input box
   - Token budget selector
   - Generated pack preview
   - Copy as Markdown
   - Copy as JSON
   - Send to agent handoff profile

### Acceptance criteria

- Default packs still work.
- Task-aware packs prefer relevant central files over smallest files.
- Every included file has reasons.
- Tests are included when likely relevant.
- Secret-like files remain excluded by default.

### Tests

Create synthetic repos:

- React app with component and test.
- Rust CLI with `main.rs`, modules, tests, Cargo files.
- Python package with `pyproject.toml`, modules, tests.
- Monorepo with multiple packages.

For each, assert the pack includes relevant files and excludes secrets/build output.

---

## P4.2 Add AST-backed import and symbol graph

### Problem

Symbol extraction is currently line-based and symbol edges are weak. The repo already includes tree-sitter dependencies, so use them.

### Files to change

- `src-tauri/src/repo_intelligence.rs`
- New module: `src-tauri/src/repo_graph/`
- `src-tauri/Cargo.toml`
- `src-tauri/src/models.rs`

### Implementation steps

1. Create modules:

   ```text
   src-tauri/src/repo_graph/mod.rs
   src-tauri/src/repo_graph/languages.rs
   src-tauri/src/repo_graph/imports.rs
   src-tauri/src/repo_graph/symbols.rs
   src-tauri/src/repo_graph/resolve.rs
   src-tauri/src/repo_graph/tests.rs
   ```

2. Support languages in this order:

   - TypeScript/TSX
   - JavaScript/JSX
   - Rust
   - Python
   - Go
   - Shell

3. Extract:

   - Imports
   - Exports
   - Function definitions
   - Classes/structs/enums/traits/interfaces
   - Constants
   - Test declarations
   - Route definitions where easy

4. Resolve local imports:

   - Relative TS/JS imports
   - Rust `mod` paths
   - Python relative imports
   - Package root hints from config files

5. Produce graph edges:

   - `imports`
   - `defines_symbol`
   - `references_symbol`
   - `test_targets_source`
   - `entrypoint_uses_config`

6. Keep caps per edge type, not one global cap.

### Acceptance criteria

- Graph quality is meaningfully better than path heuristics.
- Edge reasons are specific.
- Symbol locations include file and line.
- Large repos stay bounded and fast.

### Tests

- Fixture files for each supported language.
- Import resolution tests.
- Symbol extraction tests.
- Snapshot graph tests.

---

## P4.3 Add MCP context-pack API

### Goal

Agents should be able to request compact repo context packs directly instead of requiring manual copy/paste.

### Files to add or change

- `src-tauri/src/repo_intelligence.rs`
- `src-tauri/src/tool_manager.rs`
- New module: `src-tauri/src/mcp_server.rs`
- `docs/mcp.md`, new
- `README.md`

### MCP tools

Expose tools such as:

```json
{
  "name": "switchboard.list_context_packs",
  "description": "List available context packs for the selected repo."
}
```

```json
{
  "name": "switchboard.build_context_pack",
  "input_schema": {
    "repo_path": "string",
    "task": "string",
    "budget_tokens": "number"
  }
}
```

```json
{
  "name": "switchboard.get_repo_graph_summary",
  "input_schema": {
    "repo_path": "string"
  }
}
```

### Guardrails

- Read-only by default.
- Never include secret-like files by default.
- Require explicit repo path selection/approval.
- Log what repo was indexed, not file contents.
- Let user clear saved index.

### Acceptance criteria

- Claude Code or compatible MCP clients can discover context packs.
- Codex/OpenCode/Aider can consume JSON/Markdown handoff even if MCP support differs.
- User can disable MCP handoff.

---

# P5: Adapter Platform and Connector Expansion

## P5.1 Move connector definitions to manifests

### Problem

Gated connectors are hardcoded in Rust and frontend helpers. This will become hard to maintain.

### Files to add or change

- `src-tauri/src/client_adapters.rs`
- `src/lib/plannedConnectors.ts`
- New directory: `connectors/`
- New schema: `schemas/connector.schema.json`

### Manifest example

```json
{
  "id": "opencode",
  "name": "OpenCode",
  "category": "cli",
  "support_status": "planned",
  "detection": {
    "binaries": ["opencode", "open-code"],
    "paths": ["~/.opencode", "~/.config/opencode"]
  },
  "config": {
    "locations": ["~/.opencode", "~/.config/opencode"],
    "forbidden_reads": ["*token*", "*secret*", "auth.json"]
  },
  "automation_gates": [
    "Identify active provider config path without guessing.",
    "Back up provider settings before edits.",
    "Verify Off mode restores exact previous config."
  ],
  "manual_workflow": [
    "Confirm OpenCode is installed.",
    "Use RTK-only mode for noisy shell output.",
    "Use Repo Intelligence handoff packs until managed routing ships."
  ]
}
```

### Implementation steps

1. Add JSON schema.
2. Move gated connector metadata to `connectors/*.json`.
3. Generate Rust and TypeScript constants at build time, or load from embedded JSON.
4. Add parity check to ensure UI and backend see the same data.
5. Keep managed Claude/Codex adapters in Rust code for now, but represent their metadata with manifests too.

### Acceptance criteria

- One source of truth for connector metadata.
- Adding a gated connector does not require editing multiple files.
- CI validates connector manifests.

---

## P5.2 Define adapter lifecycle contract

### Goal

Every managed connector should implement the same safe lifecycle.

### Adapter trait concept

```rust
trait ClientAdapter {
    fn id(&self) -> &'static str;
    fn detect(&self) -> ClientStatus;
    fn plan_setup(&self) -> Result<SetupPlan>;
    fn apply_setup(&self, plan: SetupPlan) -> Result<ClientSetupResult>;
    fn verify_setup(&self) -> Result<ClientSetupVerification>;
    fn disable(&self) -> Result<DisableResult>;
    fn repair(&self, issue_id: &str) -> Result<RepairResult>;
    fn footprint(&self) -> Vec<ManagedFootprintItem>;
}
```

### Lifecycle requirements

Before any adapter becomes managed, it must support:

- Detection without reading secrets.
- Setup dry run.
- Timestamped backup.
- Idempotent apply.
- Verification.
- Off mode cleanup.
- Uninstall cleanup.
- Doctor repair.
- Fixture tests.
- Manual fallback docs.

### Acceptance criteria

- Claude and Codex are refactored toward the lifecycle contract.
- Gated adapters cannot become “managed” unless they pass lifecycle tests.

---

## P5.3 Connector expansion order

### Recommended order

1. **Cursor**
   - Editor settings are already detected.
   - Promote profile-aware settings writes only after parse, dry-run diff, backup, verification, rollback, and Off cleanup are proven.

2. **Continue**
   - Multi-provider config needs careful unmanaged-config preservation.
   - Good candidate for the next read-only-to-managed promotion after editor settings parsing is safer.

3. **Goose**
   - Agent/MCP-friendly.
   - Strong fit for Repo Intelligence handoff.

4. **Aider**
   - CLI-first.
   - Common config files.
   - Good candidate for context packs and wrapper-based env routing.

5. **Grok / xAI CLI, Qwen Code, and Amazon Q**
   - Keep account, credential, and model guardrails explicit before native writes.
   - Use sidecar/readiness dossiers until provider-specific safe mutation is proven.

Managed reference paths:

- **OpenCode** is the CLI/provider-config reference path.
- **Gemini CLI** is the shell/base-url reference path.
- **Windsurf** and **Zed AI** are the editor-settings reference paths.

### Acceptance criteria for each new managed connector

- Can be detected.
- Can be enabled without losing user config.
- Creates backups before edits.
- Can verify routing.
- Off mode restores previous state.
- Doctor can repair drift.
- Docs explain manual recovery.

---

# P6: Distribution, UX, Docs, and Benchmarks

## P6.1 Publish signed/notarized releases

### Files to change

- `.github/workflows/release-macos.yml`
- `docs/install.md`
- `docs/macos-release.md`
- `README.md`

### Implementation steps

1. Produce Apple Silicon DMG first.
2. Decide on Intel support:

   - Apple Silicon only, clearly stated; or
   - separate `x86_64-apple-darwin`; or
   - universal binary.

3. Add release assets:

   - DMG
   - `latest.json`
   - `SHA256SUMS.txt`
   - SBOM
   - release notes

4. Add Homebrew cask:

   ```ruby
   cask "mac-ai-switchboard" do
     version "0.5.1"
     sha256 "..."
     url "https://github.com/tarunag10/mac-ai-switchboard/releases/download/v#{version}/Mac-AI-Switchboard_#{version}_mac.dmg"
     name "Mac AI Switchboard"
     desc "Local-first Mac switchboard for AI coding agents"
     homepage "https://github.com/tarunag10/mac-ai-switchboard"
     app "Mac AI Switchboard.app"
   end
   ```

5. Add installation verification command:

   ```bash
   codesign --verify --deep --strict /Applications/Mac\ AI\ Switchboard.app
   spctl --assess --type execute /Applications/Mac\ AI\ Switchboard.app
   ```

### Acceptance criteria

- Users can download and install a notarized DMG.
- Release page includes checksum and SBOM.
- Install docs are accurate for non-maintainers.
- Homebrew cask works.

---

## P6.2 Improve first-run onboarding

### Goals

First-run should answer:

- What does this app do?
- What does it install?
- What files can it edit?
- What leaves my machine?
- What happens if I turn it off?

### Implementation steps

Add first-run cards:

1. **Local-first, not offline-only**
   - Provider model calls still go to Claude/OpenAI/etc.

2. **What Switchboard can manage**
   - Claude Code
   - Codex
   - RTK
   - MarkItDown
   - Repo Intelligence

3. **What it may write**
   - App support storage
   - Shell profile managed blocks
   - Claude settings/hooks
   - Codex config

4. **Off mode and uninstall**
   - Show that routing can be removed.

5. **Privacy and network**
   - Show local-only/telemetry status.

6. **Choose initial mode**
   - Off
   - RTK only
   - Headroom only
   - Full optimization

### Acceptance criteria

- New users see a clear footprint before enabling managed edits.
- Onboarding supports skip/default Off mode.
- Users can copy the footprint list.

---

## P6.3 Add recovery docs and emergency reset commands

### Files to add

- `docs/recovery.md`
- CLI or Tauri command handlers for reset actions

### Commands to implement

```bash
mac-ai-switchboard --print-managed-footprint
mac-ai-switchboard --doctor-reset
mac-ai-switchboard --disable-routing
mac-ai-switchboard --disable-rtk
mac-ai-switchboard --disable-markitdown
mac-ai-switchboard --disable-caveman
mac-ai-switchboard --uninstall-managed-config
mac-ai-switchboard --purge-logs
```

If a standalone CLI is not available yet, provide shell scripts:

```bash
scripts/recovery/disable-routing.sh
scripts/recovery/remove-managed-blocks.sh
scripts/recovery/purge-local-logs.sh
```

### Acceptance criteria

- User can recover even if the GUI will not launch.
- Recovery scripts are idempotent.
- Scripts print what they changed.
- Scripts do not remove user-owned config outside managed blocks.

---

## P6.4 Add reproducible benchmark suite

### Goal

Show token savings and quality preservation using reproducible public fixtures.

### Files to add

- `benchmarks/`
- `docs/benchmarks.md`
- `scripts/run-benchmarks.mjs`

### Benchmark categories

1. **Shell output compression**
   - noisy test logs
   - build logs
   - grep/search output
   - stack traces

2. **Repo context packs**
   - full repo scan vs implementation pack
   - full repo scan vs verification pack
   - task-aware pack vs generic pack

3. **Document conversion**
   - PDF to Markdown
   - Office docs to Markdown
   - token savings and content retention

4. **Agent success proxy tests**
   - identify failing test cause
   - locate implementation file
   - produce patch plan
   - explain build failure

### Metrics

- Original tokens
- Optimized tokens
- Percent saved
- Latency overhead
- Relevant fact retention
- Wrong omission rate
- Agent answer quality rubric

### Acceptance criteria

- Benchmarks can be run locally without secrets for static tests.
- Any LLM-judged benchmark is optional and clearly labelled.
- Docs do not overclaim savings where quality drops.

---

# Implementation Sequence by Pull Request

## PR 1: Remove local artifacts and add guard

- Remove `headroom_memory.db`.
- Update `.gitignore`.
- Add `scripts/check-no-local-artifacts.sh`.
- Add CI job for artifact check.

## PR 2: Local-free build cleanup

- Keep `.env.example` local-free by default.
- Add `check:local-build-privacy`.
- Remove remote-service requirements from local-free release path.
- Update README local build instructions.

## PR 3: Support matrix honesty

- Add `docs/connectors.md`.
- Update README support table.
- Align UI labels: Managed, Guided, Detected, Planned.

## PR 4: Branding metadata cleanup

- Rename npm/Rust metadata.
- Update descriptions and authors.
- Add legacy constants.
- Update docs to explain Headroom as upstream engine.

## PR 5: Storage and marker migration

- New storage path.
- Legacy migration logic.
- New `switchboard:` markers.
- Cleanup supports both prefixes.
- Tests for migration and cleanup.

## PR 6: PR CI and security checks

- Add `ci.yml`.
- Add secret scanning.
- Add dependency scanning.
- Add CodeQL.
- Add branch protection documentation.

## PR 7: Managed footprint report

- Add Rust data model and command.
- Add frontend Doctor panel.
- Wire uninstall disclosure to same inventory.

## PR 8: Raw message logging hardening

- Add setting, warning modal, expiry, purge.
- Add redaction.
- Add tests with fake secrets.

## PR 9: Codex DB retagging hardening

- Add opt-in setting.
- Add backups and transaction.
- Add schema validation.
- Add restore action and tests.

## PR 10: Release artifact hardening

- Add checksums.
- Add SBOM.
- Add release note template.
- Add notarization verification.

## PR 11: Repo Intelligence relevance ranking

- Add ranking model.
- Add task-aware context pack.
- Add tests with synthetic repos.

## PR 12: AST graph foundation

- Add repo graph modules.
- Add TypeScript/JavaScript/Rust/Python extraction.
- Replace path-only symbol edges.

## PR 13: MCP context-pack API

- Add MCP server/tool exposure.
- Add docs and safety controls.
- Add client setup guide.

## PR 14: Adapter manifest platform

- Add connector schema.
- Move gated connectors to manifests.
- Generate backend/frontend registry.

## PR 15: First signed public release

- Publish notarized Apple Silicon DMG.
- Add checksum/SBOM.
- Update install docs.
- Announce managed support scope clearly.

---

# Definition of Done

A feature is done only when all relevant items are true:

- It is documented.
- It has tests.
- It is visible in Doctor or Settings if it affects local state.
- It has an Off/uninstall cleanup path if it writes anything.
- It has a manual recovery path if it can break agent routing.
- It does not send data remotely in local-free builds.
- It has a clear user-facing status label.
- It preserves user config and unknown fields.
- It creates backups before editing user-owned config.
- It is covered by release readiness checks if it affects distribution.

---

# Risk Register

| Risk                                    | Severity | Mitigation                                                              |
| --------------------------------------- | -------: | ----------------------------------------------------------------------- |
| Accidental committed local data         | Critical | Remove DB, add artifact guard, secret scanning                          |
| Confusing Headroom/Switchboard identity |     High | Complete rebrand, migrate paths, document legacy compatibility          |
| Telemetry distrust                      |     High | Local-free build scanner, privacy panel, separate remote-services build |
| Raw prompt logs leak secrets            |     High | Off by default, warnings, redaction, expiry, purge                      |
| Codex DB modification breaks history    |     High | Opt-in, backup, transaction, schema guard, restore action               |
| Off mode leaves hooks behind            |     High | Managed footprint inventory, fixture tests, Doctor repair               |
| Gated connectors overmarketed           |   Medium | Honest labels, support matrix, automation gates                         |
| Repo packs omit important files         |   Medium | Relevance ranking, AST graph, test proximity                            |
| Release artifacts not trusted           |   Medium | Signed/notarized DMG, checksums, SBOM, Homebrew cask                    |
| Long-running runtime install fails      |   Medium | Better Doctor repair, logs, preflight checks, recovery scripts          |

---

# Success Metrics

## Trust and safety

- Zero unexpected network destinations in local-free build.
- Zero committed local DB/log/env artifacts.
- 100% of managed edits are listed in footprint report.
- 100% of managed edits have cleanup path.
- 100% of config edits create backups.

## Product usage

- First-run completion rate.
- Successful runtime install rate.
- Successful Claude/Codex setup verification rate.
- Off mode cleanup success rate.
- Doctor repair success rate.

## Token/context value

- Average RTK saved tokens per command.
- Average proxy compression savings.
- Repo context pack savings vs full scan.
- Task-aware pack inclusion accuracy.
- User copy/use rate for context packs.

## Release quality

- CI pass rate.
- Release smoke pass rate.
- Crash-free sessions.
- Update success rate.
- Uninstall success rate.

---

# Suggested Issue Labels

Use labels to keep the roadmap organized:

- `p0-trust`
- `p1-branding`
- `p2-ci-security`
- `p3-runtime-safety`
- `p4-repo-intelligence`
- `p5-connectors`
- `p6-release-polish`
- `docs`
- `tests`
- `privacy`
- `doctor`
- `codex`
- `claude-code`
- `rtk`
- `markitdown`
- `mcp`

---

# Recommended Milestone Structure

## Milestone 0.6.0: Public Trust Baseline

- Remove local DB artifact.
- Add local-free build scanner.
- Add PR CI.
- Add connector support matrix.
- Add managed footprint report.

## Milestone 0.7.0: Safe Release Candidate

- Complete rebrand metadata.
- Add storage/marker migration.
- Add raw log redaction/purge.
- Add Codex retag backup/restore.
- Add signed/notarized release pipeline with checksum/SBOM.

## Milestone 0.8.0: Repo Intelligence v2

- Relevance-ranked packs.
- Task-aware pack generation.
- AST-backed imports/symbols for core languages.
- Better test/source relationship graph.

## Milestone 0.9.0: Agent Handoff Platform

- MCP context-pack API.
- Connector manifests.
- Guided handoffs for Cursor, Windsurf, OpenCode, Aider, Goose, Gemini CLI.

## Milestone 1.0.0: Stable Public Release

- Signed/notarized release.
- Homebrew cask.
- Full docs.
- Recovery scripts.
- Benchmarks.
- Clear managed support for Claude Code and Codex.
- Stable local-free privacy guarantee.
