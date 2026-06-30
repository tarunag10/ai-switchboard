import { describe, expect, it } from "vitest";

import {
  buildAddonHealthCards,
  getPlannedAddon,
  plannedAddons,
} from "./plannedAddons";
import type { ManagedTool, RuntimeStatus } from "./types";

function runtimeFixture(
  overrides: Partial<RuntimeStatus> = {},
): RuntimeStatus {
  return {
    platform: "darwin",
    supportTier: "supported",
    installed: true,
    running: true,
    starting: false,
    paused: false,
    autoPaused: false,
    proxyReachable: true,
    proxyBindAddress: "127.0.0.1:6767",
    backendStatus: {
      reachable: true,
      bindAddress: "127.0.0.1",
      port: 6768,
      defaultPort: 6768,
      fallbackRangeStart: 6770,
      fallbackRangeEnd: 6790,
    },
    headroomLearnSupported: true,
    rtk: {
      installed: true,
      enabled: true,
      version: "0.1.0",
      pathConfigured: true,
      hookConfigured: true,
      totalCommands: 12,
      totalSaved: 900,
    },
    ...overrides,
  };
}

function toolFixture(overrides: Partial<ManagedTool>): ManagedTool {
  return {
    id: "markitdown",
    name: "MarkItDown",
    description: "Local document conversion",
    runtime: "python",
    required: false,
    enabled: true,
    status: "healthy",
    sourceUrl: "https://example.com/tool",
    version: "0.0.0",
    ...overrides,
  };
}

describe("planned add-ons", () => {
  it("tracks repo intelligence as an available local-first graph tool", () => {
    const repoIntelligence = getPlannedAddon("repo_intelligence");

    expect(repoIntelligence).toMatchObject({
      name: "Repo Intelligence",
      statusLabel: "Local tool",
    });
    expect(repoIntelligence?.description).toContain("Local repo graph");
    expect(repoIntelligence?.description).toContain("copying bounded");
    expect(repoIntelligence?.description).toContain("remote graph service");
    expect(repoIntelligence?.bullets.join(" ")).toContain("Available now");
    expect(repoIntelligence?.bullets.join(" ")).toContain(
      "Repo Intelligence sidebar view",
    );
    expect(repoIntelligence?.bullets.join(" ")).toContain(
      "Sample preview stays non-copyable",
    );
    expect(repoIntelligence?.bullets.join(" ")).toContain("Still planned");
    expect(repoIntelligence?.bullets.join(" ")).toContain("reverse hubs");
    expect(repoIntelligence?.bullets.join(" ")).toContain(
      "direct repo-memory MCP UI controls",
    );
    expect(repoIntelligence?.bullets.join(" ")).toContain(
      "persistent parser index",
    );
    expect(repoIntelligence?.bullets.join(" ")).toContain("Local-first");
    expect(repoIntelligence?.healthChecks.join(" ")).toContain(
      "Secret-like paths",
    );
    expect(repoIntelligence?.healthChecks.join(" ")).toContain(
      "reverse dependency hubs",
    );
    expect(repoIntelligence?.savingsSources.join(" ")).toContain(
      "bounded context packs",
    );
    expect(repoIntelligence?.verificationCommand).toBe(
      "npm run repo:intelligence -- . --manifest",
    );
  });

  it("keeps popular coding-agent connectors explicitly planned", () => {
    const connectors = getPlannedAddon("agent_connectors");

    expect(connectors).toMatchObject({
      name: "Agent Connectors",
      statusLabel: "Planned",
    });
    expect(connectors?.description).toContain("Gemini CLI");
    expect(connectors?.description).toContain("OpenCode");
    expect(connectors?.description).toContain("Cursor");
    expect(connectors?.description).toContain("Qwen Code");
    expect(connectors?.description).toContain("Amazon Q Developer CLI");
    expect(connectors?.bullets.join(" ")).toContain("read-only detection");
    expect(connectors?.bullets.join(" ")).toContain("reversible");
    expect(connectors?.healthChecks.join(" ")).toContain(
      "Off mode must remove only Switchboard-owned changes",
    );
    expect(connectors?.savingsSources.join(" ")).toContain(
      "Repo Intelligence handoff packs",
    );
  });

  it("keeps hardening add-ons visible with verification commands", () => {
    expect(plannedAddons.map((addon) => addon.id)).toEqual([
      "repo_intelligence",
      "agent_connectors",
      "rtk_hardening",
      "ponytail_hardening",
      "markitdown_hardening",
    ]);

    for (const addon of plannedAddons) {
      expect(addon.name).not.toHaveLength(0);
      expect(addon.description).not.toHaveLength(0);
      expect(addon.bullets.length).toBeGreaterThan(0);
      expect(addon.healthChecks.length).toBeGreaterThan(0);
      expect(addon.savingsSources.length).toBeGreaterThan(0);
      expect(addon.verificationCommand).toEqual(expect.any(String));
    }
  });

  it("builds live health cards for healthy runtime and enabled add-ons", () => {
    const cards = buildAddonHealthCards(runtimeFixture(), [
      toolFixture({ id: "markitdown", name: "MarkItDown", version: "1.0.0" }),
      toolFixture({
        id: "ponytail",
        name: "Ponytail",
        runtime: "plugin",
        version: "2.0.0",
      }),
    ]);

    expect(cards.map((card) => [card.id, card.statusLabel, card.tone])).toEqual([
      ["headroom_engine", "Healthy", "healthy"],
      ["rtk", "Healthy", "healthy"],
      ["markitdown", "Healthy", "healthy"],
      ["ponytail", "Healthy", "healthy"],
    ]);
    expect(cards.find((card) => card.id === "headroom_engine")?.evidence).toContain(
      "Proxy listener: 127.0.0.1:6767.",
    );
    expect(cards.find((card) => card.id === "rtk")?.evidence).toContain(
      "Tokens saved: 900.",
    );
  });

  it("surfaces degraded runtime and incomplete RTK wiring as actionable warnings", () => {
    const cards = buildAddonHealthCards(
      runtimeFixture({
        running: false,
        proxyReachable: false,
        rtk: {
          installed: true,
          enabled: false,
          pathConfigured: true,
          hookConfigured: false,
        },
      }),
      [],
    );

    expect(cards.find((card) => card.id === "headroom_engine")).toMatchObject({
      statusLabel: "Needs attention",
      tone: "warning",
      nextAction: "Use Start runtime or run Doctor from Home.",
    });
    expect(cards.find((card) => card.id === "rtk")).toMatchObject({
      statusLabel: "Needs attention",
      tone: "warning",
      nextAction: "Use Enable or run Doctor to repair shell wiring.",
    });
    expect(cards.find((card) => card.id === "markitdown")).toMatchObject({
      statusLabel: "Not installed",
      tone: "offline",
    });
  });
});
