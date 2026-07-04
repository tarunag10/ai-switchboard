# Agent Memory Implementation Plan

## Goal

Add Codex Memory, Claude Memory, and a unified Agent Memory Inspector to Mac AI Switchboard so users can see, compact, deduplicate, and safely manage the instructions and memory files that agents use before a session starts.

The product should treat memory as a local, inspectable optimization layer, not as an invisible automatic write path. The first release should detect and preview memory state, explain token cost and risk, then offer one-click apply with rollback only after the user approves.

## Product Principles

- Local-first by default. Memory files, previews, receipts, and rollback data stay on the Mac.
- Preview before mutation. No automatic writes to Codex, Claude, or repo memory files in the first safe slice.
- Repo-scoped first. Prefer project memory over broad global memory unless the user explicitly chooses global.
- Secret-safe. Scan memory candidates before writing or injecting.
- Agent-specific but unified. Codex, Claude, AGENTS.md, Repo Intelligence packs, and Repo Memory MCP should stay distinct while being shown in one inspector.
- Token-visible. Every memory source should show estimated tokens, duplication, freshness, and injection order.
- Reversible. Every write must create a rollback receipt.

## Scope

### Codex Memory

Detect and manage Codex-facing memory inputs:

- `AGENTS.md` and nested repo instruction files.
- Codex home/project notes when available.
- Switchboard-generated session packs.
- Goal summaries, command conventions, recurring failure notes, and repo-specific decisions.

Initial behavior:

- Read-only discovery.
- Token estimate per file/section.
- Duplicate instruction detection.
- Suggested compacted memory preview.
- One-click copy or one-click apply only after rollback support exists.

### Claude Memory

Detect and manage Claude-facing memory inputs:

- `CLAUDE.md`.
- `.claude/CLAUDE.md`.
- Managed Switchboard Caveman / Compact Chinese instruction blocks.
- Claude Code routing and RTK guidance blocks.

Initial behavior:

- Detect overlong or duplicated Claude memory.
- Warn when global and repo Claude memory conflict.
- Preserve app-owned managed blocks.
- Suggest splits between durable repo facts and temporary session instructions.

### Unified Memory Inspector

Add a single view that answers:

> What will this agent remember or receive if I start a session now?

The inspector should show:

- Source: Codex, Claude, AGENTS.md, CLAUDE.md, Repo Intelligence, Repo Memory MCP, Switchboard pack.
- Scope: global, repo, nested path, generated session pack.
- Status: live, stale, duplicate, app-managed, user-managed, blocked, missing.
- Estimated tokens.
- Cache friendliness.
- Redundancy percentage.
- Secret-scan status.
- Last modified time and rollback availability.

### Memory Compression

Use existing optimization primitives:

- Token X-ray for where memory tokens go.
- Prompt-cache ordering for stable memory prefixes.
- Redundancy detection for repeated rules across `AGENTS.md`, `CLAUDE.md`, repo packs, and session history.
- Caveman / Compact Chinese profiles for optional terse rewrites.

Compression output should be a preview with a diff, not an automatic rewrite.

### Memory Safety

Before any memory write:

- Scan for obvious secrets, tokens, private URLs, and personal data.
- Show exact target path.
- Show before/after diff.
- Create rollback receipt.
- Require explicit user confirmation for repo/global writes.
- Preserve app-owned managed blocks separately from user-authored text.

## Architecture

### Backend Modules

Recommended new modules:

- `src-tauri/src/agent_memory/mod.rs`
- `src-tauri/src/agent_memory/discovery.rs`
- `src-tauri/src/agent_memory/dedup.rs`
- `src-tauri/src/agent_memory/preview.rs`
- `src-tauri/src/agent_memory/rollback.rs`
- `src-tauri/src/agent_memory/secret_scan.rs`

Recommended Tauri commands:

- `get_agent_memory_snapshot(repo_path?: string)`
- `preview_agent_memory_compaction(repo_path: string, agent: AgentMemoryTarget)`
- `apply_agent_memory_update(request)`
- `rollback_agent_memory_update(receipt_id: string)`

### Frontend Modules

Recommended new UI:

- `src/components/AgentMemoryInspector.tsx`
- `src/components/AgentMemorySourceList.tsx`
- `src/components/AgentMemoryDiffPreview.tsx`
- `src/lib/agentMemory.ts`

View placement:

- Add as an Optimization / Repo Intelligence adjacent view.
- Also surface a compact memory status row inside Start Agent Session.

### Data Model

Core snapshot fields:

- `agent`: `codex | claude | shared | repo_memory_mcp`
- `sourcePath`
- `scope`
- `managedBySwitchboard`
- `estimatedTokens`
- `duplicateTokens`
- `cacheableTokens`
- `freshness`
- `secretScan`
- `recommendedAction`
- `previewAvailable`
- `rollbackAvailable`

