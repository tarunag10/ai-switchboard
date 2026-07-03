# MAC AI Switchboard Codex Implementation Plan

Repository: `tarunag10/mac-ai-switchboard`  
Project: `MAC-AS-Switchboard` / `mac-ai-switchboard`  
Plan purpose: PR-sized Codex implementation roadmap for prompt-cache optimization, automatic repo/task pack injection, preemptive compaction, Token X-ray, redundancy detection, model routing, and RTK framework presets.

---

## 0. Executive summary

The repository is a Tauri 2 desktop application, not a pure Node proxy. The maintainable path is:

- Put request-path inspection, provider payload parsing, local telemetry, compaction decisions, redundancy hashing, and model routing decisions in Rust under `src-tauri/src/optimization/`.
- Keep provider-specific mutation isolated behind adapters.
- Keep TypeScript focused on UI panels, shared report types, frontend-only helpers, and Vitest coverage.
- Extend existing Node scripts for repo-intelligence/session-start CLI workflows and local JSON report export.
- Preserve existing behavior by default. All automation starts disabled, dry-run, or observe-only.

The highest-leverage sequence is:

1. Optimization policy and local telemetry store.
2. Provider payload parser and token ledger.
3. Cache metrics and Token X-ray reporting.
4. Start Agent Session plus budgeted pack injection.
5. Preemptive compaction planning, then safe mutation.
6. Redundancy detection.
7. Model routing observe mode, then optional route mutation.
8. RTK framework presets.
9. Docs, privacy notes, and final disabled-default regression hardening.

---

## 1. Repository understanding

### 1.1 Current architecture

The app has three layers:

#### React/Vite frontend

Key paths:

```text
src/App.tsx
src/components/*
src/lib/*
src/test/setup.ts
vite.config.ts
```

Observed frontend facts:

- `src/App.tsx` is the main UI composition point.
- Current primary nav items are `Home`, `Optimize`, `Activity`, and `Addons`.
- `src/lib/types.ts` mirrors Rust/Tauri wire types.
- `src/lib/repoIntelligence.ts` already has pure TypeScript helpers for repo packs, manifests, handoffs, token estimates, graph summaries, and tests.
- Frontend tests use Vitest with jsdom.

#### Rust/Tauri backend

Key paths:

```text
src-tauri/src/lib.rs
src-tauri/src/models.rs
src-tauri/src/state.rs
src-tauri/src/proxy_intercept.rs
src-tauri/src/client_adapters.rs
src-tauri/src/repo_intelligence.rs
src-tauri/src/storage.rs
src-tauri/src/tool_manager.rs
src-tauri/src/pricing.rs
```

Observed backend facts:

- `src-tauri/src/lib.rs` declares core modules and Tauri commands.
- `src-tauri/src/models.rs` defines frontend-facing serde models.
- `src-tauri/src/state.rs` owns `AppState`, runtime lifecycle, caches, telemetry-like trackers, bearer token slots, Codex usage slots, bypass flags, and runtime boot logic.
- `src-tauri/src/proxy_intercept.rs` is the transparent local HTTP intercept on `127.0.0.1:6767`.
- The intercept forwards to the managed Python Headroom backend on a selected backend port.
- The intercept currently reads only through HTTP headers before forwarding to avoid request-body deadlocks.
- The intercept detects Codex/OpenAI paths, captures bearer tokens, stamps `X-Client: codex`, captures Codex usage/rate-limit information, and detects `413 compression_refused` responses.
- `src-tauri/src/storage.rs` centralizes local app storage under the existing Headroom app-data directory, including `config/` and `telemetry/`.
- `src-tauri/src/repo_intelligence.rs` already builds read-only repo summaries and bounded context packs.

#### Node scripts

Key paths:

```text
scripts/repo-intelligence.mjs
scripts/*.mjs
package.json
```

Observed script facts:

- `scripts/repo-intelligence.mjs` supports repo packs, agent handoffs, manifest output, and an MCP-like stdio server.
- `package.json` exposes `repo:intelligence`, frontend tests, Rust tests, release checks, governance checks, connector checks, and build scripts.

### 1.2 Current proxy/request flow

Current flow:

```text
Claude Code / Codex / supported client
  -> client config points at http://127.0.0.1:6767
  -> Rust proxy_intercept accepts loopback HTTP connection
  -> proxy_intercept reads headers only
  -> validates Host/Origin loopback safety
  -> detects OpenAI/Codex by path when applicable
  -> captures bearer token and Codex plan/usage signals
  -> applies bypass gates if active
  -> connects to managed Python Headroom backend port
  -> stamps Codex requests with X-Client: codex if needed
  -> forwards bytes to backend
  -> for Codex responses, sniffs response head for rate-limit headers and 413 compression_refused
  -> streams response to client
```

Important implication: body parsing and mutation must be added carefully. The current code intentionally avoids buffering request bodies except where direct bypass forwarding needs it. All new provider-payload features must preserve byte-for-byte pass-through when disabled and must refuse to parse/mutate unknown, chunked, streaming, oversized, or non-JSON payloads.

### 1.3 Existing session lifecycle

There are two existing “session-like” concepts:

1. App/runtime session:
   - `AppState::new_in` initializes local state, caches, token slots, bypass flags, runtime tracking, and persisted structures.
   - `warm_runtime_on_launch` restores switchboard mode, ensures RTK integrations, handles runtime maintenance, starts the Python backend, and waits for readiness.

2. Repo/agent handoff session:
   - Repo Intelligence builds `implementation`, `verification`, and `handoff` packs.
   - `scripts/repo-intelligence.mjs` can produce agent-specific handoff payloads and manifests.

There is no inspected first-class `Start Agent Session` primitive yet. This plan adds it as a new explicit local session-start API instead of overloading runtime startup.

### 1.4 Existing token and savings primitives

Reusable existing models:

```text
PipelineStageMetric
UsageEvent
DailySavingsPoint
ProviderSavingsPoint
HourlySavingsPoint
DashboardState
TransformationFeedEvent
TransformationFeedResponse
RepoFileSignal
RepoContextPack
RepoIntelligenceSummary
```

Key gap: there is request-level savings and transformation reporting, but not a session-level token ledger that attributes tokens to categories like system prompt, file reads, tool output, conversation history, retries, model responses, compaction summaries, injected packs, or redundant reads.

### 1.5 Current persistence

Existing storage convention:

```text
~/Library/Application Support/Headroom/config/
~/Library/Application Support/Headroom/telemetry/
```

Keep the existing storage root for compatibility. Do not introduce a migration unless necessary. Use JSON/JSONL first; only move to SQLite if report queries become too slow.

### 1.6 Test setup

Run commands:

```bash
npm run test:frontend
npm run test:desktop
npm run test:all
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Existing conventions:

- Frontend: Vitest + jsdom.
- Rust: Cargo tests, many module-level tests.
- Repo Intelligence has strong pure TypeScript tests.
- `proxy_intercept.rs` already has useful helper tests and should be extended carefully.

---

## 2. Proposed architecture

### 2.1 New Rust module tree

Add:

```text
src-tauri/src/optimization/
  mod.rs
  policy.rs
  telemetry_store.rs
  provider_payload.rs
  token_estimator.rs
  token_ledger.rs
  prompt_segments.rs
  prompt_layout.rs
  cache_metrics.rs
  agent_sessions.rs
  compaction.rs
  redundancy.rs
  model_routing.rs
  rtk_presets.rs
  reports.rs
```

Add to `src-tauri/src/lib.rs`:

```rust
mod optimization;
```

### 2.2 Module responsibilities

| Module | Responsibility |
|---|---|
| `policy.rs` | Load and validate local optimization policy, env overrides, safe defaults. |
| `telemetry_store.rs` | Local JSONL append/read/prune helpers under `telemetry/`. |
| `provider_payload.rs` | Normalize OpenAI and Anthropic request/response bodies into provider-agnostic envelopes. |
| `token_estimator.rs` | Token estimation and provider/model metadata. |
| `token_ledger.rs` | Session ledger, per-turn attribution, category totals. |
| `prompt_segments.rs` | Extract cacheable/stable/order-sensitive segments from provider envelopes. |
| `prompt_layout.rs` | Stable-prefix planning and unsafe-reorder validation. |
| `cache_metrics.rs` | Provider cached-token usage parsing, cache metrics, cache report aggregation. |
| `agent_sessions.rs` | Explicit Start Agent Session API and budgeted pack injection planning. |
| `compaction.rs` | Projected token thresholds, compaction triggers, deterministic extractive compaction. |
| `redundancy.rs` | SHA-256 duplicate file/tool-content hash tracking without raw content storage. |
| `model_routing.rs` | Route table, task classification, safety bypass, model route decisions. |
| `rtk_presets.rs` | Vitest/Jest/Pytest/Cargo auto-detection and output filtering presets. |
| `reports.rs` | Token X-ray, savings, cache, redundancy, compaction, routing report assembly. |

### 2.3 Frontend additions

Add:

```text
src/lib/optimizationTypes.ts
src/lib/optimizationPolicy.ts
src/lib/tokenXray.ts
src/lib/cacheEfficiency.ts
src/lib/modelRouting.ts
src/lib/rtkPresets.ts
src/components/TokenXrayPanel.tsx
src/components/CacheEfficiencyPanel.tsx
src/components/RedundancyPanel.tsx
src/components/ModelRoutingPanel.tsx
src/components/RtkPresetPanel.tsx
```

Prefer integration into existing tabs:

- `Optimize`: policy controls, Start Agent Session, cache efficiency, compaction, model routing, RTK preset selector.
- `Activity`: Token X-ray, latest cache/compaction/routing/redundancy events.
- `Addons`: RTK preset status under existing RTK add-on card.

Avoid a new top-level nav item until the panels mature.

### 2.4 Node script additions

Extend existing CLI rather than inventing a separate repo pack generator:

```bash
npm run repo:intelligence -- . --start-session --agent codex --format json
npm run repo:intelligence -- . --start-session --agent claude --auto-inject-packs
npm run repo:intelligence -- . --manifest --include-cache-plan
```

Add a report CLI:

```text
scripts/optimization-report.mjs
```

Example usage:

```bash
node scripts/optimization-report.mjs --token-xray --json
node scripts/optimization-report.mjs --cache --client codex --json
node scripts/optimization-report.mjs --redundancy --repo . --json
node scripts/optimization-report.mjs --routing --json
```

### 2.5 New request data flow

Target flow after feature wiring:

```text
Client request
  -> proxy_intercept::handle
  -> loopback/origin safety check
  -> provider/client detection
  -> if enabled and safe: bounded JSON body read
  -> ProviderRequestEnvelope
  -> token attribution
  -> redundancy detection
  -> session/pack injection plan
  -> preemptive compaction plan
  -> prompt cache layout plan
  -> model routing decision
  -> provider adapter serializes safe mutations if enabled
  -> forward to Headroom backend or direct bypass
  -> response usage metadata extraction where possible
  -> cache metrics, ledger, spend, compaction, redundancy, routing events persisted locally
