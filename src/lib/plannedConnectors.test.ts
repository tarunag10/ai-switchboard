import { describe, expect, it } from "vitest";

import { getPlannedConnector, plannedConnectors } from "./plannedConnectors";

describe("planned connectors", () => {
  it("tracks popular coding CLIs and editor agents beyond Claude Code and Codex", () => {
    expect(plannedConnectors.map((connector) => connector.id)).toEqual([
      "gemini_cli",
      "opencode",
      "cursor",
      "grok_cli",
      "aider",
      "continue",
      "goose",
    ]);
  });

  it("keeps every planned connector explicit about local reversible setup", () => {
    for (const connector of plannedConnectors) {
      expect(connector.statusLabel).toBe("Planned");
      expect(connector.integrationTarget.length).toBeGreaterThan(20);
      expect(`${connector.integrationTarget} ${connector.notes}`).toMatch(
        /local|reversible|backup|restore|off-mode|guided/i,
      );
    }
  });

  it("looks up individual planned connectors", () => {
    expect(getPlannedConnector("cursor")?.name).toBe("Cursor");
    expect(getPlannedConnector("grok_cli")?.name).toBe("Grok / xAI CLI");
    expect(getPlannedConnector("missing")).toBeNull();
  });
});
