import { describe, expect, it } from "vitest";

import { codexConcurrencyGuidance } from "./codexConcurrencyGuidance";
import type { UsageEvent } from "./types";

function usageEvent(overrides: Partial<UsageEvent>): UsageEvent {
  return {
    id: "usage-1",
    timestamp: "2026-06-30T10:00:00Z",
    client: "Codex",
    workspace: "repo",
    upstreamTarget: "openai",
    stages: [],
    estimatedInputTokens: 1_000,
    estimatedOutputTokens: 200,
    estimatedCostSavingsUsd: 0.1,
    latencyMs: 100,
    outcome: "success",
    ...overrides,
  };
}

describe("codexConcurrencyGuidance", () => {
  it("recommends RTK only when Codex is routed through Headroom", () => {
    expect(codexConcurrencyGuidance("full", "Codex, Claude Code")).toEqual({
      title: "Running several Codex goals?",
      body: "Headroom compression is best for one main Codex session. Use RTK only before running several heavy active Codex chats or goals so large requests do not stall behind compression.",
      riskLabel: "Preventive guidance",
      riskTone: "watch",
      evidence: [
        "No recent Codex token events in this app session yet.",
        "Guidance is based on Codex being routed through Headroom.",
      ],
      policies: [
        "Full: one main Codex session",
        "RTK only: 2+ heavy sessions",
        "After 413: compact, then reset Codex in Doctor",
        "Unsupported model: Repair Codex setup",
      ],
      steps: [
        "Switch to RTK only before opening several active Codex chats or goals.",
        "Compact or close stale Codex conversations before turning Headroom routing back on.",
        "If Codex was bypassed after a 413 compression_refused error, run Doctor to reset the bypass.",
        "If Codex says the model is unsupported with a ChatGPT account, use Doctor's Repair Codex action instead.",
      ],
      recommendedMode: "rtk",
      actionLabel: "Switch to RTK only",
    });
    expect(codexConcurrencyGuidance("headroom", "codex")).not.toBeNull();
  });

  it("raises a high-pressure warning from large recent Codex requests", () => {
    const guidance = codexConcurrencyGuidance("full", "Codex", [
      usageEvent({
        estimatedInputTokens: 130_000,
        estimatedOutputTokens: 4_000,
      }),
      usageEvent({
        id: "claude",
        client: "Claude Code",
        estimatedInputTokens: 500_000,
        estimatedOutputTokens: 1_000,
      }),
    ]);

    expect(guidance).toMatchObject({
      riskLabel: "High context pressure",
      riskTone: "high",
      body: "Recent Codex traffic is large enough that Headroom compression can stall. Compact the largest conversation or switch to RTK only before opening more heavy Codex work.",
      evidence: [
        "1 recent Codex request.",
        "Largest recent Codex request: 134,000 tokens.",
        "Recent Codex total: 134,000 tokens.",
      ],
    });
  });

  it("watches multiple medium Codex requests before they fail compression", () => {
    const guidance = codexConcurrencyGuidance("full", "Codex", [
      usageEvent({ id: "one", estimatedInputTokens: 30_000 }),
      usageEvent({ id: "two", estimatedInputTokens: 35_000 }),
    ]);

    expect(guidance?.riskLabel).toBe("Context pressure watch");
    expect(guidance?.riskTone).toBe("watch");
    expect(guidance?.evidence).toContain("2 recent Codex requests.");
  });

  it("stays quiet when Codex is not routed through Headroom", () => {
    expect(codexConcurrencyGuidance("rtk", "Codex, Claude Code")).toBeNull();
    expect(codexConcurrencyGuidance("off", "Codex")).toBeNull();
    expect(codexConcurrencyGuidance("full", "Claude Code")).toBeNull();
  });
});
