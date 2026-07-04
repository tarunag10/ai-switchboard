import { invoke } from "@tauri-apps/api/core";
import {
  buildPromptCacheEfficiency,
  PromptCacheEfficiency,
  PromptCacheSegment,
  recommendPromptCacheAction
} from "./promptCache";

export type OptimizationHealth = "good" | "watch" | "blocked";

export interface TokenXrayBucket {
  id: string;
  label: string;
  tokens: number;
  percent: number;
  source: string;
}

export interface TokenXraySnapshot {
  originalTokens: number;
  optimizedTokens: number;
  systemTokens: number;
  userTokens: number;
  toolTokens: number;
  packTokens: number;
  buckets: TokenXrayBucket[];
}

export interface RedundancyFinding {
  id: string;
  label: string;
  duplicateTokens: number;
  locations: string[];
  action: string;
  readCount: number;
  duplicatePercent: number;
  proof: string;
}

export interface PromptCacheClientProof {
  client: string;
  provider: string;
  promptTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  efficiencyPercent: number;
  proof: string;
}

export interface ModelRoutingDecision {
  task: string;
  selectedModel: string;
  fallbackModel: string;
  reason: string;
  estimatedSavingsPercent: number;
}

export interface CompactionSignal {
  state: OptimizationHealth;
  contextUsedPercent: number;
  triggerAtPercent: number;
  nextAction: string;
}

export interface AgentPackInjectionStatus {
  enabled: boolean;
  packName: string;
  lastInjectedAt: string | null;
  status: OptimizationHealth;
  message: string;
}

export interface RtkPreset {
  id: string;
  label: string;
  command: string;
  purpose: string;
}

export interface CompressionBypassSnapshot {
  anthropic: boolean;
  openai: boolean;
  any: boolean;
}

export interface OptimizationSnapshot {
  promptCache: PromptCacheEfficiency;
  promptCacheClients: PromptCacheClientProof[];
  tokenXray: TokenXraySnapshot;
  redundancy: RedundancyFinding[];
  routing: ModelRoutingDecision[];
  compaction: CompactionSignal;
  agentPack: AgentPackInjectionStatus;
  bypass: CompressionBypassSnapshot;
  rtkPresets: RtkPreset[];
  generatedAt: string;
  source: "tauri" | "fallback";
}


export interface ModelRoutingValidationCheck {
  client: string;
  task: string;
  requestedModel: string;
  selectedModel: string;
  fallbackModel: string;
  status: string;
  reason: string;
  observeOnly: boolean;
}

export interface ModelRoutingValidationReceipt {
  generatedAt: string;
  policyEnabled: boolean;
  checks: ModelRoutingValidationCheck[];
}

export interface OptimizationActionPolicy {
  promptCacheReorderEnabled: boolean;
  preemptiveCompactionEnabled: boolean;
  modelRoutingEnabled: boolean;
  maxPromptReorderItems: number;
}

export interface RawOptimizationSnapshot {
  promptCacheSegments?: PromptCacheSegment[];
  promptCacheClients?: PromptCacheClientProof[];
  tokenXray?: Partial<TokenXraySnapshot>;
  redundancy?: RedundancyFinding[];
  routing?: ModelRoutingDecision[];
  compaction?: Partial<CompactionSignal>;
  agentPack?: Partial<AgentPackInjectionStatus>;
  bypass?: Partial<CompressionBypassSnapshot> | null;
  rtkPresets?: RtkPreset[];
  generatedAt?: string;
}

const fallbackBypass: CompressionBypassSnapshot = {
  anthropic: false,
  openai: false,
  any: false,
};

const fallbackSegments: PromptCacheSegment[] = [
  {
    id: "repo-map",
    label: "Repo map + policy",
    tokens: 7800,
    cacheableTokens: 7200,
    hitTokens: 6100,
    misses: 2
  },
  {
    id: "agent-pack",
    label: "Start Agent Session pack",
    tokens: 2800,
    cacheableTokens: 2300,
    hitTokens: 1700,
    misses: 1
  },
  {
    id: "volatile-turn",
    label: "Latest user turn",
    tokens: 1600,
    cacheableTokens: 250,
    hitTokens: 80,
    misses: 3
  }
];


export const fallbackOptimizationSnapshot: OptimizationSnapshot =
  normalizeOptimizationSnapshot({}, "fallback");

export function getTokenReductionPercent(snapshot: TokenXraySnapshot): number {
  const removed = snapshot.originalTokens - snapshot.optimizedTokens;
  return Math.max(0, Math.round((removed / Math.max(snapshot.originalTokens, 1)) * 100));
}

