import { describe, expect, it } from "vitest";

import {
  buildAddonSavingsEstimate,
  buildFilteredSavingsLedger,
  buildSavingsCalculatorBreakdown,
  buildSavingsCalculatorSummary,
  buildSavingsLedgerRows,
  filterSavingsLedgerRowsByConfidence,
  formatSavingsLedgerShareText,
  groupSavingsLedgerRowsBySource,
  formatSavingsCalculatorShareText,
  summarizeSavingsLedgerRows,
  type SavingsCalculatorScope,
} from "./savingsCalculator";
import type { DashboardState } from "./types";

function dashboardFixture(
  overrides: Partial<DashboardState> = {},
): DashboardState {
  return {
    appVersion: "0.0.0",
    launchExperience: "dashboard",
    bootstrapComplete: true,
    pythonRuntimeInstalled: true,
    lifetimeRequests: 10,
    lifetimeEstimatedSavingsUsd: 4.5,
    lifetimeEstimatedTokensSaved: 2_000,
    sessionRequests: 2,
    sessionEstimatedSavingsUsd: 0.75,
    sessionEstimatedTokensSaved: 300,
    sessionSavingsPct: 30,
    outputReduction: null,
    dailySavings: [
      {
        date: "2026-06-25",
        estimatedSavingsUsd: 4.5,
        estimatedTokensSaved: 2_000,
        actualCostUsd: 2,
        totalTokensSent: 8_000,
      },
    ],
    hourlySavings: [],
    savingsHistoryLoaded: true,
    tools: [],
    clients: [],
    recentUsage: [
      {
        id: "usage-1",
        timestamp: "2026-06-25T10:00:00Z",
        client: "Codex",
        workspace: "repo",
        upstreamTarget: "openai",
        stages: [],
        estimatedInputTokens: 1_000,
        estimatedOutputTokens: 200,
        estimatedCostSavingsUsd: 0.75,
        latencyMs: 120,
        outcome: "success",
      },
    ],
    insights: [],
    requiredTermsVersion: 1,
    acceptedTermsVersion: 1,
    termsUrl: "https://example.com/terms",
    ...overrides,
  };
}