```

### 2.6 Body parsing safety rules

Only parse/mutate request bodies when all are true:

- Feature policy is enabled for body inspection.
- Request path is known provider API path.
- Method is `POST`.
- `Content-Type` is JSON or compatible.
- `Content-Length` exists and is below configured max, for example 4 MiB.
- Request is not chunked.
- Provider adapter supports the endpoint.
- Serialization can preserve unknown fields.

Otherwise pass through unchanged and emit observe-only skip reason.

---

## 3. Data model and interfaces

Add Rust serde models to `src-tauri/src/models.rs` or split internal models under `optimization/` and re-export frontend-facing report models from `models.rs`.

Add TypeScript mirrors to `src/lib/optimizationTypes.ts`.

### 3.1 Token attribution

```ts
export type TokenCategory =
  | "system_prompt"
  | "injected_repo_pack"
  | "injected_task_pack"
  | "file_read"
  | "tool_output"
  | "conversation_history"
  | "retry"
  | "model_response"
  | "compaction_summary"
  | "redundant_read"
  | "current_user_turn"
  | "unknown";

export interface TokenAttribution {
  id: string;
  requestId: string;
  turnId?: string;
  sessionId: string;
  clientId: string;
  provider?: string;
  model?: string;
  category: TokenCategory;
  sourceKind:
    | "provider_request"
    | "provider_response"
    | "repo_pack"
    | "tool_result"
    | "file_read"
    | "compaction"
    | "retry"
    | "estimate";
  sourceId?: string;
  path?: string;
  tokens: number;
  estimated: boolean;
  costUsd?: number;
  cacheEligible?: boolean;
  cachedTokens?: number;
  avoidable?: boolean;
  createdAt: string;
}

export interface SessionTokenLedger {
  schemaVersion: 1;
  sessionId: string;
  clientId: string;
  repoRoot?: string;
  startedAt: string;
  updatedAt: string;
  totals: {
    inputTokens: number;
    outputTokens: number;
    totalTokens: number;
    estimatedSpendUsd: number;
    estimatedSavingsUsd: number;
    cacheSavingsUsd: number;
    compactionSavingsTokens: number;
    redundancyWasteTokens: number;
  };
  byCategory: Record<TokenCategory, number>;
  turns: Array<{
    turnId: string;
    requestId: string;
    startedAt: string;
    provider?: string;
    model?: string;
    inputTokens: number;
    outputTokens: number;
    spendUsd: number;
    deltas: Partial<Record<TokenCategory, number>>;
    warnings: string[];
  }>;
  attributions: TokenAttribution[];
}
```

### 3.2 Prompt cache

```ts
export type CacheSegmentKind =
  | "system_prompt"
  | "developer_instruction"
  | "repo_pack"
  | "task_pack"
  | "stable_context"
  | "tool_definition"
  | "conversation_history"
  | "file_read"
  | "tool_output"
  | "current_user_turn"
  | "model_response"
  | "compaction_summary";

export interface CacheableSegment {
  id: string;
  kind: CacheSegmentKind;
  providerRole: "system" | "developer" | "user" | "assistant" | "tool" | "unknown";
  tokenCount: number;
  contentHash: string;
  stable: boolean;
  cacheEligible: boolean;
  orderSensitive: boolean;
  source: {
    clientId?: string;
    sessionId?: string;
    repoRoot?: string;
    packId?: string;
    messageIndex?: number;
    path?: string;
  };
  reasons: string[];
}

export interface ProviderCachePolicy {
  provider: "openai" | "anthropic" | "unknown";
  minCacheableTokens: number;
  stablePrefixRequired: boolean;
  explicitCacheControlSupported: boolean;
  cachedInputDiscountPct?: number;
  usageFields: {
    cachedTokens?: string[];
    cacheCreationTokens?: string[];
    cacheReadTokens?: string[];
  };
}

export interface PromptLayoutPlan {
  requestId: string;
  provider: string;
  model: string;
  originalSegmentOrder: string[];
  plannedSegmentOrder: string[];
  stablePrefixTokens: number;
  cacheableTokens: number;
  unsafeToReorder: boolean;
  reorderApplied: boolean;
  validationWarnings: string[];
  segments: CacheableSegment[];
}

export interface CacheMetrics {
  requestId: string;
  sessionId: string;
  clientId: string;
  provider: string;
  model: string;
  promptTokens: number;
  cacheableTokens: number;
  stablePrefixTokens: number;
  cachedTokensActual?: number;
  cacheCreationTokensActual?: number;
  cacheReadTokensActual?: number;
  cachedTokensEstimated: number;
  cacheableTokenRatio: number;
  cacheHitRate: number;
  estimatedCachedTokenSavingsUsd: number;
  providerMetadataComplete: boolean;
  calculatedAt: string;
}

export interface CacheEfficiencyReport {
  schemaVersion: 1;
  generatedAt: string;
  scope: {
    clientId?: string;
    sessionId?: string;
    repoRoot?: string;
  };
  totals: {
    requests: number;
    promptTokens: number;
    cacheableTokens: number;
    cachedTokensActual: number;
    cachedTokensEstimated: number;
    estimatedSavingsUsd: number;
  };
  byClient: Array<{
    clientId: string;
    requests: number;
    cacheableTokenRatio: number;
    cacheHitRate: number;
    estimatedSavingsUsd: number;
  }>;
  recent: CacheMetrics[];
}
```

### 3.3 Compaction

```ts
export interface CompactionTrigger {
  requestId: string;
  sessionId: string;
  provider: string;
  model: string;
  contextLimitTokens: number;
  projectedPromptTokens: number;
  utilizationPct: number;
  thresholdPct: number;
  reason:
    | "threshold_exceeded"
    | "provider_413_risk"
    | "retry_after_413"
    | "manual";
  mode: "observe" | "warn" | "compact";
}

export interface CompactionResult {
  requestId: string;
  sessionId: string;
  triggered: boolean;
  reason: CompactionTrigger["reason"];
  originalTokenCount: number;
  compactedTokenCount: number;
  savingsTokens: number;
  savingsPct: number;
  preservedSections: Array<{
    kind:
      | "system_instructions"
      | "task_state"
      | "repo_facts"
      | "recent_turns"
      | "tool_failures"
      | "unresolved_user_intent";
    tokens: number;
    reason: string;
  }>;
  summarySegmentId?: string;
  warnings: string[];
  createdAt: string;
}
```

### 3.4 Redundancy

```ts
export interface RedundancyRecord {
  id: string;
  sessionId: string;
  clientId: string;
  repoRoot?: string;
  path?: string;
  contentHash: string;
  tokenCount: number;
  firstSeenAt: string;
  lastSeenAt: string;
  repeatedCount: number;
  duplicateTokenCost: number;
  estimatedAvoidableSpendUsd: number;
  changedSinceFirstSeen: boolean;
  privacy: {
    rawContentStored: false;
    pathStored: boolean;
    hashAlgorithm: "sha256";
  };
}

export interface RedundancyReport {
  schemaVersion: 1;
  generatedAt: string;
  scope: {
    sessionId?: string;
    clientId?: string;
    repoRoot?: string;
  };
  totals: {
    observedFileReadTokens: number;
    duplicateReadTokens: number;
    redundancyPct: number;
    estimatedAvoidableSpendUsd: number;
  };
  repeatedFiles: RedundancyRecord[];
}
```

### 3.5 Model routing

```ts
export type TaskClass =
  | "lint_fix"
  | "commit_message"
  | "short_explanation"
  | "formatting_change"
  | "test_name_generation"
  | "simple_refactor"
  | "verification"
  | "architecture"
  | "security_sensitive"
  | "large_edit"
  | "ambiguous"
  | "unknown";

export interface ModelRouteDecision {
  requestId: string;
  sessionId: string;
  clientId: string;
  provider: string;
  originalModel: string;
  selectedProvider: string;
  selectedModel: string;
  taskClass: TaskClass;
  confidence: number;
  reason: string;
  estimatedInputTokens: number;
  estimatedOutputTokens?: number;
  estimatedSavingsUsd?: number;
  overrideStatus:
    | "none"
    | "explicit_model_preserved"
    | "forced_by_user"
    | "blocked_by_safety"
    | "fallback_original";
  settings: {
    temperature?: number;
    maxTokens?: number;
  };
  safetySignals: string[];
  createdAt: string;
}
```

### 3.6 RTK presets

```ts
export interface FrameworkPreset {
  id: "vitest" | "jest" | "pytest" | "cargo";
  label: string;
  autoDetect: {
    commands: string[];
    filePatterns: string[];
    manifestSignals: string[];
    outputPatterns: string[];
  };
  preservePatterns: string[];
  dropPatterns: string[];
  collapsePatterns: Array<{
    name: string;
    pattern: string;
    replacement: string;
  }>;
  maxContextLinesAroundFailure: number;
}

