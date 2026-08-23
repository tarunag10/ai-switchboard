import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  clearUsageAnalytics,
  exportDailyUsageBriefing,
  formatMetric,
  loadDailyUsageBriefing,
  loadDailyUsageBriefingHistory,
  loadUsageAnalyticsEvents,
  loadTokenXraySnapshot,
  metricLabel,
  normalizeBriefing,
  normalizeXray,
} from "./usageAnalytics";

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

  it.each([
    [null, "unavailable"],
    [{ context_pressure: { used_tokens: 60, limit_tokens: 100 } }, "medium"],
    [{ context_pressure: { used_tokens: 80, limit_tokens: 100 } }, "high"],
    [{ context_pressure: { used_tokens: 95, limit_tokens: 100 } }, "critical"],
    [{ context_pressure: { percent: 10, band: "normal" } }, "low"],
    [{ context_pressure: { percent: 65, band: "elevated" } }, "medium"],
  ])("normalizes pressure alternate %j", (raw, expectedBand) => {
    expect(normalizeXray(raw).contextPressure.band).toBe(expectedBand);
  });

  it("normalizes rich snake-case xray collections and fallback labels", () => {
    const snapshot = normalizeXray({
      schema_version: 3,
      generated_at: "2026-01-01T00:00:00.000Z",
      session_id: "s1",
      metrics: {
        saved_tokens: 12,
        input_tokens: { value: "bad", observed_at: 42 },
      },
      context_pressure: {
        used_tokens: "bad",
        limit_tokens: 0,
        projected_percent: 75,
      },
      sources: [
        { source: "cache", tokens_saved: 5, evidence: ["hit", "exact"] },
        {},
      ],
      timeline: [
        { occurred_at: "2026-01-01T00:00:00.000Z", kind: "cache" },
        {},
      ],
      anomalies: [{ message: "Spike", evidence: ["large"] }, {}],
    });
    expect(snapshot.schemaVersion).toBe(3);
    expect(snapshot.generatedAt).toBeGreaterThan(0);
    expect(snapshot.metrics.savedTokens).toMatchObject({
      value: 12,
      confidence: "inferred",
    });
    expect(snapshot.metrics.inputTokens).toMatchObject({
      value: null,
      confidence: "unavailable",
      observedAt: 42,
    });
    expect(snapshot.sources[0]).toMatchObject({
      id: "cache",
      label: "cache",
      detail: "hit · exact",
      eventCount: 0,
      runtimeEvidenceUnits: 0,
      measuredEventCount: 0,
      estimatedEventCount: 0,
      inferredEventCount: 0,
      totalTokensSent: 0,
    });
    expect(snapshot.sources[1]).toMatchObject({
      id: "source",
      label: "Optimization source",
    });
    expect(snapshot.timeline[0]).toMatchObject({ title: "cache", confidence: "inferred" });
    expect(snapshot.timeline[1]).toMatchObject({ id: "1", title: "Usage event" });
    expect(snapshot.anomalies[0]).toMatchObject({ title: "Spike", detail: "large" });
    expect(snapshot.anomalies[1]).toMatchObject({ id: "1", severity: "warning" });
  });

  it("normalizes rich briefing fallbacks and evidence coverage spellings", () => {
    const briefing = normalizeBriefing({
      schema_version: 2,
      generated_at: "invalid",
      agents: [
        {
          provider: "openai",
          requests: "3",
          input_tokens: 100,
          saved_tokens: 10,
          cache_read_tokens: 5,
          estimated_cost_usd: 1.25,
          highest_context_percent: 80,
          failures: 2,
        },
        {},
      ],
      attention_items: [{ evidence: ["one", "two"] }, {}],
      recommendations: [
        { ruleId: "r1", actionLabel: "Act", evidence: ["proof"], priorityScore: 9 },
        { reason: "because" },
      ],
      evidence_coverage: {
        measured_sources: 1,
        estimated_sources: 2,
        inferred_sources: 3,
        unavailable_metrics: 4,
        notes: ["partial", "local"],
      },
    });
    expect(briefing.generatedAt).toBe(0);
    expect(briefing.agents[0]).toMatchObject({
      id: "0",
      label: "openai",
      requests: 3,
      detail: "2 failures",
      highestContextPercent: 80,
    });
    expect(briefing.agents[1].label).toBe("Unattributed agent");
    expect(briefing.attentionItems[0]).toMatchObject({
      title: "Needs attention",
      detail: "one · two",
    });
    expect(briefing.recommendations[0]).toMatchObject({
      id: "r1",
      title: "Act",
      evidence: "proof",
      priority: 9,
    });
    expect(briefing.recommendations[1].evidence).toBe("because");
    expect(briefing.evidenceCoverage).toMatchObject({
      measured: 1,
      estimated: 2,
      inferred: 3,
      unavailable: 4,
      detail: "partial · local",
    });
  });
});