export function getRedundancyTokens(findings: RedundancyFinding[]): number {
  return findings.reduce((sum, finding) => sum + finding.duplicateTokens, 0);
}

export function formatCompactNumber(value: number): string {
  return new Intl.NumberFormat("en", {
    maximumFractionDigits: value >= 1000 ? 1 : 0,
    notation: value >= 1000 ? "compact" : "standard"
  }).format(value);
}


function normalizeCompressionBypass(
  bypass?: Partial<CompressionBypassSnapshot> | null,
): CompressionBypassSnapshot {
  const anthropic = Boolean(bypass?.anthropic);
  const openai = Boolean(bypass?.openai);

  return {
    anthropic,
    openai,
    any: Boolean(bypass?.any) || anthropic || openai,
  };
}

function normalizePromptCacheClients(
  clients: PromptCacheClientProof[] | undefined,
): PromptCacheClientProof[] {
  if (!clients || clients.length === 0) {
    return [];
  }
  return clients.map((client) => ({
    client: client.client,
    provider: client.provider,
    promptTokens: Math.max(0, client.promptTokens ?? 0),
    cacheReadTokens: Math.max(0, client.cacheReadTokens ?? 0),
    cacheCreationTokens: Math.max(0, client.cacheCreationTokens ?? 0),
    efficiencyPercent: Math.max(0, Math.min(100, client.efficiencyPercent ?? 0)),
    proof: client.proof,
  }));
}

function fallbackTokenBuckets(snapshot: TokenXraySnapshot): TokenXrayBucket[] {
  const rows = [
    { id: "system", label: "System prompts", tokens: snapshot.systemTokens, source: "derived" },
    { id: "user", label: "User/history", tokens: snapshot.userTokens, source: "derived" },
    { id: "tool", label: "Tool output", tokens: snapshot.toolTokens, source: "derived" },
    { id: "pack", label: "Repo pack", tokens: snapshot.packTokens, source: "derived" },
  ];
  const total = rows.reduce((sum, row) => sum + row.tokens, 0);
  return rows.map((row) => ({
    ...row,
    percent: Math.max(0, Math.min(100, Math.round((row.tokens / Math.max(total, 1)) * 100))),
  }));
}

function normalizeRedundancy(
  findings: Partial<RedundancyFinding>[] | undefined,
): RedundancyFinding[] {
  const rows =
    findings && findings.length > 0
      ? findings
      : [
          {
            id: "duplicated-rules",
            label: "Repeated tool and edit rules",
            duplicateTokens: 1180,
            locations: ["AGENTS.md", "session history"],
            action: "Deduplicate stable instruction block.",
            readCount: 3,
            duplicatePercent: 35,
            proof: "fallback duplicate hash across instruction and session history",
          },
          {
            id: "stale-recap",
            label: "Old rollout recap repeated",
            duplicateTokens: 740,
            locations: ["memory", "session recap"],
            action: "Replace with latest repo pack.",
            readCount: 2,
            duplicatePercent: 22,
            proof: "fallback duplicate hash across memory and session recap",
          },
        ];

  return rows.map((finding) => ({
    id: finding.id ?? "redundancy",
    label: finding.label ?? "Duplicate content",
    duplicateTokens: Math.max(0, finding.duplicateTokens ?? 0),
    locations: finding.locations ?? [],
    action: finding.action ?? "Avoid re-reading unchanged content.",
    readCount: Math.max(0, finding.readCount ?? finding.locations?.length ?? 0),
    duplicatePercent: Math.max(0, Math.min(100, finding.duplicatePercent ?? 0)),
    proof: finding.proof ?? "same content hash observed more than once",
  }));
}