export interface RtkPresetDecision {
  command: string;
  selectedPreset: FrameworkPreset["id"] | "auto" | "none";
  detectedFramework?: FrameworkPreset["id"];
  reason: string;
  originalBytes?: number;
  filteredBytes?: number;
  estimatedTokensSaved?: number;
}
```

### 3.7 Savings report

```ts
export interface SavingsReport {
  schemaVersion: 1;
  generatedAt: string;
  sessionId?: string;
  totals: {
    tokens: number;
    estimatedSpendUsd: number;
    estimatedSavingsUsd: number;
    cacheSavingsUsd: number;
    compactionSavingsTokens: number;
    redundancyWasteTokens: number;
    avoidableSpendUsd: number;
  };
  wasteIndicators: Array<{
    kind: "redundant_reads" | "low_cacheability" | "oversized_history" | "routing_missed";
    severity: "info" | "warning" | "critical";
    tokens: number;
    estimatedUsd: number;
    message: string;
  }>;
  tokenXray: SessionTokenLedger;
  cache?: CacheEfficiencyReport;
  compactionEvents: CompactionResult[];
  redundancy?: RedundancyReport;
  routing: ModelRouteDecision[];
}
```

---

## 4. Configuration design

### 4.1 Config file

Add:

```text
~/Library/Application Support/Headroom/config/optimization-policy.json
```

Default contents produced by `policy.rs` when missing:

```json
{
  "schemaVersion": 1,
  "telemetry": {
    "enabled": false,
    "storeRawPromptContent": false,
    "retentionDays": 30
  },
  "tokenXray": {
    "enabled": false,
    "mode": "observe"
  },
  "promptCache": {
    "enabled": false,
    "mode": "observe",
    "allowSafeReordering": false,
    "minStablePrefixTokens": 1024
  },
  "agentSessions": {
    "autoInjectPacks": {
      "enabled": false,
      "mode": "dry_run",
      "defaultBudgetTokens": 4000,
      "maxPackCount": 2,
      "clientDefaults": {
        "claude_code": ["implementation"],
        "codex": ["verification"]
      }
    }
  },
  "compaction": {
    "enabled": false,
    "mode": "observe",
    "observeAtPct": 0.7,
    "warnAtPct": 0.8,
    "compactAtPct": 0.9,
    "preserveRecentTurns": 6,
    "maxBodyBytes": 4194304
  },
  "redundancy": {
    "enabled": false,
    "storePathIdentifiers": true,
    "hashAlgorithm": "sha256",
    "retentionDays": 30,
    "sessionIsolation": true,
    "trackAcrossRepos": false
  },
  "modelRouting": {
    "enabled": false,
    "mode": "observe",
    "respectExplicitModel": true,
    "maxRoutableInputTokens": 12000,
    "rules": [],
    "safetyBypassTerms": [
      "security",
      "auth",
      "crypto",
      "database migration",
      "production incident",
      "large refactor"
    ]
  },
  "rtk": {
    "preset": "auto",
    "enabledFrameworks": ["vitest", "jest", "pytest", "cargo"],
    "maxOutputBytesBeforePreset": 200000,
    "preserveStackTraces": true,
    "preserveAssertionDiffs": true
  }
}
```

### 4.2 Environment overrides

Use existing `HEADROOM_` naming style:

```bash
HEADROOM_OPTIMIZATION_TELEMETRY=1
HEADROOM_TOKEN_XRAY=1
HEADROOM_PROMPT_CACHE_OPTIMIZATION=observe
HEADROOM_AUTO_INJECT_PACKS=dry_run
HEADROOM_PREEMPTIVE_COMPACTION=observe
HEADROOM_REDUNDANCY_DETECTION=1
HEADROOM_MODEL_ROUTING=observe
HEADROOM_RTK_PRESET=auto
HEADROOM_OPTIMIZATION_POLICY=/path/to/optimization-policy.json
```

Frontend-only display flag, if needed:

```bash
VITE_HEADROOM_OPTIMIZATION_PANELS=1
```

### 4.3 CLI flags

Add to `scripts/repo-intelligence.mjs`:

```bash
--start-session
--auto-inject-packs
--pack-budget <tokens>
--include-cache-plan
```

Add to `scripts/optimization-report.mjs`:

```bash
--token-xray
--cache
--redundancy
--compaction
--routing
--session <id>
--client <id>
--repo <path>
--json
```

---

## 5. Storage and persistence

Use existing local storage root and helpers from `storage.rs`.

Add files:

```text
~/Library/Application Support/Headroom/config/
  optimization-policy.json
  model-routing-rules.json
  rtk-presets.json

~/Library/Application Support/Headroom/telemetry/
  token-ledger.jsonl
  cache-metrics.jsonl
  compaction-events.jsonl
  redundancy-records.jsonl
  model-routing-decisions.jsonl
  rtk-preset-events.jsonl
  optimization-report-latest.json
```

### 5.1 Privacy defaults

- No raw prompts stored by default.
- No raw file contents stored ever for redundancy detection.
- Redundancy uses SHA-256 content hashes.
- Path identifiers are optional and redacted for secret-like paths.
- Full prompt/message capture remains controlled by existing full-message logging behavior.
- All reports must clearly label estimates.

### 5.2 Retention

Add `telemetry_store.rs` helpers:

```rust
pub fn append_jsonl<T: serde::Serialize>(path: &Path, event: &T) -> anyhow::Result<()>;
pub fn read_recent_jsonl<T: serde::de::DeserializeOwned>(path: &Path, max_records: usize) -> anyhow::Result<Vec<T>>;
pub fn prune_jsonl_by_age(path: &Path, retention_days: u32) -> anyhow::Result<PruneResult>;
```

Prune on:

- app launch
- policy update
- every N writes, for example 100

Do not introduce SQLite in the first pass unless JSONL report assembly is demonstrably too slow.

---

## 6. API, CLI, and UI changes

### 6.1 Tauri commands

Add commands to `src-tauri/src/lib.rs`:

```rust
#[tauri::command]
fn get_optimization_policy() -> Result<OptimizationPolicy, String>;

#[tauri::command]
fn update_optimization_policy(patch: OptimizationPolicyPatch) -> Result<OptimizationPolicy, String>;

#[tauri::command]
fn start_agent_session(request: AgentSessionStartRequest) -> Result<AgentSessionStartResult, String>;

#[tauri::command]
fn get_token_xray_report(session_id: Option<String>) -> Result<SavingsReport, String>;

#[tauri::command]
fn get_cache_efficiency_report(session_id: Option<String>) -> Result<CacheEfficiencyReport, String>;

#[tauri::command]
fn get_redundancy_report(session_id: Option<String>) -> Result<RedundancyReport, String>;

#[tauri::command]
fn get_compaction_events(session_id: Option<String>) -> Result<Vec<CompactionResult>, String>;

#[tauri::command]
fn get_model_routing_decisions(session_id: Option<String>) -> Result<Vec<ModelRouteDecision>, String>;

