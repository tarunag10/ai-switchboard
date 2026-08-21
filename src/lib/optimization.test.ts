import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  defaultModelRoutingExperimentPolicy,
  defaultOptimizationActionPolicy,
  exportModelRoutingEvidence,
  formatCompactNumber,
  getPromptCacheAction,
  getRedundancyTokens,
  getTokenReductionPercent,
  loadModelRoutingExperimentPolicy,
  loadOptimizationActionPolicy,
  loadOptimizationSnapshot,
  normalizeOptimizationSnapshot,
  runPreemptiveCompaction,
  recordModelRoutingEvidence,
  saveModelRoutingExperimentPolicy,
  saveOptimizationActionPolicy,
  validateModelRouting,
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

  it("keeps token x-ray empty when no live telemetry exists", () => {
    const snapshot = normalizeOptimizationSnapshot({});

    expect(snapshot.tokenXray.originalTokens).toBe(0);
    expect(snapshot.tokenXray.optimizedTokens).toBe(0);
    expect(snapshot.tokenXray.buckets).toEqual([]);
  });

  it("normalizes backend compaction and agent pack schema", () => {
    const snapshot = normalizeOptimizationSnapshot({
      compaction: {
        shouldCompact: true,
        contextUsedPercent: 88,
        thresholdPercent: 72,
        reason: "preemptive threshold exceeded",
      },
      agentPack: {
        source: "repo-pack",
        injected: true,
        lastInjectedAt: "2026-07-04T00:00:00.000Z",
        status: "good",
      },
    });

    expect(snapshot.compaction.state).toBe("blocked");
    expect(snapshot.compaction.contextUsedPercent).toBe(88);
    expect(snapshot.compaction.triggerAtPercent).toBe(72);
    expect(snapshot.compaction.nextAction).toBe("preemptive threshold exceeded");
    expect(snapshot.agentPack.enabled).toBe(true);
    expect(snapshot.agentPack.packName).toBe("repo-pack");
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

  it("clamps token reduction, redundancy, and compact formatting", () => {
    expect(
      getTokenReductionPercent({
        originalTokens: 0,
        optimizedTokens: 20,
        systemTokens: 0,
        userTokens: 0,
        toolTokens: 0,
        packTokens: 0,
        buckets: [],
      }),
    ).toBe(0);
    expect(
      getRedundancyTokens([
        { duplicateTokens: 2 } as never,
        { duplicateTokens: 3 } as never,
      ]),
    ).toBe(5);
    expect(formatCompactNumber(12)).toBe("12");
    expect(formatCompactNumber(1200)).toMatch(/1\.2K/i);
  });

  it("normalizes buckets, client bounds, redundancy defaults, bypass, and custom presets", () => {
    const snapshot = normalizeOptimizationSnapshot({
      promptCacheClients: [
        {
          client: "codex",
          provider: "openai",
          promptTokens: -1,
          cacheReadTokens: -2,
          cacheCreationTokens: -3,
          efficiencyPercent: 140,
          proof: "local",
        },
      ],
      tokenXray: {
        originalTokens: 100,
        optimizedTokens: 50,
        systemTokens: 40,
        userTokens: 30,
        toolTokens: 20,
        packTokens: 10,
      },
      redundancy: [{ locations: ["a", "b"], duplicateTokens: -5 } as never],
      compaction: { contextUsedPercent: 1, nextAction: "Compact now" },
      agentPack: { enabled: false, packName: "Custom", message: "Idle" },
      bypass: { anthropic: true, openai: false, any: false },
      routing: [{ task: "test" } as never],
      rtkPresets: [{ id: "x", label: "X", command: "rtk x", purpose: "x" }],
      generatedAt: "now",
    });
    expect(snapshot.promptCacheClients[0]).toMatchObject({
      promptTokens: 0,
      cacheReadTokens: 0,
      cacheCreationTokens: 0,
      efficiencyPercent: 100,
    });
    expect(snapshot.tokenXray.buckets.map((item) => item.percent)).toEqual([
      40, 30, 20, 10,
    ]);
    expect(snapshot.redundancy[0]).toMatchObject({
      id: "redundancy",
      duplicateTokens: 0,
      readCount: 2,
      duplicatePercent: 0,
    });
    expect(snapshot.compaction).toMatchObject({ state: "good", nextAction: "Compact now" });
    expect(snapshot.agentPack).toMatchObject({ enabled: false, status: "watch" });
    expect(snapshot.bypass).toEqual({ anthropic: true, openai: false, any: true });
    expect(snapshot.routing).toHaveLength(1);
    expect(snapshot.rtkPresets[0].id).toBe("x");
    expect(snapshot.generatedAt).toBe("now");
  });

  it("preserves explicit buckets, compaction state, and enabled agent pack", () => {
    const bucket = { id: "x", label: "X", tokens: 5, percent: 100, source: "live" };
    const snapshot = normalizeOptimizationSnapshot({
      tokenXray: { buckets: [bucket] },
      compaction: { state: "blocked", triggerAtPercent: 80 },
      agentPack: { injected: true },
      bypass: { any: true },
    });
    expect(snapshot.tokenXray.buckets).toEqual([bucket]);
    expect(snapshot.compaction).toMatchObject({ state: "blocked", triggerAtPercent: 80 });
    expect(snapshot.agentPack).toMatchObject({ enabled: true, status: "good" });
    expect(snapshot.bypass.any).toBe(true);
  });

  it("loads and saves optimization action policy with exact payloads", async () => {
    const policy = { ...defaultOptimizationActionPolicy, maxPromptReorderItems: 8 };
    invokeMock.mockResolvedValueOnce(policy).mockResolvedValueOnce(policy);
    await expect(loadOptimizationActionPolicy()).resolves.toEqual(policy);
    await expect(saveOptimizationActionPolicy(policy)).resolves.toEqual(policy);
    expect(invokeMock.mock.calls).toEqual([
      ["get_optimization_action_policy"],
      ["set_optimization_action_policy", { policy }],
    ]);

    invokeMock.mockRejectedValueOnce(new Error("missing"));
    await expect(loadOptimizationActionPolicy()).resolves.toBe(
      defaultOptimizationActionPolicy,
    );
  });

  it("loads and saves model-routing policy with exact payloads", async () => {
    const policy = { ...defaultModelRoutingExperimentPolicy, stage: "userApproved" as const };
    invokeMock.mockResolvedValueOnce(policy).mockResolvedValueOnce(policy);
    await expect(loadModelRoutingExperimentPolicy()).resolves.toEqual(policy);
    await expect(saveModelRoutingExperimentPolicy(policy)).resolves.toEqual(policy);
    expect(invokeMock.mock.calls).toEqual([
      ["get_model_routing_experiment_policy"],
      ["set_model_routing_experiment_policy", { policy }],
    ]);

    invokeMock.mockRejectedValueOnce(new Error("missing"));
    await expect(loadModelRoutingExperimentPolicy()).resolves.toBe(
      defaultModelRoutingExperimentPolicy,
    );
  });

  it("validates routing and runs preemptive compaction", async () => {
    const validation = { generatedAt: "now", policyEnabled: true, checks: [] };
    const receipt = {
      recordedAt: "now",
      triggered: true,
      contextUsedPercent: 91,
      thresholdPercent: 90,
      reason: "threshold",
      action: "compact",
    };
    invokeMock.mockResolvedValueOnce(validation).mockResolvedValueOnce(receipt);
    await expect(validateModelRouting()).resolves.toEqual(validation);
    await expect(runPreemptiveCompaction()).resolves.toEqual(receipt);
    expect(invokeMock.mock.calls).toEqual([
      ["validate_model_routing"],
      ["run_preemptive_compaction"],
    ]);
  });

  it("records and exports only the redacted observe-only routing evidence contract", async () => {
    const observation = {
      runId: "run-1",
      capturedAt: "2026-08-21T10:00:00Z",
      taskClass: "formatting",
      arm: "baseline" as const,
      baselineModel: "frontier",
      candidateModel: "fast/local",
      succeeded: true,
      successfulTaskCostMicrounits: 1000,
      qualityScoreBps: 9800,
      latencyMs: 800,
      followUpRework: false,
    };
    const artifact = { evidenceClass: "local_runtime_observation", promotionEligible: false };
    invokeMock.mockResolvedValueOnce(undefined).mockResolvedValueOnce(artifact);
    await expect(recordModelRoutingEvidence(observation)).resolves.toBeUndefined();
    await expect(exportModelRoutingEvidence("run-1", "formatting")).resolves.toEqual(artifact);
    expect(invokeMock.mock.calls).toEqual([
      ["record_model_routing_evidence", { observation }],
      ["export_model_routing_evidence", { runId: "run-1", taskClass: "formatting" }],
    ]);
  });

  it("returns a safe local compaction preview on native failure", async () => {
    invokeMock.mockRejectedValueOnce(new Error("not implemented"));
    await expect(runPreemptiveCompaction()).resolves.toMatchObject({
      triggered: false,
      contextUsedPercent: 0,
      thresholdPercent: 90,
      reason: "Local preview only",
    });
  });
});