Rollback receipt fields:

- `receiptId`
- `createdAt`
- `agent`
- `targetPath`
- `beforeSha256`
- `afterSha256`
- `backupPath`
- `managedBlockIds`
- `userConfirmed`

## Implementation Slices

### Slice 1: Read-Only Agent Memory Snapshot

Deliverables:

- Discover Codex, Claude, AGENTS.md, CLAUDE.md, Repo Intelligence, and Repo Memory MCP memory sources.
- Show source path, scope, ownership, modified time, and estimated tokens.
- No writes.

Verification:

- Unit tests for path discovery.
- Fixture tests for nested `AGENTS.md` and `.claude/CLAUDE.md`.
- UI test for source list rendering.

### Slice 2: Token X-ray + Redundancy Integration

Deliverables:

- Memory snapshot reports duplicate tokens across memory files.
- Token X-ray gets a dedicated `memory` bucket.
- Prompt-cache view identifies stable memory prefixes.

Verification:

- Duplicate `AGENTS.md` / `CLAUDE.md` fixture.
- Snapshot includes `memoryTokens`, `duplicateMemoryTokens`, and `cacheableMemoryTokens`.

### Slice 3: Compaction Preview

Deliverables:

- Generate compacted memory preview without writing.
- Show before/after token estimate.
- Show diff.
- Preserve app-managed blocks.

Verification:

- Diff fixture test.
- App-managed block preservation test.
- No-write guarantee test.

### Slice 4: Secret Scan and Safety Gates

Deliverables:

- Secret scan for memory previews and proposed writes.
- Block apply when high-risk secrets are found.
- Show user-readable reason and affected line/category.

Verification:

- Token/API-key fixture tests.
- Safe false-positive handling tests.

### Slice 5: Apply With Rollback

Deliverables:

- One-click apply after explicit confirmation.
- Rollback receipt and backup.
- Rollback command.
- Doctor issue when receipt is stale or rollback backup is missing.

Verification:

- Apply/rollback integration test using temp repo.
- Backup integrity SHA checks.
- Doctor stale rollback test.

### Slice 6: Start Agent Session Integration

Deliverables:

- Start Agent Session automatically includes the approved compact memory pack.
- Users can inspect memory pack before starting.
- Cache-friendly ordering puts stable memory first.

Verification:

- Session pack includes memory summary.
- Prompt-cache ordering test.
- UI test for memory status row.

### Slice 7: Durable Savings Attribution

Deliverables:

- Attribute memory compaction savings by agent and source.
- Report saved tokens from removed duplication and shorter memory files.
- Show memory savings in daily/session ledgers.

Verification:

- Savings attribution fixture.
- Per-agent memory savings row in dashboard.

## Sub-Agent Plan

Use sub-agents in the main implementation thread with non-overlapping ownership:

- Backend discovery agent: `agent_memory/discovery.rs`, tests, fixtures.
- Backend safety agent: secret scan, rollback receipts, apply flow.
- Optimization agent: Token X-ray, prompt-cache, redundancy integration.
- Frontend agent: inspector UI, diff preview, Start Agent Session row.
- QA agent: fixtures, smoke scripts, docs verification.

The main thread should integrate slices, run full checks, and commit/push each slice separately.

## UX Requirements

- Do not hide where memory comes from.
- Do not auto-write memory files in the first release.
- Distinguish generated session memory from durable repo memory.
- Use “Preview,” “Copy,” “Apply,” and “Rollback” as separate actions.
- Show readable warnings for global memory edits.
- Keep long memory paths wrapped and copyable.
- Make secret warnings blocking, not decorative.

## Verification Commands

```bash
npm run build
npm run test:frontend -- src/lib/agentMemory.test.ts src/components/AgentMemoryInspector.test.tsx
cargo test --manifest-path src-tauri/Cargo.toml agent_memory -- --test-threads=1
npm run check:file-size-budget
```

## Open Questions

- Which Codex memory paths should be considered app-managed versus user-managed?
- Should global memory writes be disabled until repo-scoped writes have shipped?
- Should Compact Chinese be offered for memory files or only for session output?
- Should memory compaction create Markdown, JSON, or both?
- Should memory packs be exported through Repo Memory MCP or only Start Agent Session?

## Success Criteria

- Users can see all agent memory sources before starting a session.
- Users can identify duplicated, stale, or expensive memory.
- Users can preview compacted memory with a token estimate.
- No write happens without explicit approval and rollback.
- Memory savings appear in Token X-ray and savings attribution.
- Codex and Claude memory stay distinct while sharing a single inspector.