#[tauri::command]
fn get_rtk_preset_status() -> Result<RtkPresetStatus, String>;
```

### 6.2 Sample Token X-ray JSON response

```json
{
  "schemaVersion": 1,
  "generatedAt": "2026-07-03T12:00:00Z",
  "sessionId": "ses_abc",
  "totals": {
    "tokens": 184230,
    "estimatedSpendUsd": 3.42,
    "estimatedSavingsUsd": 1.18,
    "cacheSavingsUsd": 0.64,
    "compactionSavingsTokens": 32000,
    "redundancyWasteTokens": 9100,
    "avoidableSpendUsd": 0.21
  },
  "wasteIndicators": [
    {
      "kind": "redundant_reads",
      "severity": "warning",
      "tokens": 9100,
      "estimatedUsd": 0.21,
      "message": "3 unchanged files were read repeatedly in this session."
    }
  ],
  "tokenXray": {
    "schemaVersion": 1,
    "sessionId": "ses_abc",
    "clientId": "codex",
    "startedAt": "2026-07-03T11:10:00Z",
    "updatedAt": "2026-07-03T12:00:00Z",
    "totals": {
      "inputTokens": 143000,
      "outputTokens": 41230,
      "totalTokens": 184230,
      "estimatedSpendUsd": 3.42,
      "estimatedSavingsUsd": 1.18,
      "cacheSavingsUsd": 0.64,
      "compactionSavingsTokens": 32000,
      "redundancyWasteTokens": 9100
    },
    "byCategory": {
      "system_prompt": 12000,
      "injected_repo_pack": 8000,
      "injected_task_pack": 0,
      "file_read": 53000,
      "tool_output": 21000,
      "conversation_history": 39000,
      "retry": 4000,
      "model_response": 41230,
      "compaction_summary": 4500,
      "redundant_read": 9100,
      "current_user_turn": 5200,
      "unknown": 0
    },
    "turns": [],
    "attributions": []
  },
  "compactionEvents": [],
  "routing": []
}
```

### 6.3 UI placement

#### Optimize tab

Add cards:

- Start Agent Session
- Auto-injected packs dry-run/apply result
- Prompt-cache efficiency
- Preemptive compaction thresholds
- Model routing mode and latest decision
- RTK preset selector

#### Activity tab

Add timeline/report cards:

- Token X-ray summary
- Per-turn deltas
- Cache hit/miss trend
- Compaction triggered/skipped events
- Redundancy warnings
- Model route selected/skipped decisions
- Export JSON button

#### Addons tab

Extend RTK card:

- Preset: `auto`, `none`, `vitest`, `jest`, `pytest`, `cargo`
- Last detected framework
- Estimated output tokens saved by preset

---

## 7. Feature implementation details

## A. Prompt-cache optimization

### Goal

Measure and optimize provider prompt-cache efficiency. Keep stable content first where safe, and report cacheable token ratio, cache-hit rate, cached-token savings estimate, and per-client cache efficiency.

### Current repo leverage

- `RepoContextPack` already models implementation/verification/handoff packs.
- `RepoIntelligenceSummary` already stores pack token estimates.
- `TransformationFeedEvent` already carries provider/model and input token counts.
- Local proxy path list already includes `/cache`, useful for future backend cache status integration.

### Implementation steps

1. Add `ProviderCachePolicy` for OpenAI, Anthropic, and unknown.
2. Add `provider_payload.rs` normalized envelope:

```rust
pub struct ProviderRequestEnvelope {
    pub request_id: String,
    pub provider: ProviderKind,
    pub endpoint: ProviderEndpoint,
    pub model: Option<String>,
    pub messages: Vec<ProviderMessage>,
    pub system: Vec<ProviderContentBlock>,
    pub tools: Vec<ProviderToolDefinition>,
    pub raw_usage: Option<serde_json::Value>,
    pub unknown_fields_preserved: bool,
}
```

3. Add `prompt_segments.rs` extraction:
   - system prompt
   - developer instructions
   - repo packs
   - task packs
   - tool definitions
   - stable context blocks
   - file reads
   - tool output
   - conversation history
   - current user turn

4. Add stable hashing:
   - SHA-256 over canonical text and segment metadata.
   - Store segment hash and token count only.

5. Add cache metrics:
   - `cacheableTokenRatio`
   - `stablePrefixTokens`
   - `cachedTokensActual`
   - `cacheCreationTokensActual`
   - `cacheReadTokensActual`
   - fallback `cachedTokensEstimated`
   - per-client aggregation

6. Add validation before reordering:
   - Do not reorder assistant/tool interleavings.
   - Do not reorder segments with order-sensitive references.
   - Do not reorder unknown structured content.
   - Do not reorder unless provider adapter explicitly permits it.

7. Phase 1: metrics only.
8. Phase 2: layout plan only.
9. Phase 3: safe reordering only when policy enables it and validation passes.

### Files to add/modify

```text
src-tauri/src/optimization/provider_payload.rs
src-tauri/src/optimization/prompt_segments.rs
src-tauri/src/optimization/prompt_layout.rs
src-tauri/src/optimization/cache_metrics.rs
src-tauri/src/optimization/reports.rs
src-tauri/src/models.rs
src/lib/optimizationTypes.ts
src/lib/cacheEfficiency.ts
src/components/CacheEfficiencyPanel.tsx
```

### Tests

```text
src-tauri/tests/fixtures/provider/openai_usage_cached.json
src-tauri/tests/fixtures/provider/anthropic_usage_cached.json
src-tauri/src/optimization/cache_metrics.rs tests
src-tauri/src/optimization/prompt_layout.rs tests
src/lib/cacheEfficiency.test.ts
```

Test cases:

- OpenAI cached-token usage is parsed correctly.
- Anthropic cache-read/cache-creation usage is parsed correctly.
- Missing usage metadata falls back to estimates.
- Stable repo/system segments are ordered before volatile user history in plan.
- Tool calls/results are never separated.
- Unsafe order references block reordering.
- Per-client cache report aggregates correctly.

---

## B. Auto-inject packs into Start Agent Session

### Goal

Use existing Repo Intelligence packs to automatically provide budgeted context at session start, without manual copy/paste, while preserving local-first and cache-friendly ordering.

### Current repo leverage

- Rust repo intelligence builds `implementation`, `verification`, and `handoff` packs.
- Node CLI has agent handoff profiles for Claude, Codex, Gemini, OpenCode, Aider, Goose, Cursor, Continue, Grok, Qwen, Amazon Q, Windsurf, and Zed.
- Existing pack output already excludes secret-like paths and is read-only.

### Implementation steps

1. Add `AgentSessionStartRequest` and `AgentSessionStartResult`.
2. Add `agent_sessions.rs`:
   - Load latest repo summary, or build one if `repoRoot` is provided.
   - Infer task kind from `clientId`, `taskKind`, and optional `userGoal`.
   - Select candidate packs.
   - Enforce `maxPackCount`, `defaultBudgetTokens`, and per-pack token budget.
   - Hash pack contents for duplicate prevention.
   - Return injection plan.
3. Add Tauri command `start_agent_session`.
4. Extend Node CLI with `--start-session` dry-run output.
5. Add UI card in `OptimizePanel`.
6. First shipping mode: dry-run and copyable output only.
7. Later mode: provider-envelope injection when the first request of a session is safely parseable and policy enables mutation.

### Pack selection defaults

| Client | Task kind | Default pack |
|---|---|---|
| `claude_code` | implementation/auto | `implementation` |
| `codex` | verification/auto | `verification` |
| editor/chat tools | handoff/auto | `handoff` |
| unknown | auto | smallest pack under budget |

### Files to add/modify

```text
src-tauri/src/optimization/agent_sessions.rs
src-tauri/src/models.rs
src-tauri/src/lib.rs
src/lib/optimizationTypes.ts
src/lib/agentSessions.ts
src/components/OptimizePanel.tsx
scripts/repo-intelligence.mjs
```

### Tests

Cases:

- Disabled auto-inject returns no injected packs.
- Dry-run returns would-inject decisions.
- Budget enforcement skips oversized packs.
- Duplicate content hash prevents reinjection.
- Pack ordering is stable and cache-friendly.
- Missing repo summary returns warnings, not panic.

---

## C. Preemptive compaction

### Goal

Prevent context-window failure and `413 compression_refused` by triggering compaction before oversized requests hit Headroom/provider failure paths.

### Current repo leverage

- Existing Codex 413 compression refusal detection and bypass is in `proxy_intercept.rs`.
- README and troubleshooting flow already recognize this as a user-facing failure mode.
- `TransformationFeedEvent` already models original and optimized token counts.

### Implementation steps

1. Add provider/model context-window metadata:

```rust
pub struct ModelContextMetadata {
    pub provider: ProviderKind,
    pub model_pattern: String,
    pub context_limit_tokens: u64,
    pub source: String,
}
```

2. Add `project_prompt_tokens(envelope)` using provider usage if present or local estimate.
3. Add `CompactionTrigger` decision logic:
   - below 70%: none
   - 70-80%: observe
   - 80-90%: warn
   - above 90%: compact if enabled, otherwise skipped event
4. Add deterministic extractive compaction:
   - Preserve system instructions.
   - Preserve current user turn.
   - Preserve recent N turns.
   - Preserve explicit task state.
   - Preserve repo facts and injected packs.
   - Preserve failing tool output, assertion diffs, stack traces, line numbers.
   - Replace old low-value history/tool output with compact summary segments.
5. Phase 1: observe and report only.
6. Phase 2: mutation when provider adapter validates serialization.
7. Keep existing 413 bypass intact as fallback.

### Files to add/modify

```text
src-tauri/src/optimization/token_estimator.rs
src-tauri/src/optimization/compaction.rs
src-tauri/src/optimization/provider_payload.rs
src-tauri/src/optimization/reports.rs
src-tauri/src/proxy_intercept.rs
src/components/TokenXrayPanel.tsx
src/components/CacheEfficiencyPanel.tsx
```

### Tests

Fixtures:

```text
src-tauri/tests/fixtures/compaction/large_openai_request.json
src-tauri/tests/fixtures/compaction/large_anthropic_request.json
```

Cases:

- No-op below threshold.
- Warning event at warn threshold.
- Trigger event above compact threshold.
- System and current user intent preserved exactly.
- Tool failure details preserved.
- Compacted fixture falls below target.
- Unknown context window skips mutation and logs warning.
- Existing 413 bypass tests still pass.

---

## D. Token X-ray

### Goal

Give users a per-session breakdown of where tokens went so waste is visible and actionable.

### Categories

- system prompt
- injected repo packs
- injected task packs
- file reads
- tool output
- conversation history
- retries
- model responses
- compaction summaries
- redundant reads
- current user turn
- unknown

### Implementation steps

1. Add `TokenCategory`, `TokenAttribution`, `SessionTokenLedger`.
2. Add ledger session creation and update helpers.
3. Add attribution from provider envelope segments.
4. Add attribution from provider response usage metadata when available.
5. Add estimation fallback.
6. Add `SavingsReport` aggregation.
7. Add Tauri command `get_token_xray_report`.
8. Add `scripts/optimization-report.mjs --token-xray --json`.
9. Add UI panel.

### Files to add/modify

```text
src-tauri/src/optimization/token_ledger.rs
src-tauri/src/optimization/reports.rs
src-tauri/src/optimization/telemetry_store.rs
src-tauri/src/models.rs
src-tauri/src/lib.rs
src/lib/optimizationTypes.ts
src/lib/tokenXray.ts
src/components/TokenXrayPanel.tsx
scripts/optimization-report.mjs
package.json
```

### Tests

- Category totals equal attribution sums.
- Missing provider usage falls back to estimated tokens.
- Cached-token savings and compaction savings show in totals.
- JSON output is stable and dashboard-friendly.
- Empty report renders a clean no-data state.

---

## E. Redundancy detection

### Goal

Detect when agents re-read unchanged file contents and quantify duplicate-read token cost.

### Implementation steps

1. Add `redundancy.rs`.
2. Extract file-read/tool-output content segments from provider envelopes.
3. Identify optional path from structured tool blocks or common text patterns.
4. Hash content using SHA-256.
5. Track by session/client/repo/path/hash.
6. Same path + same hash in same session increments duplicate count.
7. Same path + different hash marks changed file and avoids duplicate classification.
8. Persist only metadata.
9. Add report and Tauri command.

### Files to add/modify

```text
src-tauri/src/optimization/redundancy.rs
src-tauri/src/optimization/provider_payload.rs
src-tauri/src/optimization/token_ledger.rs
src-tauri/src/optimization/reports.rs
src-tauri/src/models.rs
src/components/RedundancyPanel.tsx
```

### Privacy rules

- Never store raw content.
- Store SHA-256 hash.
- Path storage configurable.
- Secret-like paths redacted even when path storage is enabled.
- Session isolation enabled by default.

### Tests

- Duplicate same path/hash counted.
- Same path/new hash treated as changed, not duplicate.
- Same hash in different sessions isolated by default.
- Raw content does not appear in persisted JSONL.
- Secret-like path is redacted.

---

## F. Model routing

### Goal

Let the switchboard route low-risk/trivial requests to cheaper models, with explicit override and safety bypasses.

### Implementation steps

1. Add `model_routing.rs`.
2. Add route table config.
3. Add deterministic classifier for low-risk tasks:
   - commit messages
   - short explanations
   - formatting changes
   - lint fixes
   - test-name generation
   - simple refactors
4. Add safety bypasses:
   - explicit model requested
   - high token count
   - ambiguous instruction
   - security/auth/crypto/payment keywords
   - database migrations
   - production incident
   - multi-file architecture
   - unsupported provider endpoint
5. Add `ModelRouteDecision` event.
6. Phase 1: observe mode only.
7. Phase 2: mutate `model` only for known-safe provider envelopes and explicit opt-in policy.

### Example route table

```json
{
  "enabled": false,
  "mode": "observe",
  "respectExplicitModel": true,
  "maxRoutableInputTokens": 12000,
  "rules": [
    {
      "taskClass": "commit_message",
      "match": ["commit message", "summarize diff"],
      "route": {
        "provider": "openai",
        "model": "gpt-4.1-mini",
        "temperature": 0.2,
        "maxTokens": 800
      }
    },
    {
      "taskClass": "short_explanation",
      "maxInputTokens": 4000,
      "route": {
        "provider": "anthropic",
        "model": "claude-3-5-haiku-latest",
        "temperature": 0.3,
        "maxTokens": 1200
      }
    }
  ]
}
```

### Files to add/modify

```text
src-tauri/src/optimization/model_routing.rs
src-tauri/src/optimization/provider_payload.rs
src-tauri/src/optimization/reports.rs
src-tauri/src/models.rs
src/components/ModelRoutingPanel.tsx
```

### Tests

- Rule matching chooses route in observe mode.
- Explicit model is preserved.
- Safety bypass blocks routing.
- Unknown provider falls back to original.
- Enabled mutation changes model only in known-safe fixture.
- Decision appears in report.

---

## G. RTK presets per framework

### Goal

Ship tuned output compression filters for Vitest, Jest, Pytest, and Cargo, preserving failure details while dropping known noise.

### Implementation steps

1. Add `rtk_presets.rs`.
2. Add preset definitions:
   - `vitest`
   - `jest`
   - `pytest`
   - `cargo`
3. Auto-detect from:
   - command string
   - manifest files
   - file names
   - output patterns
4. Preserve:
   - failure names
   - assertion diffs
   - stack traces
   - file paths
   - line numbers
   - panic/error messages
   - command summary
5. Drop/collapse:
   - passed test lists
   - repeated watcher hints
   - dependency build spam
   - progress bars
   - duplicate traceback context when safe
6. Wire into RTK hook/wrapper path without changing behavior when RTK disabled.
7. Add UI selector.

### Files to add/modify

```text
src-tauri/src/optimization/rtk_presets.rs
src-tauri/src/client_adapters.rs
src-tauri/src/tool_manager.rs
src-tauri/src/models.rs
src/lib/rtkPresets.ts
src/components/RtkPresetPanel.tsx
```

### Fixtures

```text
src-tauri/tests/fixtures/rtk/vitest-fail.txt
src-tauri/tests/fixtures/rtk/jest-fail.txt
src-tauri/tests/fixtures/rtk/pytest-fail.txt
src-tauri/tests/fixtures/rtk/cargo-fail.txt
```

### Tests

- Auto-detect each framework from command.
- Auto-detect each framework from output.
- Preserve line numbers.
- Preserve assertion diff.
- Preserve stack trace or traceback.
- Collapse passed-test noise.
- Manual preset overrides auto.
- `none` returns original output.
- RTK disabled state remains respected.

---

## 8. Observability

Use local JSONL events by default. Use remote analytics only if existing remote telemetry is explicitly enabled.

### Event: `mac_as.prompt_layout_planned`

```json
{
  "event": "mac_as.prompt_layout_planned",
  "requestId": "req_123",
  "sessionId": "ses_123",
  "clientId": "codex",
  "provider": "openai",
  "model": "gpt-5.1",
  "segments": 12,
  "stablePrefixTokens": 4200,
  "cacheableTokens": 7600,
  "reorderApplied": false,
  "unsafeToReorder": true,
  "warnings": ["tool_result_order_sensitive"]
}
```

### Event: `mac_as.cache_metrics_calculated`

```json
{
  "event": "mac_as.cache_metrics_calculated",
  "requestId": "req_123",
  "cacheableTokenRatio": 0.41,
  "cacheHitRate": 0.22,
  "cachedTokensActual": 1800,
  "cachedTokensEstimated": 2100,
  "estimatedSavingsUsd": 0.08,
  "providerMetadataComplete": true
}
```

### Event: `mac_as.pack_auto_injected`

```json
{
  "event": "mac_as.pack_auto_injected",
  "sessionId": "ses_123",
  "clientId": "claude_code",
  "repoRootHash": "sha256:...",
  "packId": "implementation",
  "estimatedTokens": 3500,
  "budgetTokens": 4000,
  "reason": "client_default_implementation",
  "dryRun": true
}
```

### Event: `mac_as.compaction_triggered`

```json
{
  "event": "mac_as.compaction_triggered",
  "requestId": "req_123",
  "model": "gpt-5.1",
  "contextLimitTokens": 128000,
  "projectedPromptTokens": 119000,
  "utilizationPct": 0.93,
  "thresholdPct": 0.9,
  "originalTokenCount": 119000,
  "compactedTokenCount": 87000,
  "savingsTokens": 32000
}
```

### Event: `mac_as.compaction_skipped`

```json
{
  "event": "mac_as.compaction_skipped",
  "requestId": "req_123",
  "reason": "feature_disabled",
  "projectedPromptTokens": 119000,
  "thresholdPct": 0.9
}
```

### Event: `mac_as.token_attribution_recorded`

```json
{
  "event": "mac_as.token_attribution_recorded",
  "sessionId": "ses_123",
  "requestId": "req_123",
  "category": "file_read",
  "tokens": 2400,
  "estimated": true,
  "sourceKind": "file_read"
}
```

### Event: `mac_as.redundancy_detected`

```json
{
  "event": "mac_as.redundancy_detected",
  "sessionId": "ses_123",
  "clientId": "codex",
  "pathHash": "sha256:...",
  "contentHash": "sha256:...",
  "tokenCount": 1800,
  "repeatedCount": 3,
  "estimatedAvoidableSpendUsd": 0.04
}
```

### Event: `mac_as.model_route_selected`

```json
{
  "event": "mac_as.model_route_selected",
  "requestId": "req_123",
  "originalModel": "gpt-5.1",
  "selectedModel": "gpt-4.1-mini",
  "taskClass": "commit_message",
  "reason": "low_risk_commit_summary",
  "estimatedSavingsUsd": 0.12,
  "overrideStatus": "none"
}
```

### Event: `mac_as.rtk_preset_applied`

```json
{
  "event": "mac_as.rtk_preset_applied",
  "command": "npm test",
  "detectedFramework": "vitest",
  "selectedPreset": "vitest",
  "originalBytes": 180000,
  "filteredBytes": 42000,
  "estimatedTokensSaved": 34500
}
```

---

## 9. Testing strategy

### 9.1 Unit tests

Rust:

```text
src-tauri/src/optimization/policy.rs
src-tauri/src/optimization/telemetry_store.rs
src-tauri/src/optimization/provider_payload.rs
src-tauri/src/optimization/token_estimator.rs
src-tauri/src/optimization/token_ledger.rs
src-tauri/src/optimization/cache_metrics.rs
src-tauri/src/optimization/prompt_layout.rs
src-tauri/src/optimization/agent_sessions.rs
src-tauri/src/optimization/compaction.rs
src-tauri/src/optimization/redundancy.rs
src-tauri/src/optimization/model_routing.rs
src-tauri/src/optimization/rtk_presets.rs
```

Frontend:

```text
src/lib/optimizationPolicy.test.ts
src/lib/tokenXray.test.ts
src/lib/cacheEfficiency.test.ts
src/lib/modelRouting.test.ts
src/lib/rtkPresets.test.ts
src/components/TokenXrayPanel.test.tsx
src/components/CacheEfficiencyPanel.test.tsx
src/components/RedundancyPanel.test.tsx
src/components/ModelRoutingPanel.test.tsx
```

### 9.2 Fixture tests

Provider fixtures:

```text
src-tauri/tests/fixtures/provider/openai_request_chat.json
src-tauri/tests/fixtures/provider/openai_request_responses.json
src-tauri/tests/fixtures/provider/openai_usage_cached.json
src-tauri/tests/fixtures/provider/anthropic_request_messages.json
src-tauri/tests/fixtures/provider/anthropic_usage_cached.json
```

Compaction fixtures:

```text
src-tauri/tests/fixtures/compaction/large_openai_request.json
src-tauri/tests/fixtures/compaction/large_anthropic_request.json
```

RTK fixtures:

```text
src-tauri/tests/fixtures/rtk/vitest-fail.txt
src-tauri/tests/fixtures/rtk/jest-fail.txt
src-tauri/tests/fixtures/rtk/pytest-fail.txt
src-tauri/tests/fixtures/rtk/cargo-fail.txt
```

### 9.3 Regression tests

Required regression checks:

- With all features disabled, provider request bytes are unchanged.
- Existing Headroom/RTK/Full/Off modes still work.
- Existing Codex 413 bypass behavior still works.
- RTK disabled state is respected and integrations are not silently re-added.
- Full-message logging remains the only path that stores raw request message content.
- Unknown provider request shapes pass through unchanged.
- Chunked or oversized requests pass through unchanged.

### 9.4 Commands

```bash
npm run test:frontend
npm run test:desktop
npm run test:all
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run check:governance
npm run check:colors
```

---

## 10. Migration strategy

### Phase 1: Instrumentation-only

- Add policy, telemetry store, token ledger.
- Add provider parser in safe bounded observe mode.
- Add cache, compaction, redundancy, and model routing “would have” events.
- No provider request mutation.

### Phase 2: Reporting

- Add Tauri report commands.
- Add UI panels.
- Add CLI JSON output.
- Add docs and privacy notes.

### Phase 3: Safe automatic optimization

- Enable Start Agent Session pack injection through explicit command.
- Enable prompt-cache layout only as plan.
- Enable preemptive compaction mutation only when validation passes.
- Keep provider-specific logic isolated.

### Phase 4: Model routing and advanced policies

- Enable model routing observe mode.
- Allow opt-in low-risk route mutation.
- Add route-table editor or config examples.
- Add Doctor checks for unsafe policy combinations.

### Backwards-compatible defaults

- All new behavior off or observe/dry-run by default.
- Existing public APIs preserved.
- Existing app modes preserved.
- No heavy dependency added.
- No live provider API calls in tests.

---

## 11. PR-sized implementation roadmap

### PR 1: Add optimization architecture doc

**Complexity:** S

**Objective:** Document current proxy/runtime architecture and planned optimization hook points.

**Inspect:**

```text
README.md
src-tauri/src/lib.rs
src-tauri/src/proxy_intercept.rs
src-tauri/src/state.rs
src-tauri/src/models.rs
src-tauri/src/client_adapters.rs
src-tauri/src/repo_intelligence.rs
scripts/repo-intelligence.mjs
```

**Modify:**

```text
docs/optimization-architecture.md
```

**Acceptance criteria:**

- Documents actual Rust intercept + Python backend flow.
- States body mutation is disabled by default.
- Documents future modules and feature flags.
- Notes that Start Agent Session is a new primitive.

**Tests/checks:**

```bash
npm run check:governance
```

**Risks:**

- Docs drift unless updated during later PRs.

---

### PR 2: Add optimization policy foundation

**Complexity:** M

**Objective:** Add safe-default local optimization policy loader.

**Inspect:**

```text
src-tauri/src/storage.rs
src-tauri/src/lib.rs
src-tauri/src/models.rs
src/lib/types.ts
```

**Modify/add:**

```text
src-tauri/src/optimization/mod.rs
src-tauri/src/optimization/policy.rs
src-tauri/src/models.rs
src-tauri/src/lib.rs
src/lib/optimizationTypes.ts
src/lib/optimizationPolicy.ts
```

**Acceptance criteria:**

- Missing policy file returns safe defaults.
- Invalid policy file returns clear error and safe fallback.
- Env overrides work.
- Tauri command exposes policy.
- No runtime behavior changes.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:frontend
```

