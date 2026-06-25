import { describe, expect, it } from "vitest";

import { codexConcurrencyGuidance } from "./codexConcurrencyGuidance";

describe("codexConcurrencyGuidance", () => {
  it("recommends RTK only when Codex is routed through Headroom", () => {
    expect(codexConcurrencyGuidance("full", "Codex, Claude Code")).toEqual({
      title: "Running several Codex goals?",
      body: "Use RTK only for multiple heavy active Codex chats or goals; keep Full optimization for one main Codex session after compacting context.",
    });

    expect(codexConcurrencyGuidance("headroom", "codex")).not.toBeNull();
  });

  it("stays quiet when Codex is not routed through Headroom", () => {
    expect(codexConcurrencyGuidance("rtk", "Codex, Claude Code")).toBeNull();
    expect(codexConcurrencyGuidance("off", "Codex")).toBeNull();
    expect(codexConcurrencyGuidance("full", "Claude Code")).toBeNull();
  });
});
