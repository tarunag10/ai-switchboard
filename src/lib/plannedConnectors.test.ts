import { describe, expect, it } from "vitest";

import {
  getPlannedConnector,
  getPlannedConnectorSetupGuide,
  plannedConnectors,
} from "./plannedConnectors";

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
      expect(["Detect", "Guide", "Adapt"]).toContain(connector.setupPhase);
      expect(connector.integrationTarget.length).toBeGreaterThan(20);
      expect(`${connector.integrationTarget} ${connector.notes}`).toMatch(
        /local|reversible|backup|restore|off-mode|guided/i,
      );
    }
  });

  it("stages connector rollout before automatic config edits", () => {
    expect(
      plannedConnectors.filter((connector) => connector.setupPhase === "Detect"),
    ).toHaveLength(1);
    expect(
      plannedConnectors.filter((connector) => connector.setupPhase === "Guide"),
    ).toHaveLength(3);
    expect(
      plannedConnectors.filter((connector) => connector.setupPhase === "Adapt"),
    ).toHaveLength(3);
  });

  it("looks up individual planned connectors", () => {
    expect(getPlannedConnector("cursor")?.name).toBe("Cursor");
    expect(getPlannedConnector("grok_cli")?.name).toBe("Grok / xAI CLI");
    expect(getPlannedConnector("missing")).toBeNull();
  });

  it("provides copyable manual setup guides without routing mutations", () => {
    for (const connector of plannedConnectors) {
      const guide = getPlannedConnectorSetupGuide(connector.id);

      expect(guide?.label.length).toBeGreaterThan(8);
      expect(guide?.command).toMatch(/command -v|open /);
      expect(guide?.notes).toMatch(/confirm|review|manual|backup|Doctor|RTK/i);
      expect(`${guide?.command} ${guide?.notes}`).not.toMatch(
        /ANTHROPIC_BASE_URL|OPENAI_BASE_URL|HEADROOM_PROXY_URL/
      );
    }

    expect(getPlannedConnectorSetupGuide("missing")).toBeNull();
  });
});