**Risks:**

- Accidentally enabling optimization by default.

---

### PR 3: Add local telemetry store and token ledger

**Complexity:** M

**Objective:** Add privacy-preserving token ledger foundation.

**Inspect:**

```text
src-tauri/src/models.rs
src-tauri/src/state.rs
src-tauri/src/storage.rs
src-tauri/src/activity_facts.rs
src/lib/types.ts
```

**Modify/add:**

```text
src-tauri/src/optimization/telemetry_store.rs
src-tauri/src/optimization/token_ledger.rs
src-tauri/src/models.rs
src/lib/optimizationTypes.ts
```

**Acceptance criteria:**

- Can create ledger, append attribution, summarize totals.
- JSONL writes under `telemetry/`.
- Raw content is not stored.
- Category totals equal attribution sums.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

**Risks:**

- Double-counting retries or model responses.

---

### PR 4: Add provider payload parser in observe-only mode

**Complexity:** L

**Objective:** Normalize OpenAI and Anthropic payloads without modifying requests.

**Inspect:**

```text
src-tauri/src/proxy_intercept.rs
src-tauri/src/models.rs
src/lib/types.ts
```

**Modify/add:**

```text
src-tauri/src/optimization/provider_payload.rs
src-tauri/tests/fixtures/provider/openai_request_chat.json
src-tauri/tests/fixtures/provider/openai_request_responses.json
src-tauri/tests/fixtures/provider/anthropic_request_messages.json
```

