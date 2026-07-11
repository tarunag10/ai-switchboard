import { invoke } from "@tauri-apps/api/core";

export type AgentMemoryTarget = "codex" | "claude" | "shared" | "repo_memory_mcp";
export type AgentMemoryScope = "global" | "repo" | "nested" | "session" | "unknown";
export type AgentMemoryStatus = "live" | "stale" | "duplicate" | "app-managed" | "user-managed" | "blocked" | "missing";

export interface AgentMemorySecretScan {
  status: "safe" | "blocked" | "unavailable";
  reason: string | null;
  categories: string[];
}

export interface AgentMemorySource {
  id: string;
  agent: AgentMemoryTarget;
  sourcePath: string;
  scope: AgentMemoryScope;
  status: AgentMemoryStatus;
  managedBySwitchboard: boolean;
  estimatedTokens: number | null;
  duplicateTokens: number | null;
  cacheableTokens: number | null;
  modifiedAt: string | null;
  secretScan: AgentMemorySecretScan;
  recommendedAction: string | null;
  previewAvailable: boolean;
  rollbackAvailable: boolean;
}

export interface AgentMemorySnapshot {
  generatedAt: string | null;
  repoPath: string | null;
  sources: AgentMemorySource[];
}

export interface AgentMemoryCompactionPreview {
  agent: AgentMemoryTarget;
  sourcePath: string | null;
  beforeTokens: number | null;
  afterTokens: number | null;
  duplicateTokensRemoved: number | null;
  secretScan: AgentMemorySecretScan;
  diff: string | null;
  summary: string | null;
  confirmationPhrase: string | null;
  applyEligible: boolean;
  applyBlockedReason: string | null;
}

/** A content-free proof of a Switchboard-managed memory change. */
export interface AgentMemoryCompactionReceipt {
  receiptId: string;
  agent: AgentMemoryTarget;
  sourcePath: string | null;
  backupPath: string | null;
  appliedAt: string | null;
  rollbackAvailable: boolean;
  summary: string | null;
  rollbackConfirmationPhrase: string | null;
}

const record = (value: unknown): Record<string, any> => value && typeof value === "object" ? value as Record<string, any> : {};
const list = <T,>(value: unknown): T[] => Array.isArray(value) ? value as T[] : [];
const target = (value: unknown): AgentMemoryTarget => ["codex", "claude", "shared", "repo_memory_mcp"].includes(String(value)) ? value as AgentMemoryTarget : "shared";
const scope = (value: unknown): AgentMemoryScope => ["global", "repo", "nested", "session"].includes(String(value)) ? value as AgentMemoryScope : "unknown";
const status = (value: unknown): AgentMemoryStatus => ["live", "stale", "duplicate", "app-managed", "user-managed", "blocked", "missing"].includes(String(value)) ? value as AgentMemoryStatus : "missing";
const number = (value: unknown): number | null => typeof value === "number" && Number.isFinite(value) ? value : null;

function secretScan(raw: unknown): AgentMemorySecretScan {
  const value = record(raw);
  const rawStatus = value.status ?? value.state;
  return {
    status: rawStatus === "blocked" || rawStatus === "unsafe" ? "blocked" : rawStatus === "safe" || rawStatus === "clear" ? "safe" : "unavailable",
    reason: typeof value.reason === "string" ? value.reason : typeof value.detail === "string" ? value.detail : null,
    categories: list<unknown>(value.categories ?? value.matches).map(String).filter(Boolean),
  };
}

