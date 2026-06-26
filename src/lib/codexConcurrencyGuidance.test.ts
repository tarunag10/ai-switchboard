import { describe, expect, it } from "vitest";

import { codexConcurrencyGuidance } from "./codexConcurrencyGuidance";

describe("codexConcurrencyGuidance", () => {
  it("recommends RTK only when Codex is routed through Headroom", () => {
    expect(codexConcurrencyGuidance("full", "Codex, Claude Code")).toEqual({
      title: "Running several Codex goals?",
      body: "Headroom compression is best for one main Codex session. Use RTK only before running several heavy active Codex chats or goals so large requests do not stall behind compression.",
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

  it("stays quiet when Codex is not routed through Headroom", () => {
    expect(codexConcurrencyGuidance("rtk", "Codex, Claude Code")).toBeNull();
    expect(codexConcurrencyGuidance("off", "Codex")).toBeNull();
    expect(codexConcurrencyGuidance("full", "Claude Code")).toBeNull();
  });
});