**Acceptance criteria:**

- Parses OpenAI chat completions.
- Parses OpenAI responses API.
- Parses Anthropic messages API.
- Extracts model, messages, system, tool-ish content, and usage when present.
- Unknown shapes produce unsupported pass-through result.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

**Risks:**

- Provider API shape drift.
- Unsafe buffering if wired too early.

---

### PR 5: Add prompt-cache metrics and prompt layout planning

**Complexity:** M

**Objective:** Add cache metrics and layout plan in observe mode.

**Inspect:**

```text
src-tauri/src/optimization/provider_payload.rs
src-tauri/src/models.rs
src/lib/repoIntelligence.ts
```

**Modify/add:**

```text
src-tauri/src/optimization/prompt_segments.rs
src-tauri/src/optimization/prompt_layout.rs
src-tauri/src/optimization/cache_metrics.rs
src-tauri/src/optimization/reports.rs
src-tauri/tests/fixtures/provider/openai_usage_cached.json
src-tauri/tests/fixtures/provider/anthropic_usage_cached.json
src/lib/cacheEfficiency.ts
src/lib/cacheEfficiency.test.ts
```

**Acceptance criteria:**

- Calculates stable prefix tokens.
- Calculates cacheable token ratio.
- Parses OpenAI/Anthropic cached-token fields from fixtures.
- Falls back to estimates when metadata missing.
- Unsafe reorder validation blocks order-sensitive content.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:frontend
```

**Risks:**

- Misrepresenting estimated cache savings as actual savings.

---

### PR 6: Add Start Agent Session and dry-run pack injection

**Complexity:** L

**Objective:** Add explicit local session-start primitive and budgeted pack injection plan.

**Inspect:**

```text
src-tauri/src/repo_intelligence.rs
scripts/repo-intelligence.mjs
src/lib/repoIntelligence.ts
src/components/OptimizePanel.tsx
```

**Modify/add:**

```text
src-tauri/src/optimization/agent_sessions.rs
src-tauri/src/models.rs
src-tauri/src/lib.rs
src/lib/agentSessions.ts
src/components/OptimizePanel.tsx
scripts/repo-intelligence.mjs
```

**Acceptance criteria:**

- Disabled config returns no injection.
- Dry-run reports selected/skipped packs.
- Budget enforcement works.
- Duplicate prevention by hash works.
- Output order is cache-friendly.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:frontend
npm run repo:intelligence -- . --start-session --agent codex --format json
```

**Risks:**

- Over-injecting context and increasing session-start cost.

---

### PR 7: Add preemptive compaction planning

**Complexity:** L

**Objective:** Add context threshold detection and extractive compaction planning.

**Inspect:**

```text
src-tauri/src/proxy_intercept.rs
src-tauri/src/state.rs
src-tauri/src/models.rs
```

**Modify/add:**

```text
src-tauri/src/optimization/token_estimator.rs
src-tauri/src/optimization/compaction.rs
src-tauri/src/optimization/reports.rs
src-tauri/tests/fixtures/compaction/large_openai_request.json
src-tauri/tests/fixtures/compaction/large_anthropic_request.json
```

**Acceptance criteria:**

- Below threshold: no-op.
- Warn threshold: event only.
- Compact threshold: deterministic compaction result.
- Essential sections preserved.
- Existing 413 bypass behavior unchanged.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

**Risks:**

- Lossy compaction affecting answer quality.

---

### PR 8: Add Token X-ray report and UI

**Complexity:** M

**Objective:** Expose token ledger and savings report through Tauri, CLI, and UI.

**Inspect:**

```text
src-tauri/src/lib.rs
src-tauri/src/models.rs
src/components/ActivityFeed.tsx
src/components/OptimizePanel.tsx
package.json
```

**Modify/add:**

```text
src-tauri/src/optimization/reports.rs
src-tauri/src/lib.rs
src/lib/tokenXray.ts
src/lib/tokenXray.test.ts
src/components/TokenXrayPanel.tsx
src/components/TokenXrayPanel.test.tsx
scripts/optimization-report.mjs
package.json
```

**Acceptance criteria:**

- `get_token_xray_report` returns report.
- CLI prints JSON.
- UI renders no-data state and populated state.
- Reports label estimates.

**Tests/checks:**

```bash
npm run test:frontend
cargo test --manifest-path src-tauri/Cargo.toml
```

**Risks:**

- Report clutter or confusing spend estimates.

---

### PR 9: Add redundancy detection

**Complexity:** M

**Objective:** Track repeated unchanged file/tool content safely.

**Inspect:**

```text
src-tauri/src/optimization/provider_payload.rs
src-tauri/src/optimization/token_ledger.rs
src-tauri/src/repo_intelligence.rs
src-tauri/src/storage.rs
```

**Modify/add:**

```text
src-tauri/src/optimization/redundancy.rs
src-tauri/src/optimization/reports.rs
src-tauri/src/models.rs
src/components/RedundancyPanel.tsx
```

**Acceptance criteria:**

- Same session/path/hash duplicates counted.
- Same path/new hash marked changed.
- Raw content not persisted.
- Report includes redundancy percent and avoidable spend estimate.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:frontend
```

**Risks:**

- False positives from repeated snippets.
- Path privacy concerns.

---

### PR 10: Add model routing observe mode

**Complexity:** L

**Objective:** Add safe model routing decisions without mutation.

**Inspect:**

```text
src-tauri/src/proxy_intercept.rs
src-tauri/src/client_adapters.rs
src-tauri/src/pricing.rs
src-tauri/src/models.rs
```

**Modify/add:**

```text
src-tauri/src/optimization/model_routing.rs
src-tauri/src/optimization/reports.rs
src-tauri/src/models.rs
src/lib/modelRouting.ts
src/components/ModelRoutingPanel.tsx
```

**Acceptance criteria:**

- Low-risk rules produce observe decisions.
- Explicit model requests preserved.
- Safety bypasses block routing.
- Unknown provider falls back to original.
- Decision report visible.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:frontend
```

**Risks:**

- Incorrect classification.

---

### PR 11: Add opt-in model routing mutation

**Complexity:** M

**Objective:** Safely mutate model field only when policy and adapter allow it.

**Inspect:**

```text
src-tauri/src/optimization/provider_payload.rs
src-tauri/src/optimization/model_routing.rs
src-tauri/src/proxy_intercept.rs
```

**Modify/add:**

```text
src-tauri/src/optimization/provider_payload.rs
src-tauri/src/optimization/model_routing.rs
src-tauri/src/proxy_intercept.rs
```

**Acceptance criteria:**

