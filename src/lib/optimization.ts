import { invoke } from "@tauri-apps/api/core";
import {
  buildPromptCacheEfficiency,
  PromptCacheEfficiency,
  PromptCacheSegment,
  recommendPromptCacheAction
} from "./promptCache";

export type OptimizationHealth = "good" | "watch" | "blocked";

export interface TokenXraySnapshot {
  originalTokens: number;
  optimizedTokens: number;
  systemTokens: number;
  userTokens: number;
  toolTokens: number;
  packTokens: number;
}

export interface RedundancyFinding {
  id: string;
  label: string;
  duplicateTokens: number;
  locations: string[];
  action: string;
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

export interface OptimizationSnapshot {
  promptCache: PromptCacheEfficiency;
  tokenXray: TokenXraySnapshot;
  redundancy: RedundancyFinding[];
  routing: ModelRoutingDecision[];
  compaction: CompactionSignal;
  agentPack: AgentPackInjectionStatus;
  rtkPresets: RtkPreset[];
  generatedAt: string;
  source: "tauri" | "fallback";
}

export interface OptimizationActionPolicy {
  promptCacheReorderEnabled: boolean;
  preemptiveCompactionEnabled: boolean;
  modelRoutingEnabled: boolean;
  maxPromptReorderItems: number;
}

export interface RawOptimizationSnapshot {
  promptCacheSegments?: PromptCacheSegment[];
  tokenXray?: Partial<TokenXraySnapshot>;
  redundancy?: RedundancyFinding[];
  routing?: ModelRoutingDecision[];
  compaction?: Partial<CompactionSignal>;
  agentPack?: Partial<AgentPackInjectionStatus>;
  rtkPresets?: RtkPreset[];
  generatedAt?: string;
}

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
    tokenXray,
    redundancy:
      raw.redundancy && raw.redundancy.length > 0
        ? raw.redundancy
        : [
            {
              id: "duplicated-rules",
              label: "Repeated tool and edit rules",
              duplicateTokens: 1180,
              locations: ["AGENTS.md", "Start Agent pack"],
              action: "Keep one canonical rule block and inject references."
            },
            {
              id: "stale-recap",
              label: "Old rollout recap repeated in prompt",
              duplicateTokens: 740,
              locations: ["memory", "session prelude"],
              action: "Use compact memory citations instead of full recap text."
            }
          ],
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
  promptCacheReorderEnabled: false,
  preemptiveCompactionEnabled: false,
  modelRoutingEnabled: false,
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

  return {
    originalTokens,
    optimizedTokens,
    systemTokens: tokenXray?.systemTokens ?? Math.round(optimizedTokens * 0.28),
    userTokens: tokenXray?.userTokens ?? Math.round(optimizedTokens * 0.34),
    toolTokens: tokenXray?.toolTokens ?? Math.round(optimizedTokens * 0.24),
    packTokens: tokenXray?.packTokens ?? Math.round(optimizedTokens * 0.14)
  };
}
