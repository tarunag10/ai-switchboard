import { describe, expect, it } from "vitest";

import { getPlannedAddon, plannedAddons } from "./plannedAddons";

describe("planned add-ons", () => {
  it("tracks repo intelligence as a planned local-first graph capability", () => {
    const repoIntelligence = getPlannedAddon("repo_intelligence");

    expect(repoIntelligence).toMatchObject({
      name: "Repo Intelligence",
      statusLabel: "Planned",
    });
    expect(repoIntelligence?.description).toContain("Local repo graph");
    expect(repoIntelligence?.description).toContain("context-pack foundation");
expect(repoIntelligence?.description).toContain("path-based relationship graph");
    expect(repoIntelligence?.bullets.join(" ")).toContain("Foundation added");
    expect(repoIntelligence?.bullets.join(" ")).toContain("Not complete yet");
    expect(repoIntelligence?.bullets.join(" ")).toContain("path-based import/dependency edges");
expect(repoIntelligence?.bullets.join(" ")).toContain("reverse dependency hubs");
    expect(repoIntelligence?.bullets.join(" ")).toContain("Graphy-style");
    expect(repoIntelligence?.bullets.join(" ")).toContain("tree-sitter");
    expect(repoIntelligence?.bullets.join(" ")).toContain("repomix-style");
    expect(repoIntelligence?.bullets.join(" ")).toContain("MCP repo-memory");
    expect(repoIntelligence?.bullets.join(" ")).toContain("persistent parser index");
    expect(repoIntelligence?.bullets.join(" ")).toContain("Local-first");
expect(repoIntelligence?.healthChecks.join(" ")).toContain("Secret-like paths");
expect(repoIntelligence?.healthChecks.join(" ")).toContain("reverse dependency hubs");
expect(repoIntelligence?.savingsSources.join(" ")).toContain("bounded context packs");
expect(repoIntelligence?.verificationCommand).toBe("npm run repo:intelligence -- . --manifest");
  });

  it("keeps planned add-on ids stable for UI rendering", () => {
    expect(plannedAddons.map((addon) => addon.id)).toEqual([
      "repo_intelligence",
      "agent_connectors",
    ]);
  });

  it("tracks popular planned coding-agent connectors", () => {
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
expect(connectors?.description).toContain("Windsurf");
expect(connectors?.description).toContain("Zed AI");
expect(connectors?.description).toContain("Grok / xAI CLI");
    expect(connectors?.bullets.join(" ")).toContain("read-only detection");
    expect(connectors?.bullets.join(" ")).toContain("reversible local");
    expect(connectors?.bullets.join(" ")).toContain("Doctor repair");
  });
});
