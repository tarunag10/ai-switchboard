import { describe, expect, it } from "vitest";

import {
  buildCompressionDashboardOverview,
  formatCompressionConfidence,
} from "./compressionDashboard";

const baseDashboard = {
  sessionEstimatedTokensSaved: 4_200,
  lifetimeEstimatedTokensSaved: 128_000,
} as const;

describe("compressionDashboard", () => {
  it("returns empty attribution without fabricating zero measured rows", () => {
    const overview = buildCompressionDashboardOverview({ scope: "today" });

    expect(overview.sources).toHaveLength(0);
    expect(overview.compressionTokensSaved).toBeNull();
    expect(overview.caveats.some((line) => line.includes("omitted"))).toBe(true);
  });

  it("builds a full overview with confidence labels for each family", () => {
    const overview = buildCompressionDashboardOverview({
      scope: "session",
      dashboard: baseDashboard as never,
      runtimeStatus: {
        rtk: { totalSaved: 900, daily: [], installed: true, enabled: true },
      } as never,
      semanticCache: {
        enabled: true,
        hits: 12,
        misses: 3,
        evidence: "estimated until counterfactual provider evidence",
      },
      repoSavings: {
        bestPackTokensAvoided: 5_500,
        bestPack: { title: "Implementation Pack" },
      } as never,
      addonRows: [
        {
          id: "ponytail",
          label: "Ponytail",
          tokensSaved: 880,
          confidence: "inferred",
          detail: "Bounded change slices vs an unbounded rewrite baseline.",
        },
      ],
      generatedAt: "2026-08-04T12:00:00.000Z",
    });

    expect(overview.sources.map((row) => row.family)).toEqual([
      "headroom",
      "rtk",
      "cache",
      "repo-intelligence",
      "addon",
    ]);
    expect(overview.sources.find((row) => row.family === "headroom")).toMatchObject({
      confidence: "measured",
      tokensSaved: 4_200,
    });
    expect(overview.sources.find((row) => row.family === "rtk")).toMatchObject({
      confidence: "measured",
      tokensSaved: 900,
    });
    expect(overview.sources.find((row) => row.family === "cache")).toMatchObject({
      tokensSaved: null,
      confidence: "estimated",
    });
    expect(overview.sources.find((row) => row.family === "repo-intelligence")).toMatchObject({
      confidence: "estimated",
    });
    expect(overview.sources.find((row) => row.family === "addon")).toMatchObject({
      confidence: "inferred",
    });
    expect(overview.compressionTokensSaved).toBe(11_480);
    expect(overview.compressionConfidence).toBe("measured");
  });

  it("omits families without positive savings data", () => {
    const overview = buildCompressionDashboardOverview({
      scope: "today",
      dashboard: {
        sessionEstimatedTokensSaved: 0,
        lifetimeEstimatedTokensSaved: 0,
      } as never,
      runtimeStatus: {
        rtk: { totalSaved: 0, daily: [], installed: false, enabled: false },
      } as never,
      semanticCache: { enabled: false, hits: 0, misses: 0 },
    });

    expect(overview.sources).toHaveLength(0);
    expect(overview.compressionTokensSaved).toBeNull();
  });

  it("formats confidence labels for UI copy", () => {
    expect(formatCompressionConfidence("measured")).toBe("Measured");
    expect(formatCompressionConfidence("inferred")).toBe("Inferred");
  });

  it("adds content-class rows when Headroom exposes buckets", () => {
    const overview = buildCompressionDashboardOverview({
      scope: "session",
      dashboard: baseDashboard as never,
      contentClass: {
        toolResultTokens: 400,
        historyTokens: null,
        userMessageTokens: 200,
      },
    });

    expect(overview.sources.some((row) => row.id === "headroom-tool-results")).toBe(true);
    expect(overview.sources.some((row) => row.id === "headroom-user-messages")).toBe(true);
    expect(overview.sources.some((row) => row.id === "headroom-history")).toBe(false);
  });
});
