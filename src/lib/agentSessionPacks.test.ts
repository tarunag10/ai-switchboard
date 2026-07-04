import { describe, expect, it } from "vitest";

import {
  buildAgentSessionPayload,
  getAgentSessionActionLabel,
  prepareStartAgentSessionPack,
  type AgentSessionPackCandidate,
} from "./agentSessionPacks";

const candidates: AgentSessionPackCandidate[] = [
  {
    id: "handoff",
    name: "Handoff",
    summary: "Status and next steps.",
    estimatedTokens: 900,
    stablePrefix: "Use current branch context.",
  },
  {
    id: "implementation",
    name: "Implementation",
    summary: "Plan and file boundaries.",
    estimatedTokens: 700,
    cacheableTokens: 650,
    stablePrefix: "Own only target files.",
  },
];

describe("prepareStartAgentSessionPack", () => {
  it("selects the preferred pack and reports remaining/cacheable budget", () => {
    const preparation = prepareStartAgentSessionPack({
      agentId: "codex",
      task: "build workflow slice",
      tokenBudget: 1_000,
      enabled: true,
      preferredPackId: "implementation",
      candidates,
    });

    expect(preparation.inject).toBe(true);
    expect(preparation.packId).toBe("implementation");
    expect(preparation.packName).toBe("Implementation");
    expect(preparation.remainingBudget).toBe(300);
    expect(preparation.cacheableTokens).toBe(650);
    expect(preparation.stablePrefixMarkdown).toContain("<stable-context-pack>");
    expect(preparation.stablePrefixMarkdown).toContain("task: build workflow slice");
  });

  it("does not prepare when disabled", () => {
    const preparation = prepareStartAgentSessionPack({
      agentId: "codex",
      task: "build",
      tokenBudget: 1_000,
      enabled: false,
      candidates,
    });

    expect(preparation.inject).toBe(false);
    expect(preparation.reason).toBe("pack_injection_disabled");
    expect(preparation.remainingBudget).toBe(1_000);
  });

  it("blocks packs that exceed the budget", () => {
    const preparation = prepareStartAgentSessionPack({
      agentId: "codex",
      task: "build",
      tokenBudget: 100,
      enabled: true,
      preferredPackId: "handoff",
      candidates,
    });

    expect(preparation.inject).toBe(false);
    expect(preparation.packId).toBe("handoff");
    expect(preparation.reason).toBe("context_pack_exceeds_budget");
    expect(getAgentSessionActionLabel(preparation)).toContain("Increase budget");
  });

  it("builds a stable start-session payload", () => {
    const payload = buildAgentSessionPayload({
      agentId: "codex",
      task: "build",
      tokenBudget: 1_000,
      enabled: true,
      preferredPackId: "implementation",
      candidates,
    });

    expect(JSON.parse(payload)).toMatchObject({
      action: "start_agent_session",
      agent: "codex",
      injectStablePrefix: true,
      remainingBudget: 300,
      reason: "context_pack_injected",
      pack: {
        id: "implementation",
        cacheableTokens: 650,
      },
    });
    expect(payload).toContain("Own only target files.");
  });
});
