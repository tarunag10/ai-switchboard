import { describe, expect, it } from "vitest";

import { prepareStartAgentSessionPack } from "./agentSessionPacks";

const candidates = [
  {
    id: "handoff",
    label: "Handoff",
    markdown: "handoff pack",
    estimatedTokens: 900,
  },
  {
    id: "implementation",
    label: "Implementation",
    markdown: "implementation pack",
    estimatedTokens: 700,
    cacheableTokens: 650,
  },
];

describe("prepareStartAgentSessionPack", () => {
  it("injects the implementation pack as a stable prefix when it fits", () => {
    const preparation = prepareStartAgentSessionPack({
      agent: "codex",
      task: "build the next slice",
      tokenBudget: 1_000,
      candidates,
      enabled: true,
    });

    expect(preparation.inject).toBe(true);
    expect(preparation.packId).toBe("implementation");
    expect(preparation.remainingBudget).toBe(300);
    expect(preparation.cacheableTokens).toBe(650);
    expect(preparation.stablePrefixMarkdown).toContain("<stable-context-pack>");
  });

  it("does not inject when disabled", () => {
    const preparation = prepareStartAgentSessionPack({
      agent: "codex",
      task: "build",
      tokenBudget: 1_000,
      candidates,
      enabled: false,
    });

    expect(preparation.inject).toBe(false);
    expect(preparation.reason).toBe("pack_injection_disabled");
  });

  it("does not inject packs that exceed the budget", () => {
    const preparation = prepareStartAgentSessionPack({
      agent: "codex",
      task: "build",
      tokenBudget: 100,
      preferredPackId: "handoff",
      candidates,
      enabled: true,
    });

    expect(preparation.inject).toBe(false);
    expect(preparation.packId).toBe("handoff");
    expect(preparation.reason).toBe("context_pack_exceeds_budget");
  });
});
