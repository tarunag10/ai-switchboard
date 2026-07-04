import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getPromptCacheAction,
  getRedundancyTokens,
  getTokenReductionPercent,
  loadOptimizationSnapshot,
  normalizeOptimizationSnapshot
} from "./optimization";
import { buildPromptCacheEfficiency } from "./promptCache";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args)
}));

describe("optimization helpers", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("calculates prompt-cache efficiency from segments", () => {
    const efficiency = buildPromptCacheEfficiency([
      {
        id: "rules",
        label: "Rules",
        tokens: 100,
        cacheableTokens: 80,
        hitTokens: 60,
        misses: 1
      },
      {
        id: "turn",
        label: "Turn",
        tokens: 50,
        cacheableTokens: 20,
        hitTokens: 10,
        misses: 2
      }
    ]);

    expect(efficiency.totalTokens).toBe(150);
    expect(efficiency.cacheableTokens).toBe(100);
    expect(efficiency.hitTokens).toBe(70);
    expect(efficiency.efficiencyPercent).toBe(70);
  });

  it("normalizes raw telemetry and derives token savings", () => {
    const snapshot = normalizeOptimizationSnapshot({
      promptCacheSegments: [
        {
          id: "pack",
          label: "Pack",
          tokens: 1000,
          cacheableTokens: 800,
          hitTokens: 400,
          misses: 2
        }
      ],
      tokenXray: {
        originalTokens: 2000,
        optimizedTokens: 1200
      },
      redundancy: [
        {
          id: "dupe",
          label: "Duplicate prompt",
          duplicateTokens: 250,
          locations: ["A", "B"],
          action: "Remove duplicate.",
          readCount: 2,
          duplicatePercent: 12,
          proof: "same content hash observed twice",
        }
      ]
    });

    expect(snapshot.promptCache.efficiencyPercent).toBe(50);
    expect(getTokenReductionPercent(snapshot.tokenXray)).toBe(40);
    expect(getRedundancyTokens(snapshot.redundancy)).toBe(250);
    expect(getPromptCacheAction(snapshot)).toMatch(/Pin reusable headers/);
  });

  it("preserves empty provider cache telemetry", () => {
    const snapshot = normalizeOptimizationSnapshot({
      promptCacheClients: [],
    });

    expect(snapshot.promptCacheClients).toEqual([]);
  });

  it("loads Tauri telemetry when available", async () => {
    invokeMock.mockResolvedValue({
      promptCacheSegments: [
        {
          id: "rules",
          label: "Rules",
          tokens: 100,
          cacheableTokens: 100,
          hitTokens: 90,
          misses: 0
        }
      ],
      generatedAt: "2026-07-04T00:00:00.000Z"
    });

    const snapshot = await loadOptimizationSnapshot();

    expect(invokeMock).toHaveBeenCalledWith("get_optimization_snapshot");
    expect(snapshot.source).toBe("tauri");
    expect(snapshot.promptCache.efficiencyPercent).toBe(90);
  });

  it("falls back when Tauri telemetry is not implemented yet", async () => {
    invokeMock.mockRejectedValue(new Error("unknown command"));

    const snapshot = await loadOptimizationSnapshot();

    expect(snapshot.source).toBe("fallback");
    expect(snapshot.rtkPresets.length).toBeGreaterThan(0);
  });
});
