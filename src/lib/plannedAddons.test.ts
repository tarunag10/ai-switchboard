import { describe, expect, it } from "vitest";

import { getPlannedAddon, plannedAddons } from "./plannedAddons";

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
});