- Disabled/default behavior byte-for-byte unchanged.
- Observe mode does not mutate.
- Enabled mode mutates only known-safe fixture payloads.
- Explicit model override prevents mutation.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

**Risks:**

- User surprise from changed model.
- Provider capability mismatch.

---

### PR 12: Add RTK framework presets

**Complexity:** M

**Objective:** Add tuned framework output filters for Vitest, Jest, Pytest, and Cargo.

**Inspect:**

```text
src-tauri/src/client_adapters.rs
src-tauri/src/tool_manager.rs
src/App.tsx
src/components/SwitchboardPanel.tsx
```

**Modify/add:**

```text
src-tauri/src/optimization/rtk_presets.rs
src-tauri/src/client_adapters.rs
src-tauri/src/tool_manager.rs
src/lib/rtkPresets.ts
src/lib/rtkPresets.test.ts
src/components/RtkPresetPanel.tsx
src-tauri/tests/fixtures/rtk/vitest-fail.txt
src-tauri/tests/fixtures/rtk/jest-fail.txt
src-tauri/tests/fixtures/rtk/pytest-fail.txt
src-tauri/tests/fixtures/rtk/cargo-fail.txt
```

**Acceptance criteria:**

- Presets auto-detect framework.
- Preserve actionable failure details.
- Collapse known noise.
- Manual preset and `none` work.
- RTK disabled state is respected.

