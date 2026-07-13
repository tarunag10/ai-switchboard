import { beforeEach, describe, expect, it, vi } from "vitest";
import { normalizeBriefing, normalizeXray } from "./usageAnalytics";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

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

describe("usage analytics retention contract", () => {
  beforeEach(() => invokeMock.mockReset());

  it("normalizes the versioned briefing preview and preserves an honest zero event count", async () => {
    invokeMock.mockResolvedValue({
      briefingCount: 2,
      eventCount: 0,
      dayKeys: ["2026-07-10", "2026-07-11"],
      scope: "daily_usage_briefing_snapshots_only",
      detail: "Detailed normalized event facts are not persisted yet.",
    });

    const { previewClearUsageAnalytics } = await import("./usageAnalytics");
    await expect(previewClearUsageAnalytics()).resolves.toEqual({
      briefingCount: 2,
      eventCount: 0,
      dayKeys: ["2026-07-10", "2026-07-11"],
      scope: "daily_usage_briefing_snapshots_only",
      detail: "Detailed normalized event facts are not persisted yet.",
    });
    expect(invokeMock).toHaveBeenCalledWith("preview_clear_usage_analytics");
  });

  it("supports the legacy snapshot count spelling while using the safe fallback detail", async () => {
    invokeMock.mockResolvedValue({ snapshotCount: 1, day_keys: ["2026-07-11"] });

    const { previewClearUsageAnalytics } = await import("./usageAnalytics");
    const preview = await previewClearUsageAnalytics();
    expect(preview.briefingCount).toBe(1);
    expect(preview.eventCount).toBe(0);
    expect(preview.dayKeys).toEqual(["2026-07-11"]);
    expect(preview.scope).toBe("daily_usage_briefing_snapshots_only");
    expect(preview.detail).toContain("savings ledger");
  });
});