export function normalizeOptimizationSnapshot(
  raw: RawOptimizationSnapshot,
  source: OptimizationSnapshot["source"] = "tauri"
): OptimizationSnapshot {
  const promptCache = buildPromptCacheEfficiency(
    raw.promptCacheSegments && raw.promptCacheSegments.length > 0
      ? raw.promptCacheSegments
      : fallbackSegments
  );
  const tokenXray = normalizeTokenXray(raw.tokenXray, promptCache);

  return {
    promptCache,
    promptCacheClients: normalizePromptCacheClients(raw.promptCacheClients),
    tokenXray,
    redundancy: normalizeRedundancy(raw.redundancy),
    routing:
      raw.routing && raw.routing.length > 0
        ? raw.routing
        : [
            {
              task: "Repo scan",
              selectedModel: "fast/local",
              fallbackModel: "frontier",
              reason: "Deterministic search, low reasoning risk.",
              estimatedSavingsPercent: 64
            },
            {
              task: "Patch planning",
              selectedModel: "frontier",
              fallbackModel: "fast/local",
              reason: "Cross-file judgment and conflict risk.",
              estimatedSavingsPercent: 18
            }
          ],
    compaction: {
      state: raw.compaction?.state ?? (tokenXray.optimizedTokens > 12000 ? "watch" : "good"),
      contextUsedPercent: raw.compaction?.contextUsedPercent ?? 58,
      triggerAtPercent: raw.compaction?.triggerAtPercent ?? 72,
      nextAction:
        raw.compaction?.nextAction ??
        "Pre-compact after the next tool burst if cache misses rise."
    },
    agentPack: {
      enabled: raw.agentPack?.enabled ?? true,
      packName: raw.agentPack?.packName ?? "Start Agent Session",
      lastInjectedAt: raw.agentPack?.lastInjectedAt ?? null,
      status: raw.agentPack?.status ?? "good",
      message:
        raw.agentPack?.message ??
        "Pack headers are ready for session injection."
    },
    bypass: normalizeCompressionBypass(raw.bypass),
    rtkPresets:
      raw.rtkPresets && raw.rtkPresets.length > 0
        ? raw.rtkPresets
        : [
            {
              id: "status",
              label: "Compact status",
              command: "rtk git status --short",
              purpose: "Dirty-worktree check"
            },
            {
              id: "tests",
              label: "Focused tests",
              command: "rtk npm test -- --run",
              purpose: "Failure-only Vitest output"
            },
            {
              id: "search",
              label: "Code search",
              command: "rtk rg <pattern> src",
              purpose: "Low-token repo lookup"
            }
          ],
    generatedAt: raw.generatedAt ?? new Date(0).toISOString(),
    source
  };
}

export function getPromptCacheAction(snapshot: OptimizationSnapshot): string {
  return recommendPromptCacheAction(snapshot.promptCache);
}

export async function loadOptimizationSnapshot(): Promise<OptimizationSnapshot> {
  try {
    const raw = await invoke<RawOptimizationSnapshot>("get_optimization_snapshot");
    return normalizeOptimizationSnapshot(raw, "tauri");
  } catch {
    return fallbackOptimizationSnapshot;
  }
}

export const defaultOptimizationActionPolicy: OptimizationActionPolicy = {
  promptCacheReorderEnabled: true,
  preemptiveCompactionEnabled: true,
  modelRoutingEnabled: true,
  maxPromptReorderItems: 24,
};

export async function loadOptimizationActionPolicy(): Promise<OptimizationActionPolicy> {
  try {
    return await invoke<OptimizationActionPolicy>("get_optimization_action_policy");
  } catch {
    return defaultOptimizationActionPolicy;
  }
}

export async function saveOptimizationActionPolicy(
  policy: OptimizationActionPolicy,
): Promise<OptimizationActionPolicy> {
  return invoke<OptimizationActionPolicy>("set_optimization_action_policy", {
    policy,
  });
}

function normalizeTokenXray(
  tokenXray: Partial<TokenXraySnapshot> | undefined,
  promptCache: PromptCacheEfficiency
): TokenXraySnapshot {
  const originalTokens = tokenXray?.originalTokens ?? promptCache.totalTokens + 3200;
  const optimizedTokens =
    tokenXray?.optimizedTokens ??
    Math.max(originalTokens - promptCache.estimatedTokensSaved, 0);

  const snapshot: TokenXraySnapshot = {
    originalTokens,
    optimizedTokens,
    systemTokens: tokenXray?.systemTokens ?? Math.round(optimizedTokens * 0.28),
    userTokens: tokenXray?.userTokens ?? Math.round(optimizedTokens * 0.34),
    toolTokens: tokenXray?.toolTokens ?? Math.round(optimizedTokens * 0.24),
    packTokens: tokenXray?.packTokens ?? Math.round(optimizedTokens * 0.14),
    buckets: tokenXray?.buckets ?? [],
  };
  if (snapshot.buckets.length === 0) {
    snapshot.buckets = fallbackTokenBuckets(snapshot);
  }
  return snapshot;
}

export async function validateModelRouting(): Promise<ModelRoutingValidationReceipt> {
  return invoke<ModelRoutingValidationReceipt>("validate_model_routing");
}
