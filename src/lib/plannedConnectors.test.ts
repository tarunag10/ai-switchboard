import { describe, expect, it } from "vitest";

import {
  getPlannedConnector,
  getPlannedConnectorSetupChecklistScript,
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
      expect(connector.capabilityBadges.length).toBeGreaterThanOrEqual(3);
      expect(connector.capabilityBadges.every((badge) => badge.length > 5)).toBe(
        true,
      );
      expect(`${connector.integrationTarget} ${connector.notes}`).toMatch(
        /local|reversible|backup|restore|off-mode|guided/i,
      );
    }
  });

  it("surfaces concrete capability badges roadmap decisions", () => {
    const badges = plannedConnectors.flatMap((connector) => connector.capabilityBadges);

    expect(badges).toContain("RTK-safe today");
    expect(badges).toContain("Backup/restore pending");
    expect(badges).toContain("Repo packs planned");
    expect(badges).toContain("Provider routing pending");
  });

  it("defines safe automation contracts for every future connector", () => {
    for (const connector of plannedConnectors) {
      expect(connector.configSurfaces.length).toBeGreaterThanOrEqual(3);
      expect(connector.automationGates.length).toBeGreaterThanOrEqual(3);
      expect(connector.manualWorkflow.length).toBeGreaterThanOrEqual(3);
      expect(connector.configSurfaces.join(" ")).toMatch(
        /binary|config|settings|environment|provider|profile|MCP|app/i,
      );
      expect(connector.automationGates.join(" ")).toMatch(
        /detect|backup|restore|Off mode|Doctor|guardrails|secrets|MCP|provider/i,
      );
      expect(connector.manualWorkflow.join(" ")).toMatch(
        /confirm|review|copy|RTK|manual|installed|settings|packs/i,
      );
    }
  });

  it("shows a concrete capability matrix for each future agent", () => {
    for (const connector of plannedConnectors) {
      expect(connector.capabilityRows).toHaveLength(3);
      expect(
        connector.capabilityRows.some(
          (capability) => capability.state === "Available now",
        ),
      ).toBe(true);
      expect(
        connector.capabilityRows.some((capability) => capability.state === "Planned"),
      ).toBe(true);

      for (const capability of connector.capabilityRows) {
        expect(capability.label.length).toBeGreaterThan(4);
        expect(capability.detail.length).toBeGreaterThan(30);
        expect(["Available now", "Manual today", "Planned"]).toContain(
          capability.state,
        );
      }
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

  it("looks up connector metadata by id", () => {
    expect(getPlannedConnector("aider")?.name).toBe("Aider");
    expect(getPlannedConnector("missing")).toBeNull();
  });

  it("provides manual setup checks without enabling automatic repair", () => {
    for (const connector of plannedConnectors) {
      const guide = getPlannedConnectorSetupGuide(connector.id);

      expect(guide?.label.length).toBeGreaterThan(5);
      expect(guide?.command.length).toBeGreaterThan(8);
      expect(guide?.notes).toMatch(/manual|confirm|review|before|after|only/i);
    }
  });

  it("builds a read-only setup checklist for every planned connector", () => {
    const script = getPlannedConnectorSetupChecklistScript();

    expect(script).toContain("Read-only");
    expect(script).not.toMatch(/export|>|tee|sed -i|defaults write|launchctl/i);
    for (const connector of plannedConnectors) {
      expect(script).toContain(`== ${connector.name} ==`);
      expect(script).toContain(getPlannedConnectorSetupGuide(connector.id)?.command);
    }
  });
});
