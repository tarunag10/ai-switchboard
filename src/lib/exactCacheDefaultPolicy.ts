export type SwitchboardModeId = "full" | "headroom" | "rtk" | "off";

export interface ExactCacheRecommendationInput {
  mode: SwitchboardModeId;
  semanticCacheEnabled: boolean;
  proxyReachable: boolean;
}

export interface ExactCacheRecommendation {
  recommend: boolean;
  reason: string;
}

export function recommendExactCacheDefault(
  input: ExactCacheRecommendationInput,
): ExactCacheRecommendation {
  if (input.semanticCacheEnabled) {
    return {
      recommend: false,
      reason: "Exact cache is already enabled.",
    };
  }

  if (!input.proxyReachable) {
    return {
      recommend: false,
      reason: "Exact cache requires a reachable local proxy in Full or Headroom mode.",
    };
  }

  if (input.mode === "off" || input.mode === "rtk") {
    return {
      recommend: false,
      reason: "Exact cache applies only to provider traffic in Full or Headroom mode.",
    };
  }

  return {
    recommend: true,
    reason:
      "Enable exact cache for safe deterministic requests. Cache hits stay separate from compression savings and remain opt-in for semantic replay.",
  };
}