describe("savings calculator", () => {
  it.each<SavingsCalculatorScope>(["session", "overall"])(
    "returns stable %s totals",
    (scope) => {
      const summary = buildSavingsCalculatorSummary(dashboardFixture(), scope);

      expect(summary.scope).toBe(scope);
      expect(summary.savedTokens).toBe(scope === "session" ? 300 : 2_000);
      expect(summary.savedUsd).toBe(scope === "session" ? 0.75 : 4.5);
      expect(summary.conservativeSavedUsd).toBe(
        scope === "session" ? 0.375 : 2.25,
      );
      expect(summary.requests).toBe(scope === "session" ? 2 : 10);
      expect(summary.savingsPct).toBe(20);
    },
  );

  it("does not invent a percentage before usage exists", () => {
    const summary = buildSavingsCalculatorSummary(
      dashboardFixture({
        lifetimeRequests: 0,
        lifetimeEstimatedSavingsUsd: 0,
        lifetimeEstimatedTokensSaved: 0,
        sessionRequests: 0,
        sessionEstimatedSavingsUsd: 0,
        sessionEstimatedTokensSaved: 0,
        sessionSavingsPct: 0,
        dailySavings: [],
        recentUsage: [],
      }),
      "overall",
    );

    expect(summary.savingsPct).toBeNull();
    expect(summary.beforeTokens).toBe(0);
  });

  it("breaks down overall savings by runtime, RTK, and repo context", () => {
    const rows = buildSavingsCalculatorBreakdown(
      dashboardFixture(),
      "overall",
      {
        runtimeStatus: {
          platform: "darwin",
          supportTier: "supported",
          installed: true,
          running: true,
          starting: false,
          paused: false,
          autoPaused: false,
          proxyReachable: true,
          headroomLearnSupported: true,
          rtk: {
            installed: true,
            enabled: true,
            pathConfigured: true,
            hookConfigured: true,
            totalCommands: 12,
            totalSaved: 900,
            avgSavingsPct: 72,
          },
        },
        repoSavings: {
          fullScanTokens: 10_000,
          bestPackTokens: 2_000,
          bestPackTokensAvoided: 8_000,
          bestPackSavingsPct: 80,
          allPacksTokens: 4_000,
          allPacksTokensAvoided: 6_000,
          allPacksSavingsPct: 60,
          bestPack: {
            id: "implementation",
            title: "Implementation Pack",
            purpose: "Feature work",
            estimatedTokens: 2_000,
            savingsVsFullScanPct: 80,
            files: [],
          },
        },
      },
    );

    expect(rows.map((row) => row.id)).toEqual([
      "headroom",
      "rtk",
      "repo_intelligence",
    ]);
    expect(rows.map((row) => row.confidence)).toEqual([
      "estimated",
      "measured",
      "inferred",
    ]);
    expect(rows[0].savedUsd).toBe(4.5);
    expect(rows[1].savedTokens).toBe(900);
    expect(rows[2].detail).toContain("Implementation Pack");
    expect(rows[2].detail).toContain("graph summary");
  });

  it("appends inferred add-on rows when their estimates avoid tokens", () => {
    const rows = buildSavingsCalculatorBreakdown(dashboardFixture(), "overall", {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
          avgSavingsPct: 72,
        },
      },
      repoSavings: {
        fullScanTokens: 10_000,
        bestPackTokens: 2_000,
        bestPackTokensAvoided: 8_000,
        bestPackSavingsPct: 80,
        allPacksTokens: 4_000,
        allPacksTokensAvoided: 6_000,
        allPacksSavingsPct: 60,
        bestPack: {
          id: "implementation",
          title: "Implementation Pack",
          purpose: "Feature work",
          estimatedTokens: 2_000,
          savingsVsFullScanPct: 80,
          files: [],
        },
      },
      cavemanSavings: buildAddonSavingsEstimate(480, 180),
      ponytailSavings: buildAddonSavingsEstimate(1_400, 520),
      markitdownSavings: buildAddonSavingsEstimate(3_200, 900),
    });

    expect(rows.map((row) => row.id)).toEqual([
      "headroom",
      "rtk",
      "repo_intelligence",
      "caveman",
      "ponytail",
      "markitdown",
    ]);
    const addonRows = rows.filter((row) =>
      ["caveman", "ponytail", "markitdown"].includes(row.id),
    );
    expect(addonRows.map((row) => row.confidence)).toEqual([
      "inferred",
      "inferred",
      "inferred",
    ]);
    expect(addonRows.map((row) => row.savedUsd)).toEqual([null, null, null]);
    expect(addonRows.map((row) => row.savedTokens)).toEqual([300, 880, 2_300]);
  });

  it("omits add-on rows when the estimate is missing or avoids no tokens", () => {
    const rows = buildSavingsCalculatorBreakdown(dashboardFixture(), "overall", {
      cavemanSavings: buildAddonSavingsEstimate(200, 200),
      ponytailSavings: null,
    });

    expect(rows.map((row) => row.id)).toEqual(["headroom"]);
  });

  it("derives add-on estimates defensively", () => {
    expect(buildAddonSavingsEstimate(500, 200)).toMatchObject({
      tokensAvoided: 300,
      savingsPct: 60,
    });
    expect(buildAddonSavingsEstimate(100, 400)).toMatchObject({
      tokensAvoided: 0,
      savingsPct: 0,
    });
    expect(buildAddonSavingsEstimate(0, 0).savingsPct).toBe(0);
  });

  it("does not show lifetime RTK totals in the session breakdown", () => {
    const rows = buildSavingsCalculatorBreakdown(
      dashboardFixture(),
      "session",
      {
        runtimeStatus: {
          platform: "darwin",
          supportTier: "supported",
          installed: true,
          running: true,
          starting: false,
          paused: false,
          autoPaused: false,
          proxyReachable: true,
          headroomLearnSupported: true,
          rtk: {
            installed: true,
            enabled: true,
            pathConfigured: true,
            hookConfigured: true,
            totalCommands: 12,
            totalSaved: 900,
          },
        },
      },
    );

    expect(rows.map((row) => row.id)).toEqual(["headroom"]);
  });

  it("builds scoped ledger rows with confidence labels", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture(),
      "overall",
      "2026-06-27T10:00:00.000Z",
      {
        runtimeStatus: {
          platform: "darwin",
          supportTier: "supported",
          installed: true,
          running: true,
          starting: false,
          paused: false,
          autoPaused: false,
          proxyReachable: true,
          headroomLearnSupported: true,
          rtk: {
            installed: true,
            enabled: true,
            pathConfigured: true,
            hookConfigured: true,
            totalCommands: 12,
            totalSaved: 900,
          },
        },
      },
    );

    expect(rows.map((row) => row.scope)).toEqual(["overall", "overall"]);
    expect(rows.map((row) => row.recordedAt)).toEqual([
      "2026-06-27T10:00:00.000Z",
      "2026-06-27T10:00:00.000Z",
    ]);
    expect(rows.map((row) => row.confidence)).toEqual([
      "estimated",
      "measured",
    ]);
    expect(rows.map((row) => row.source)).toEqual(["headroom_engine", "rtk"]);
    expect(rows[0].caveat).toContain("Estimated from saved history");
    expect(rows[1].caveat).toContain("Observed from local counters");
  });

  it("summarizes ledger confidence buckets without upgrading inferred rows", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture(),
      "overall",
      "2026-06-27T10:00:00.000Z",
      {
        runtimeStatus: {
          platform: "darwin",
          supportTier: "supported",
          installed: true,
          running: true,
          starting: false,
          paused: false,
          autoPaused: false,
          proxyReachable: true,
          headroomLearnSupported: true,
          rtk: {
            installed: true,
            enabled: true,
            pathConfigured: true,
            hookConfigured: true,
            totalCommands: 12,
            totalSaved: 900,
          },
        },
        repoSavings: {
          fullScanTokens: 10_000,
          bestPackTokens: 2_500,
          bestPackTokensAvoided: 7_500,
          bestPackSavingsPct: 75,
          allPacksTokens: 4_000,
          allPacksTokensAvoided: 6_000,
          allPacksSavingsPct: 60,
          bestPack: {
            id: "implementation",
            title: "Implementation pack",
            purpose: "Build next slice",
            estimatedTokens: 2_500,
            savingsVsFullScanPct: 75,
            files: [],
          },
        },
        ponytailSavings: buildAddonSavingsEstimate(1_400, 520),
      },
    );
    const summary = summarizeSavingsLedgerRows(
      rows,
      "overall",
      "2026-06-27T10:00:00.000Z",
    );

    expect(rows.map((row) => row.source)).toEqual([
      "headroom_engine",
      "rtk",
      "repo_intelligence",
      "ponytail",
    ]);
    expect(summary).toMatchObject({
      rowCount: 4,
      measuredTokens: 900,
      estimatedTokens: 2_000,
      inferredTokens: 8_380,
      totalTokens: 11_280,
      estimatedUsd: 4.5,
      measuredUsd: 0,
    });
  });

  it("groups ledger rows by source and preserves the time window", () => {
    const recordedAt = "2026-06-27T10:00:00.000Z";
    const rows = buildSavingsLedgerRows(dashboardFixture(), "overall", recordedAt, {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
        },
      },
      repoSavings: {
        fullScanTokens: 10_000,
        bestPackTokens: 2_500,
        bestPackTokensAvoided: 7_500,
        bestPackSavingsPct: 75,
        allPacksTokens: 4_000,
        allPacksTokensAvoided: 6_000,
        allPacksSavingsPct: 60,
        bestPack: {
          id: "implementation",
          title: "Implementation pack",
          purpose: "Build next slice",
          estimatedTokens: 2_500,
          savingsVsFullScanPct: 75,
          files: [],
        },
      },
    });
    const groups = groupSavingsLedgerRowsBySource(rows, "overall", recordedAt);

    expect(groups.map((group) => group.source)).toEqual([
      "headroom_engine",
      "rtk",
      "repo_intelligence",
    ]);
    expect(groups.map((group) => group.scope)).toEqual([
      "overall",
      "overall",
      "overall",
    ]);
    expect(groups.map((group) => group.recordedAt)).toEqual([
      recordedAt,
      recordedAt,
      recordedAt,
    ]);
    expect(groups.map((group) => group.confidence)).toEqual([
      "estimated",
      "measured",
      "inferred",
    ]);
    expect(groups[2]).toMatchObject({
      label: "Repo Intelligence",
      inferredTokens: 7_500,
      measuredTokens: 0,
      estimatedTokens: 0,
    });
  });

  it("returns no source groups for an empty ledger", () => {
    expect(
      groupSavingsLedgerRowsBySource(
        [],
        "session",
        "2026-06-27T10:00:00.000Z",
      ),
    ).toEqual([]);
  });

  it("filters ledger rows by confidence before grouping and copying", () => {
    const recordedAt = "2026-06-27T10:00:00.000Z";
    const rows = buildSavingsLedgerRows(dashboardFixture(), "overall", recordedAt, {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
        },
      },
      repoSavings: {
        fullScanTokens: 10_000,
        bestPackTokens: 2_500,
        bestPackTokensAvoided: 7_500,
        bestPackSavingsPct: 75,
        allPacksTokens: 4_000,
        allPacksTokensAvoided: 6_000,
        allPacksSavingsPct: 60,
        bestPack: {
          id: "implementation",
          title: "Implementation pack",
          purpose: "Build next slice",
          estimatedTokens: 2_500,
          savingsVsFullScanPct: 75,
          files: [],
        },
      },
      ponytailSavings: buildAddonSavingsEstimate(1_400, 520),
    });

    expect(filterSavingsLedgerRowsByConfidence(rows, "measured").map((row) => row.source)).toEqual([
      "rtk",
    ]);

    const inferred = buildFilteredSavingsLedger(
      rows,
      "overall",
      recordedAt,
      "inferred",
    );
    expect(inferred.summary).toMatchObject({
      rowCount: 2,
      measuredTokens: 0,
      estimatedTokens: 0,
      inferredTokens: 8_380,
      totalTokens: 8_380,
    });
    expect(inferred.groups.map((group) => group.source)).toEqual([
      "repo_intelligence",
      "ponytail",
    ]);

    const copied = formatSavingsLedgerShareText(
      inferred.rows,
      "overall",
      recordedAt,
    );
    expect(copied).toContain("Rows: 2");
    expect(copied).toContain("- repo_intelligence: Repo Intelligence");
    expect(copied).toContain("- ponytail: Ponytail");
    expect(copied).not.toContain("- rtk: RTK");
    expect(copied).not.toContain("- headroom_engine: Headroom");
  });

  it("formats a copyable savings ledger with confidence caveats", () => {
    const recordedAt = "2026-06-27T10:00:00.000Z";
    const rows = buildSavingsLedgerRows(dashboardFixture(), "overall", recordedAt, {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
        },
      },
      markitdownSavings: buildAddonSavingsEstimate(3_200, 900),
    });
    const text = formatSavingsLedgerShareText(rows, "overall", recordedAt);

    expect(text).toContain("Mac AI Switchboard savings ledger (overall history)");
    expect(text).toContain("Recorded: 2026-06-27T10:00:00.000Z");
    expect(text).toContain("Measured tokens: 900 / $0.00");
    expect(text).toContain("Estimated tokens: 2,000 / $4.50");
    expect(text).toContain("Inferred tokens: 2,300");
    expect(text).toContain(
      "Confidence labels are not interchangeable: inferred rows are never reported as measured.",
    );
    expect(text).toContain(
      "- markitdown: MarkItDown (inferred) saved 2,300 tokens.",
    );
  });

  it("formats a copyable session savings summary", () => {
    const dashboard = dashboardFixture();
    const summary = buildSavingsCalculatorSummary(dashboard, "session");
    const rows = buildSavingsCalculatorBreakdown(dashboard, "session");
    const text = formatSavingsCalculatorShareText(summary, rows);

    expect(text).toContain("Mac AI Switchboard savings (current app session)");
    expect(text).toContain("Saved: 300 tokens / $0.75");
    expect(text).toContain(
      "Confidence: measured = observed local counters; estimated = saved history or cost estimate; inferred = modelled template or context-pack delta.",
    );
    expect(text).toContain(
      "- headroom_engine: Headroom (measured) saved 300 tokens / $0.75",
    );
  });

  it("formats a copyable overall savings summary with local sources", () => {
    const dashboard = dashboardFixture();
    const summary = buildSavingsCalculatorSummary(dashboard, "overall");
    const rows = buildSavingsCalculatorBreakdown(dashboard, "overall", {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
          avgSavingsPct: 72,
        },
      },
      repoSavings: {
        fullScanTokens: 10_000,
        bestPackTokens: 2_500,
        bestPackTokensAvoided: 7_500,
        bestPackSavingsPct: 75,
        allPacksTokens: 4_000,
        allPacksTokensAvoided: 6_000,
        allPacksSavingsPct: 60,
        bestPack: {
          id: "implementation",
          title: "Implementation pack",
          purpose: "Build next slice",
          estimatedTokens: 2_500,
          savingsVsFullScanPct: 75,
          files: [],
        },
      },
      cavemanSavings: buildAddonSavingsEstimate(480, 180),
      ponytailSavings: buildAddonSavingsEstimate(1_400, 520),
      markitdownSavings: buildAddonSavingsEstimate(3_200, 900),
    });
    const text = formatSavingsCalculatorShareText(summary, rows);

    expect(text).toContain("Mac AI Switchboard savings (overall history)");
    expect(text).toContain("Saved: 2,000 tokens / $4.50");
    expect(text).toContain("Confidence: measured = observed local counters");
    expect(text).toContain(
      "- headroom_engine: Headroom (estimated) saved 2,000 tokens / $4.50",
    );
    expect(text).toContain("- rtk: RTK (measured) saved 900 tokens");
    expect(text).toContain(
      "- repo_intelligence: Repo Intelligence (inferred) saved 7,500 tokens",
    );
    expect(text).toContain("- caveman: Caveman (inferred) saved 300 tokens");
    expect(text).toContain("- ponytail: Ponytail (inferred) saved 880 tokens");
    expect(text).toContain(
      "- markitdown: MarkItDown (inferred) saved 2,300 tokens",
    );
  });
});
