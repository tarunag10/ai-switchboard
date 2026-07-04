export interface PromptCacheSegment {
  id: string;
  label: string;
  tokens: number;
  cacheableTokens: number;
  hitTokens: number;
  misses: number;
}

export interface PromptCacheEfficiency {
  totalTokens: number;
  cacheableTokens: number;
  hitTokens: number;
  missTokens: number;
  efficiencyPercent: number;
  estimatedTokensSaved: number;
  segments: PromptCacheSegment[];
}

export function clampPercent(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(100, Math.round(value)));
}

export function buildPromptCacheEfficiency(
  segments: PromptCacheSegment[]
): PromptCacheEfficiency {
  const totalTokens = segments.reduce((sum, segment) => sum + segment.tokens, 0);
  const cacheableTokens = segments.reduce(
    (sum, segment) => sum + segment.cacheableTokens,
    0
  );
  const hitTokens = segments.reduce((sum, segment) => sum + segment.hitTokens, 0);
  const missTokens = Math.max(cacheableTokens - hitTokens, 0);
  const efficiencyPercent = clampPercent((hitTokens / Math.max(cacheableTokens, 1)) * 100);

  return {
    totalTokens,
    cacheableTokens,
    hitTokens,
    missTokens,
    efficiencyPercent,
    estimatedTokensSaved: hitTokens,
    segments
  };
}

export function recommendPromptCacheAction(
  efficiency: PromptCacheEfficiency
): string {
  if (efficiency.cacheableTokens === 0) {
    return "No stable prompt blocks detected yet.";
  }
  if (efficiency.efficiencyPercent >= 80) {
    return "Cache alignment healthy.";
  }
  if (efficiency.missTokens > efficiency.hitTokens) {
    return "Move repo map, rules, and pack headers before volatile user text.";
  }
  return "Pin reusable headers and avoid timestamp churn inside cached blocks.";
}