function source(raw: unknown, index: number): AgentMemorySource {
  const value = record(raw);
  return {
    id: String(value.id ?? value.sourcePath ?? value.source_path ?? `memory-source-${index}`),
    agent: target(value.agent),
    sourcePath: String(value.sourcePath ?? value.source_path ?? "Path unavailable"),
    scope: scope(value.scope),
    status: status(value.status),
    managedBySwitchboard: Boolean(value.managedBySwitchboard ?? value.managed_by_switchboard),
    estimatedTokens: number(value.estimatedTokens ?? value.estimated_tokens),
    duplicateTokens: number(value.duplicateTokens ?? value.duplicate_tokens),
    cacheableTokens: number(value.cacheableTokens ?? value.cacheable_tokens),
    modifiedAt: typeof (value.modifiedAt ?? value.modified_at) === "string" ? value.modifiedAt ?? value.modified_at : null,
    secretScan: secretScan(value.secretScan ?? value.secret_scan),
    recommendedAction: typeof (value.recommendedAction ?? value.recommended_action) === "string" ? value.recommendedAction ?? value.recommended_action : null,
    previewAvailable: Boolean(value.previewAvailable ?? value.preview_available),
    rollbackAvailable: Boolean(value.rollbackAvailable ?? value.rollback_available ?? ((value.status === "applied") && (value.rollbackConfirmationPhrase ?? value.rollback_confirmation_phrase))),
  };
}

export function normalizeAgentMemorySnapshot(raw: unknown): AgentMemorySnapshot {
  const value = record(raw);
  return {
    generatedAt: typeof (value.generatedAt ?? value.generated_at) === "string" ? value.generatedAt ?? value.generated_at : null,
    repoPath: typeof (value.repoPath ?? value.repo_path) === "string" ? value.repoPath ?? value.repo_path : null,
    sources: list(value.sources ?? value.memorySources ?? value.memory_sources).map(source),
  };
}

export function normalizeAgentMemoryPreview(raw: unknown): AgentMemoryCompactionPreview {
  const value = record(raw);
  const sources = list<Record<string, any>>(value.sources);
  const beforeTokens = number(value.beforeTokens ?? value.before_tokens) ?? sources.reduce<number | null>((total, item) => total === null ? null : total + (number(item.beforeTokens ?? item.before_tokens) ?? 0), sources.length ? 0 : null);
  const afterTokens = number(value.afterTokens ?? value.after_tokens) ?? sources.reduce<number | null>((total, item) => total === null ? null : total + (number(item.afterTokens ?? item.after_tokens) ?? 0), sources.length ? 0 : null);
  const removed = number(value.duplicateTokensRemoved ?? value.duplicate_tokens_removed) ?? sources.reduce<number | null>((total, item) => total === null ? null : total + (number(item.estimatedTokensSaved ?? item.estimated_tokens_saved) ?? 0), sources.length ? 0 : null);
  const blocked = Boolean(value.blockedBySecrets ?? value.blocked_by_secrets);
  const warnings = list<unknown>(value.warnings).map(String).filter(Boolean);
  const structuralDiff = sources.flatMap((item) => list<unknown>(item.diffSummary ?? item.diff_summary).map(String)).filter(Boolean).join("\n");
  return {
    agent: target(value.agent), sourcePath: typeof (value.sourcePath ?? value.source_path) === "string" ? value.sourcePath ?? value.source_path : typeof sources[0]?.sourcePath === "string" ? sources[0].sourcePath : typeof sources[0]?.source_path === "string" ? sources[0].source_path : null,
    beforeTokens, afterTokens, duplicateTokensRemoved: removed,
    secretScan: blocked ? { status: "blocked", reason: warnings[0] ?? "Secret scan blocked this compaction preview.", categories: [] } : secretScan(value.secretScan ?? value.secret_scan),
    diff: typeof value.diff === "string" ? value.diff : structuralDiff || null, summary: typeof value.summary === "string" ? value.summary : warnings[0] ?? null,
    confirmationPhrase: typeof (value.confirmationPhrase ?? value.confirmation_phrase) === "string" ? value.confirmationPhrase ?? value.confirmation_phrase : null,
    applyEligible: Boolean(value.applyEligible ?? value.apply_eligible),
    applyBlockedReason: typeof (value.applyBlockedReason ?? value.apply_blocked_reason) === "string" ? value.applyBlockedReason ?? value.apply_blocked_reason : null,
  };
}

