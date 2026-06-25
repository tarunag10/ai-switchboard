import { describe, expect, it } from "vitest";

import { plannedConnectors } from "./plannedConnectors";

describe("planned connector mode readiness", () => {
  it("declares safe mode coverage and first automation step per tool", () => {
    for (const connector of plannedConnectors) {
      expect(connector.supportedModes.length).toBeGreaterThanOrEqual(2);
      expect(connector.supportedModes).toContain("Off");
      expect(connector.safeToday.length).toBeGreaterThan(40);
      expect(connector.firstAutomation.length).toBeGreaterThan(40);
      expect(connector.firstAutomation).toMatch(
        /backup|restore|read-only|dry-run|Doctor|wrapper|MCP|provider/i,
      );
    }
  });

  it("keeps repo-pack capable tools explicit", () => {
    const repoPackTools = plannedConnectors
      .filter((connector) => connector.supportedModes.includes("Repo packs"))
      .map((connector) => connector.id);

    expect(repoPackTools).toEqual(["cursor", "aider", "continue", "goose"]);
  });
});
