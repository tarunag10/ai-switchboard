import { describe, expect, it } from "vitest";
import { normalizeBriefing, normalizeXray } from "./usageAnalytics";

describe("usage analytics contract normalization", () => {
  it("preserves confidence and derives pressure only from a credible limit", () => {
    const snapshot = normalizeXray({
      generated_at: 20,
      freshness: "live",
      metrics: { input_tokens: { value: 120, confidence: "measured", source: "runtime" } },
      context_pressure: { used_tokens: 120, limit_tokens: 400, limit_source: "model metadata" },
    });
    expect(snapshot.metrics.inputTokens.confidence).toBe("measured");
    expect(snapshot.contextPressure.percent).toBe(30);
    expect(snapshot.contextPressure.limitSource).toBe("model metadata");
  });

  it("normalizes a partial briefing and limits recommendations", () => {
    const briefing = normalizeBriefing({
      day_key: "2026-07-11", completeness: "partial",
      totals: { input_tokens: { value: 900, confidence: "estimated" } },
      recommendations: [{ title: "One" }, { title: "Two" }, { title: "Three" }, { title: "Four" }],
    });
    expect(briefing.totals.spentTokens.confidence).toBe("estimated");
    expect(briefing.recommendations).toHaveLength(3);
    expect(briefing.totals.savedTokens.value).toBeNull();
  });
});
