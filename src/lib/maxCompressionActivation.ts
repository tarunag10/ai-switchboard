import {
  createOptimizationLifecycleReceipt,
  filterActivatableOptimizationEngineIds,
  optimizationEngineIds,
  type OptimizationEngineId,
  type OptimizationReceipt,
} from "./optimizationEngines";
import { recommendExactCacheDefault } from "./exactCacheDefaultPolicy";
import type { SwitchboardModeId } from "./exactCacheDefaultPolicy";

export const MAX_COMPRESSION_ENGINE_CANDIDATES = [
  "headroom-native",
  "semantic-cache",
  "rtk",
] as const satisfies readonly OptimizationEngineId[];

export const MAX_COMPRESSION_EXCLUDED_ENGINE_IDS = [
  "leanctx",
  "llmlingua-2",
  "chonkify",
  "pxpipe-text-image",
] as const satisfies readonly OptimizationEngineId[];

export type MaxCompressionStepId =
  | "full-mode"
  | "headroom-native"
  | "semantic-cache"
  | "rtk"
  | "repo-index";

export interface MaxCompressionStep {
  id: MaxCompressionStepId;
  label: string;
  detail: string;
  engineId?: OptimizationEngineId;
}

export interface MaxCompressionExcludedEngine {
  id: OptimizationEngineId;
  reason: string;
}

export interface MaxCompressionActivationPlan {
  version: 1;
  engines: OptimizationEngineId[];
  steps: MaxCompressionStep[];
  excludedEngines: MaxCompressionExcludedEngine[];
  excludedCopy: string;
  suggestDoctorRerun: boolean;
}

export interface MaxCompressionActivationInput {
  mode?: SwitchboardModeId;
  semanticCacheEnabled?: boolean;
  proxyReachable?: boolean;
}

export function resolveMaxCompressionActivatableEngines(): OptimizationEngineId[] {
  return filterActivatableOptimizationEngineIds([
    ...MAX_COMPRESSION_ENGINE_CANDIDATES,
  ]);
}

export function describeMaxCompressionExcludedEngines(): MaxCompressionExcludedEngine[] {
  return MAX_COMPRESSION_EXCLUDED_ENGINE_IDS.map((id) => ({
    id,
    reason: `Blocked until promotion gates pass for ${id}.`,
  }));
}

export function createMaxCompressionActivationPlan(
  input: MaxCompressionActivationInput = {},
): MaxCompressionActivationPlan {
  const engines = resolveMaxCompressionActivatableEngines();
  const cacheRecommendation = recommendExactCacheDefault({
    mode: input.mode ?? "full",
    semanticCacheEnabled: input.semanticCacheEnabled ?? false,
    proxyReachable: input.proxyReachable ?? false,
  });
  const steps: MaxCompressionStep[] = [
    {
      id: "full-mode",
      label: "Enable Full optimization",
      detail: "Route managed clients through Headroom on loopback.",
    },
    {
      id: "headroom-native",
      label: "Headroom native compression",
      detail: "Use the supported Headroom native engine for live requests.",
      engineId: "headroom-native",
    },
  ];

  if (cacheRecommendation.recommend) {
    steps.push({
      id: "semantic-cache",
      label: "Enable Exact Response Cache",
      detail: cacheRecommendation.reason,
      engineId: "semantic-cache",
    });
  }

  steps.push(
    {
      id: "rtk",
      label: "Install and enable RTK",
      detail: "Compress shell command output before it reaches agent context.",
      engineId: "rtk",
    },
    {
      id: "repo-index",
      label: "Open Repo Intelligence index",
      detail: "Index the active repository before starting an agent session.",
    },
  );

  const excludedEngines = describeMaxCompressionExcludedEngines();
  const excludedCopy = [
    "Max compression does not enable experimental engines:",
    excludedEngines.map((engine) => engine.id).join(", "),
    "leanctx remains shadow-only even when promotion evidence exists.",
  ].join(" ");

  return {
    version: 1,
    engines,
    steps,
    excludedEngines,
    excludedCopy,
    suggestDoctorRerun: true,
  };
}

export function createMaxCompressionLifecycleReceipts(
  plan: MaxCompressionActivationPlan,
  action = "max-compression-activate",
): OptimizationReceipt[] {
  return plan.engines.map((engine) =>
    createOptimizationLifecycleReceipt(engine, action),
  );
}

export function assertMaxCompressionAllowlistMatchesRegistry(): string[] {
  const errors: string[] = [];
  for (const id of MAX_COMPRESSION_ENGINE_CANDIDATES) {
    if (!optimizationEngineIds.includes(id)) {
      errors.push(`unknown engine id in max compression allowlist: ${id}`);
    }
  }
  for (const id of MAX_COMPRESSION_EXCLUDED_ENGINE_IDS) {
    if (resolveMaxCompressionActivatableEngines().includes(id)) {
      errors.push(`excluded engine is activatable: ${id}`);
    }
  }
  return errors;
}