**Tests/checks:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:frontend
```

**Risks:**

- Dropping useful test output.

---

### PR 13: Docs and examples

**Complexity:** S

**Objective:** Document feature flags, usage, reports, privacy, and limitations.

**Inspect:**

```text
README.md
PRIVACY.md
docs/
package.json
```

**Modify/add:**

```text
README.md
PRIVACY.md
docs/optimization-architecture.md
docs/token-xray.md
docs/prompt-cache.md
docs/preemptive-compaction.md
docs/model-routing.md
docs/rtk-presets.md
```

**Acceptance criteria:**

- Docs say all automation is opt-in.
- Docs show CLI examples.
- Docs explain savings estimates and provider metadata limits.
- Docs explain hashing/privacy.

**Tests/checks:**

```bash
npm run check:governance
npm run build
```

**Risks:**

- Docs overpromise exact billing savings.

---

### PR 14: Final integration hardening

**Complexity:** M

**Objective:** Regression hardening before rollout.

**Inspect:**

```text
src-tauri/src/proxy_intercept.rs
src-tauri/src/optimization/*
src-tauri/src/lib.rs
src-tauri/src/models.rs
src/lib/optimizationTypes.ts
README.md
PRIVACY.md
```

**Modify:**

```text
Any new modules needing safety fixes
Additional regression tests
```

**Acceptance criteria:**

- All features disabled or observe/dry-run by default.
- Existing proxy pass-through unchanged when disabled.
- No raw content stored by default.
- Unknown provider/streaming/chunked requests pass through.
- UI handles empty report state.
- Retention cleanup bounded.

**Tests/checks:**

```bash
npm run test:all
npm run build
npm run check:governance
npm run check:colors
```

**Risks:**

- Last-mile integration regressions around request buffering.

---

## 12. Copy-paste Codex prompts

### Prompt 1: Architecture discovery

```text
You are working in tarunag10/mac-ai-switchboard.

Before editing, inspect README.md, package.json, src-tauri/src/lib.rs, src-tauri/src/proxy_intercept.rs, src-tauri/src/state.rs, src-tauri/src/models.rs, src-tauri/src/client_adapters.rs, src-tauri/src/repo_intelligence.rs, scripts/repo-intelligence.mjs, and existing docs.

Create docs/optimization-architecture.md documenting the current architecture and planned hook points for:
- token ledger
- prompt-cache metrics
- auto-injected repo/task packs
- preemptive compaction
- redundancy detection
- model routing
- RTK framework presets

Do not change runtime behavior. Preserve existing public APIs. Clearly state that provider request mutation must remain disabled by default. Include the current proxy flow through 127.0.0.1:6767 and the managed Python backend. Run available documentation/check commands that are reasonable for this change, then summarize what changed and any risks.
```

### Prompt 2: Optimization policy foundation

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/storage.rs, src-tauri/src/lib.rs, src-tauri/src/models.rs, src/lib/types.ts, and existing tests before editing.

Add a local optimization policy foundation:
- Rust OptimizationPolicy and OptimizationPolicyPatch models with serde camelCase.
- Safe defaults where every mutation-capable feature is disabled or observe/dry-run.
- Policy load from the existing config directory as optimization-policy.json.
- Environment overrides using HEADROOM_* names.
- Tauri commands get_optimization_policy and update_optimization_policy.
- TypeScript mirror types in src/lib/optimizationTypes.ts and a small src/lib/optimizationPolicy.ts helper.

Do not wire provider request parsing or mutation yet. Preserve existing behavior. Add tests for default policy, invalid JSON fallback, and env overrides. Run cargo tests and relevant frontend tests. Summarize changed files, commands run, and risks.
```

### Prompt 3: Token ledger foundation

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/models.rs, src-tauri/src/state.rs, src-tauri/src/storage.rs, src-tauri/src/activity_facts.rs, src/lib/types.ts, and existing tests before editing.

Add a local, privacy-preserving token ledger foundation:
- Rust models for TokenCategory, TokenAttribution, SessionTokenLedger, and summary totals.
- TypeScript mirror types in src/lib/optimizationTypes.ts.
- A small JSONL telemetry store under the existing telemetry directory.
- Unit tests for category aggregation, serialization round-trip, and retention-safe writes.

Do not wire request parsing or proxy mutation yet. Do not store raw prompt or file contents. Preserve existing behavior. Run cargo tests and frontend tests that are relevant, then summarize changes and any follow-up work.
```

### Prompt 4: Provider payload parser

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/proxy_intercept.rs, src-tauri/src/models.rs, src/lib/types.ts, and the new optimization policy/token ledger modules before editing.

Implement provider payload parsing in observe-only helpers:
- Add src-tauri/src/optimization/provider_payload.rs.
- Normalize OpenAI chat completions, OpenAI responses API, and Anthropic messages API into a ProviderRequestEnvelope.
- Preserve unknown fields in parsed structures where possible.
- Return an unsupported/pass-through result for unknown shapes.
- Add fixture JSON for OpenAI and Anthropic requests.

Do not wire mutation into proxy_intercept yet. Do not buffer live request bodies yet. Add fixture tests for model extraction, message extraction, system/tool/file-ish segment extraction, and unsupported fallback. Run cargo tests and summarize.
```

### Prompt 5: Cache metrics

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/optimization/provider_payload.rs, src-tauri/src/models.rs, src/lib/types.ts, src/lib/repoIntelligence.ts, and existing repo-intelligence tests before editing.

Implement prompt-cache metrics in observe-only mode:
- Add CacheableSegment, ProviderCachePolicy, PromptLayoutPlan, CacheMetrics, and CacheEfficiencyReport Rust models and TS mirror types.
- Add provider usage parsing for OpenAI-style cached token fields and Anthropic-style cache read/create fields using fixture JSON, without live API calls.
- Add stable-prefix/cacheable-token estimation with fallback behavior when provider metadata is incomplete.
- Add validation helpers that mark prompt reordering unsafe for order-sensitive segments.

Do not mutate provider requests. Add tests for stable-prefix ordering, actual provider cached-token aggregation, fallback estimates, and per-client report aggregation. Run cargo tests and Vitest where relevant, then summarize.
```

### Prompt 6: Auto-inject packs

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/repo_intelligence.rs, src/lib/repoIntelligence.ts, scripts/repo-intelligence.mjs, src/components/OptimizePanel.tsx, and existing repo-intelligence tests before editing.

Add an explicit Start Agent Session foundation:
- Rust AgentSessionStartRequest and AgentSessionStartResult models.
- An agent_sessions module that loads the latest Repo Intelligence summary, selects default packs by client/task, applies token budgets, prevents duplicates by content hash, and returns an injection plan.
- Tauri command start_agent_session.
- Extend scripts/repo-intelligence.mjs with a dry-run --start-session mode if this fits the existing CLI style.
- Keep auto-injection disabled by default and dry-run unless explicitly enabled.

Do not inject into live provider requests yet. Add tests for enabled/disabled behavior, budget enforcement, duplicate prevention, and cache-friendly ordering. Run npm and cargo tests relevant to repo intelligence and the new module.
```

### Prompt 7: Preemptive compaction

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/proxy_intercept.rs, src-tauri/src/state.rs, src-tauri/src/models.rs, docs/codex-compression-troubleshooting.md if present, and existing 413/compression_refused tests before editing.

Add preemptive compaction planning in observe-only mode first:
- Context-window metadata per provider/model with safe unknown fallback.
- CompactionTrigger and CompactionResult models.
- Projection of prompt tokens before forwarding when a bounded JSON request body is safely parseable.
- Threshold decisions for observe/warn/compact modes.
- Deterministic extractive compaction helper that preserves system instructions, task state, repo facts, recent turns, tool failures, and unresolved user intent.

Do not enable request mutation by default. Preserve existing Codex 413 bypass behavior. Add fixture tests for below-threshold no-op, threshold trigger, preservation rules, and 413-prevention projection. Run cargo tests.
```

### Prompt 8: Token X-ray report

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect the token ledger, src-tauri/src/lib.rs, src-tauri/src/models.rs, src/components/ActivityFeed.tsx, src/components/OptimizePanel.tsx, src/lib/types.ts, and package.json before editing.

Expose Token X-ray reports:
- Add SavingsReport and report aggregation in Rust.
- Add get_token_xray_report Tauri command.
- Add src/lib/tokenXray.ts helper formatting.
- Add a small TokenXrayPanel component integrated into the existing Activity or Optimize surface.
- Add scripts/optimization-report.mjs with --token-xray --json output, reading local telemetry safely.

Handle empty/no-data state cleanly. Keep all telemetry local. Add unit tests for aggregation and JSON formatting plus component smoke tests. Run npm run test:frontend and cargo tests relevant to reports.
```

### Prompt 9: Redundancy detection

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/optimization/provider_payload.rs, src-tauri/src/optimization/token_ledger.rs, src-tauri/src/storage.rs, src-tauri/src/repo_intelligence.rs, and existing privacy/secret-path handling before editing.

Implement privacy-safe redundancy detection:
- Hash file/tool content crossing the parsed provider envelope with SHA-256.
- Store only content hash, optional path identifier, session/client/repo metadata, token counts, timestamps, and repeat counts.
- Add RedundancyRecord and RedundancyReport models.
- Add report aggregation and a Tauri command.
- Add config fields for retention and path identifier storage.

Do not persist raw content. Add tests for duplicate detection, changed-file detection, session isolation, and raw-content-not-stored assertions. Run cargo tests and relevant frontend tests.
```

### Prompt 10: Model routing observe mode

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/proxy_intercept.rs, src-tauri/src/client_adapters.rs, src-tauri/src/pricing.rs, src-tauri/src/models.rs, and provider payload parsing before editing.

Implement model routing in observe mode:
- Add route table config and ModelRouteDecision models.
- Add rule matching for commit messages, short explanations, formatting changes, lint fixes, test-name generation, and simple refactors.
- Add safety bypasses for security-sensitive tasks, ambiguous instructions, high token counts, large edits, and explicit model requests.
- Add report storage and Tauri command for routing decisions.

Do not mutate model fields unless a test-only or explicitly enabled policy says to. Preserve explicit client model requests. Add tests for rule matching, explicit override, safety bypass, fallback behavior, and estimated savings fields. Run cargo tests.
```

### Prompt 11: Model routing mutation

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/optimization/provider_payload.rs, src-tauri/src/optimization/model_routing.rs, src-tauri/src/proxy_intercept.rs, and all model-routing tests before editing.

Add opt-in model routing mutation:
- Mutation must be disabled by default.
- Observe mode must not mutate.
- Enabled mode may only mutate known-safe provider request envelopes.
- Explicit model requests must be preserved.
- Unsupported provider shapes must pass through unchanged.
- Every mutation must record a ModelRouteDecision with original model, selected model, reason, and override status.

Add disabled-default regression tests and fixture tests proving only the model field changes when enabled. Run cargo tests and summarize any safety limitations.
```

### Prompt 12: RTK presets

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect src-tauri/src/client_adapters.rs, src-tauri/src/tool_manager.rs, src/App.tsx, RTK-related tests, and addon UI copy before editing.

Add RTK framework preset support:
- Define FrameworkPreset and RtkPresetDecision models/types.
- Implement auto-detection for Vitest, Jest, Pytest, and Cargo from command strings, file names, manifests, and output patterns.
- Add fixture-based output filtering helpers that preserve failing tests, stack traces, assertion diffs, file paths, line numbers, command summaries, and actionable errors.
- Add config for preset auto/manual/none.
- Wire preset selection to the RTK hook/wrapper path without changing behavior when RTK is disabled.

Add fixtures for each framework and tests proving noisy output is compressed while failure details are preserved. Run npm and cargo tests.
```

### Prompt 13: Docs and examples

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect README.md, PRIVACY.md, docs/, package.json, and the implemented optimization modules before editing.

Update docs for:
- optimization policy and feature flags
- Token X-ray reports
- prompt-cache metrics and limitations
- auto-injected repo/task packs
- preemptive compaction
- redundancy detection privacy model
- model routing safety and override behavior
- RTK presets

Include example CLI commands and JSON snippets. Make clear that raw prompt/file content is not stored by default, all automation is opt-in, and savings are estimates unless provider usage metadata is present. Run docs/check commands available in package.json and summarize changes.
```

### Prompt 14: Final integration hardening

```text
You are working in tarunag10/mac-ai-switchboard.

Inspect all new optimization modules, src-tauri/src/proxy_intercept.rs, src-tauri/src/lib.rs, src-tauri/src/models.rs, src/lib/optimizationTypes.ts, package.json, README.md, and tests before editing.

Perform final hardening:
- Verify all new automation defaults are disabled or observe/dry-run.
- Verify current proxy behavior is byte-for-byte unchanged when features are disabled.
- Verify no raw content is stored unless existing full-message logging is explicitly enabled.
- Verify Tauri commands fail safely with useful errors.
- Verify retention cleanup is bounded.
- Verify frontend handles empty/no-data states.
- Add regression tests for disabled-default behavior.

Run npm run test:all, npm run build, and any governance/color checks that apply. Summarize exact commands run, failures, and remaining risks.
```

---

## 13. Documentation updates

### README additions

Add a section similar to:

```md
## Optimization Reports

Mac AI Switchboard can optionally produce local-only optimization reports:

- Token X-ray: per-session token attribution by category.
- Cache efficiency: estimated and provider-reported prompt-cache savings.
- Redundancy report: repeated unchanged file reads, stored as hashes and metadata.
- Compaction events: projected context-window pressure and proactive compaction.
- Model routing decisions: observe-mode or enabled route choices with safety reasons.

All reports are disabled by default and stored locally under the existing Headroom application-support directory.
```

### CLI docs

```bash
npm run repo:intelligence -- . --manifest
npm run repo:intelligence -- . --start-session --agent codex --auto-inject-packs --format json

node scripts/optimization-report.mjs --token-xray --json
node scripts/optimization-report.mjs --cache --client codex --json
node scripts/optimization-report.mjs --redundancy --repo . --json
```

### Privacy docs

Add:

```md
Redundancy detection stores SHA-256 hashes, token counts, timestamps, client/session metadata, and optional path identifiers. It does not store raw file contents. Path identifiers can be disabled or redacted. Prompt content is not stored unless the existing full-message logging option is explicitly enabled.
```

### Savings limitation docs

Add:

```md
Savings are estimates unless a provider response includes explicit usage metadata. Prompt-cache accounting differs by provider. Missing cached-token fields fall back to stable-prefix estimates and should be treated as directional, not billing-grade.
```

---

## 14. Risk analysis

| Risk | Mitigation |
|---|---|
| Incorrect token estimates | Label estimates clearly; use provider usage metadata when present; keep fallback estimator conservative. |
| Unsafe prompt reordering | Default to no reordering; require provider adapter validation; never move tool calls/results or order-sensitive segments. |
| Provider API differences | Isolate provider-specific logic in `provider_payload.rs`; fixture-test OpenAI and Anthropic shapes; unsupported shapes pass through unchanged. |
| Degraded model quality from routing | Observe mode first; explicit model override; safety bypasses; per-client disable; routing decisions visible in UI/report. |
| Lossy compaction | Deterministic extractive first version; preserve essential sections; mutation disabled by default; compact only known-safe payloads. |
| Privacy concerns | No raw content by default; SHA-256 hashes only for redundancy; retention controls; path redaction; local-first storage. |
| Streaming request breakage | Keep header-only pass-through default; parse body only for bounded JSON with known `Content-Length`; chunked/unknown pass through. |
| Increased latency | O(n) parsing with max body limits; heavy aggregation deferred to report generation. |
| UI complexity | Put panels inside existing Optimize/Activity/Addons tabs first. |
| Runtime/Python mismatch | Keep Rust-side features observe/report first; use explicit capability checks before asking Python backend to mutate. |
| Pricing drift | Centralize pricing metadata and mark unknown pricing as unavailable instead of guessing. |
| Duplicated repo-intelligence logic | Reuse existing `RepoContextPack`, `RepoIntelligenceSummary`, and CLI handoff profile logic. |

---

## 15. Prioritized GitHub issue checklist

1. Document current optimization architecture and proxy hook points.
2. Add optimization policy schema and safe defaults.
3. Add JSONL telemetry store.
4. Add token ledger foundation.
5. Add provider payload parser in observe-only mode.
6. Add prompt-cache metrics and provider usage fixtures.
7. Add prompt layout planning and unsafe-reorder validation.
8. Add cache efficiency report command and frontend formatter.
9. Add explicit Start Agent Session command.
10. Add dry-run budgeted pack injection.
11. Extend repo-intelligence CLI with session-start output.
12. Add preemptive compaction threshold planner.
13. Add deterministic extractive compaction helper.
14. Add Token X-ray report command and CLI export.
15. Add Token X-ray UI panel.
16. Add redundancy detection with SHA-256 and no raw content persistence.
17. Add redundancy report UI card.
18. Add model routing route table and observe-mode decisions.
19. Add model routing report UI card.
20. Add opt-in model routing mutation for known-safe payloads.
21. Add RTK framework preset definitions.
22. Add RTK fixture filters for Vitest, Jest, Pytest, and Cargo.
23. Add RTK preset UI selector.
24. Update README with optimization report examples.
25. Update PRIVACY.md with hashing and telemetry notes.
26. Add final disabled-default regression tests.
27. Run full test/build/governance checks.
28. Cut staged rollout behind feature flags.

---

## 16. Definition of done

The feature set is ready for staged release when:

- All new automation is disabled, dry-run, or observe-only by default.
- Existing proxy behavior is unchanged with default policy.
- Token X-ray, cache efficiency, redundancy, compaction, and routing reports work from local data.
- Pack injection is available through explicit Start Agent Session flow.
- Compaction and model routing only mutate requests behind explicit opt-in policy and provider adapter validation.
- RTK presets preserve failure details in all fixture tests.
- No tests require live provider API calls.
- Privacy docs clearly explain local telemetry, hashing, retention, and raw-content behavior.
- `npm run test:all`, `npm run build`, governance checks, and relevant Cargo tests pass.
# Codex Intake Note

Status: hidden working plan, not part of the public docs index.

Commentary:
- Keep privacy disabled by default for telemetry, reports, provider payload capture, and repo/session intelligence.
- Treat the plan as PR-sized slices, starting with the Rust `src-tauri/src/optimization/` foundation before wiring UI or provider-specific behavior.
- Prefer observe-only metrics first: cache/token ledgers, deterministic compaction decisions, Token X-ray reports, and RTK presets should prove value before changing request payloads.
- Do not merge broad provider rewrites until loopback proxy safety, streaming behavior, and oversized/non-JSON refusal paths are covered by tests.