describe("usage analytics retention contract", () => {
  beforeEach(() => invokeMock.mockReset());

  it("normalizes the versioned briefing preview and preserves event counts", async () => {
    invokeMock.mockResolvedValue({
      briefingCount: 2,
      eventCount: 0,
      dayKeys: ["2026-07-10", "2026-07-11"],
      scope: "daily_usage_briefing_snapshots_and_normalized_events",
      detail: "Normalized analytics events are retained for 30 days.",
    });

    const { previewClearUsageAnalytics } = await import("./usageAnalytics");
    await expect(previewClearUsageAnalytics()).resolves.toEqual({
      briefingCount: 2,
      eventCount: 0,
      dayKeys: ["2026-07-10", "2026-07-11"],
      scope: "daily_usage_briefing_snapshots_and_normalized_events",
      detail: "Normalized analytics events are retained for 30 days.",
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
    expect(preview.scope).toBe("daily_usage_briefing_snapshots_and_normalized_events");
    expect(preview.detail).toContain("savings ledger");
  });

  it("invokes and normalizes all analytics loaders", async () => {
    invokeMock
      .mockResolvedValueOnce({ generated_at: 5 })
      .mockResolvedValueOnce({ day_key: "2026-01-01" })
      .mockResolvedValueOnce([{ day_key: "2026-01-02" }, null]);
    await expect(loadTokenXraySnapshot()).resolves.toMatchObject({ generatedAt: 5 });
    await expect(loadDailyUsageBriefing()).resolves.toMatchObject({
      dayKey: "2026-01-01",
    });
    await expect(loadDailyUsageBriefingHistory()).resolves.toHaveLength(2);
    expect(invokeMock.mock.calls).toEqual([
      ["get_token_xray_snapshot"],
      ["get_daily_usage_briefing"],
      ["list_daily_usage_briefings"],
    ]);
  });

  it("returns an empty history for malformed native history", async () => {
    invokeMock.mockResolvedValueOnce({ not: "a list" });
    await expect(loadDailyUsageBriefingHistory()).resolves.toEqual([]);
  });

  it("normalizes bounded content-free event history", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        id: "usage-abc",
        occurred_at: "2026-08-17T10:00:00Z",
        kind: "usage",
        label: "Agent request",
        confidence: "estimated",
        input_tokens: 100,
        output_tokens: 20,
        saved_tokens: 5,
        avoided_tokens: 0,
        request_count: 1,
        latency_ms: 30,
        outcome: "success",
        source: "recent_usage",
      },
    ]);
    await expect(loadUsageAnalyticsEvents()).resolves.toEqual([
      expect.objectContaining({
        id: "usage-abc",
        occurredAt: Date.parse("2026-08-17T10:00:00Z"),
        inputTokens: 100,
        savedTokens: 5,
        confidence: "estimated",
      }),
    ]);
    expect(invokeMock).toHaveBeenCalledWith("list_usage_analytics_events");
  });

  it("normalizes clear aliases, summary fallback, and exact command", async () => {
    invokeMock.mockResolvedValueOnce({
      affectedBriefings: "4",
      deletedEvents: "2",
      dayKeys: "invalid",
      scope: 42,
      summary: "Done",
    });
    await expect(clearUsageAnalytics()).resolves.toEqual({
      briefingCount: 4,
      eventCount: 2,
      dayKeys: [],
      scope: "daily_usage_briefing_snapshots_and_normalized_events",
      detail: "Done",
    });
    expect(invokeMock).toHaveBeenCalledWith("clear_usage_analytics");
  });

  it("exports markdown, json, briefing fallback, and empty values", async () => {
    invokeMock
      .mockResolvedValueOnce({ markdown: "# Briefing" })
      .mockResolvedValueOnce({ json: "{\"ok\":true}" })
      .mockResolvedValueOnce({ briefing: { day: "today" } })
      .mockResolvedValueOnce({});
    await expect(exportDailyUsageBriefing("markdown")).resolves.toBe("# Briefing");
    await expect(exportDailyUsageBriefing("json")).resolves.toBe('{"ok":true}');
    await expect(exportDailyUsageBriefing("json")).resolves.toContain('"day"');
    await expect(exportDailyUsageBriefing("markdown")).resolves.toBe("");
  });

  it("formats unavailable, currency, compact, and standard metrics", () => {
    expect(metricLabel({ value: 1, confidence: "measured", source: "x", observedAt: null, caveat: null })).toBe("measured");
    expect(formatMetric({ value: null, confidence: "unavailable", source: "x", observedAt: null, caveat: null })).toBe("Unavailable");
    expect(formatMetric({ value: 12.3, confidence: "measured", source: "x", observedAt: null, caveat: null }, true)).toContain("$12.30");
    expect(formatMetric({ value: 1200, confidence: "measured", source: "x", observedAt: null, caveat: null })).toMatch(/1\.2K/i);
    expect(formatMetric({ value: 12.34, confidence: "measured", source: "x", observedAt: null, caveat: null })).toBe("12.3");
  });
});