export function normalizeAgentMemoryReceipt(raw: unknown): AgentMemoryCompactionReceipt {
  const value = record(raw);
  return {
    receiptId: String(value.receiptId ?? value.receipt_id ?? value.id ?? ""),
    agent: target(value.agent),
    sourcePath: typeof (value.sourcePath ?? value.source_path ?? value.targetPath ?? value.target_path) === "string" ? value.sourcePath ?? value.source_path ?? value.targetPath ?? value.target_path : null,
    backupPath: typeof (value.backupPath ?? value.backup_path) === "string" ? value.backupPath ?? value.backup_path : null,
    appliedAt: typeof (value.appliedAt ?? value.applied_at) === "string" ? value.appliedAt ?? value.applied_at : null,
    rollbackAvailable: Boolean(value.rollbackAvailable ?? value.rollback_available),
    summary: typeof value.summary === "string" ? value.summary : typeof value.status === "string" ? `Memory compaction ${value.status}.` : null,
    rollbackConfirmationPhrase: typeof (value.rollbackConfirmationPhrase ?? value.rollback_confirmation_phrase) === "string" ? value.rollbackConfirmationPhrase ?? value.rollback_confirmation_phrase : null,
  };
}

export async function getAgentMemorySnapshot(repoPath?: string): Promise<AgentMemorySnapshot> {
  const raw = await invoke("get_agent_memory_snapshot", repoPath?.trim() ? { repoPath: repoPath.trim() } : undefined);
  return normalizeAgentMemorySnapshot(raw);
}

export async function previewAgentMemoryCompaction(repoPath: string, agent: AgentMemoryTarget): Promise<AgentMemoryCompactionPreview> {
  const raw = await invoke("preview_agent_memory_compaction", { repoPath, agent });
  return normalizeAgentMemoryPreview(raw);
}

export async function applyAgentMemoryCompaction(repoPath: string, agent: AgentMemoryTarget, confirmationPhrase: string): Promise<AgentMemoryCompactionReceipt> {
  const raw = await invoke("apply_agent_memory_compaction", { repoPath, agent, confirmationPhrase });
  const receipts = list(raw);
  if (!receipts.length) throw new Error("The Agent Memory backend did not return a change receipt.");
  return normalizeAgentMemoryReceipt(receipts[0]);
}

export async function rollbackAgentMemoryCompaction(receiptId: string, confirmationPhrase: string): Promise<AgentMemoryCompactionReceipt> {
  const raw = await invoke("rollback_agent_memory_compaction", { receiptId, confirmationPhrase });
  return normalizeAgentMemoryReceipt(raw);
}

export function canApplyAgentMemoryCompaction(source: AgentMemorySource, preview: AgentMemoryCompactionPreview | null): boolean {
  return Boolean(source.managedBySwitchboard && source.previewAvailable && source.status !== "blocked" && source.status !== "user-managed" && source.secretScan.status === "safe" && preview && preview.secretScan.status === "safe" && preview.applyEligible && preview.confirmationPhrase);
}

export function formatMemoryTokens(value: number | null) { return value === null ? "Unavailable" : new Intl.NumberFormat("en-US", { notation: value >= 1000 ? "compact" : "standard", maximumFractionDigits: 1 }).format(value); }

export function buildSafeMemorySummary(snapshot: AgentMemorySnapshot): string {
  const lines = ["# AI Switchboard Agent Memory summary", "", `Sources: ${snapshot.sources.length}`, ""];
  for (const item of snapshot.sources) lines.push(`- ${item.agent} · ${item.scope} · ${item.status} · ${formatMemoryTokens(item.estimatedTokens)} tokens · secret scan: ${item.secretScan.status}`);
  lines.push("", "This summary intentionally excludes memory contents and paths.");
  return lines.join("\n");
}
