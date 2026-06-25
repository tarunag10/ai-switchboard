import { describe, expect, it } from "vitest";

import { codexConcurrencyGuidance } from "./codexConcurrencyGuidance";

describe("codexConcurrencyGuidance", () => {
  it("recommends RTK only when Codex is routed through Headroom", () => {
    expect(codexConcurrencyGuidance("full", "Codex, Claude Code")).toEqual({
      title: "Running several Codex goals?",
      body: "Use RTK only for multiple heavy active Codex chats or goals. Keep Full optimization for one main Codex session after compacting context.",
      steps: [
        "Switch to RTK only before opening several active Codex chats.",
        "Compact or close stale Codex conversations before turning Headroom routing back on.",
        "If Codex was bypassed after a 413 compression_refused error, run Doctor to reset the bypass.",
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
