import { describe, expect, it } from "vitest";

import {
  getPlannedConnector,
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorReadinessContract,
  getPlannedConnectorReadinessContracts,
  getPlannedConnectorSetupChecklistScript,
  getPlannedConnectorSetupGuide,
  plannedConnectorReadinessStageOrder,
  plannedConnectors,
  summarizePlannedConnectorSupport,
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
      "qwen_code",
      "amazon_q",
      "windsurf",
      "zed_ai",
    ]);
  });

  it("keeps every planned connector explicit about local reversible setup", () => {
    for (const connector of plannedConnectors) {
      expect(connector.statusLabel).toBe("Planned");
      expect(["Detect", "Guide", "Adapt"]).toContain(connector.setupPhase);
      expect(connector.integrationTarget.length).toBeGreaterThan(20);
      expect(connector.capabilityBadges.length).toBeGreaterThanOrEqual(3);
      expect(
        connector.capabilityBadges.every((badge) => badge.length > 5),
      ).toBe(true);
      expect(`${connector.integrationTarget} ${connector.notes}`).toMatch(
        /local|reversible|backup|restore|off-mode|guided/i,
      );
    }
  });

  it("surfaces concrete capability badges roadmap decisions", () => {
    const badges = plannedConnectors.flatMap(
      (connector) => connector.capabilityBadges,
    );

    expect(badges).toContain("RTK-safe today");
    expect(badges).toContain("Backup/restore pending");
    expect(badges).toContain("Repo packs planned");
    expect(badges).toContain("Provider routing pending");
  });

  it("summarizes safe today and gated planned capabilities", () => {
    const summary = summarizePlannedConnectorSupport();

    expect(summary.connectorCount).toBe(plannedConnectors.length);
    expect(summary.safeTodayCount).toBeGreaterThanOrEqual(
      plannedConnectors.length,
    );
    expect(summary.manualTodayCount).toBeGreaterThan(0);
    expect(summary.plannedCount).toBeGreaterThanOrEqual(
      plannedConnectors.length,
    );
    expect(summary.automationGateCount).toBe(
      plannedConnectors.reduce(
        (total, connector) => total + connector.automationGates.length,
        0,
      ),
    );
    expect(summary.safeTodayLabels.join(" ")).toContain("Gemini CLI");
    expect(summary.plannedLabels.join(" ")).toContain("Provider");
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
        connector.capabilityRows.some(
          (capability) => capability.state === "Planned",
        ),
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
      plannedConnectors.filter(
        (connector) => connector.setupPhase === "Detect",
      ),
    ).toHaveLength(2);
    expect(
      plannedConnectors.filter((connector) => connector.setupPhase === "Guide"),
    ).toHaveLength(6);
    expect(
      plannedConnectors.filter((connector) => connector.setupPhase === "Adapt"),
    ).toHaveLength(3);
  });

  it("looks up connector metadata by id", () => {
    expect(getPlannedConnector("aider")?.name).toBe("Aider");
    expect(getPlannedConnector("qwen_code")?.name).toBe("Qwen Code");
    expect(getPlannedConnector("amazon_q")?.name).toBe(
      "Amazon Q Developer CLI",
    );
    expect(getPlannedConnector("windsurf")?.name).toBe("Windsurf");
    expect(getPlannedConnector("zed_ai")?.name).toBe("Zed AI");
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
      expect(script).toContain(
        getPlannedConnectorSetupGuide(connector.id)?.command,
      );
    }
  });

  it("defines a staged readiness contract before connector automation", () => {
    const contracts = getPlannedConnectorReadinessContracts();

    expect(contracts).toHaveLength(plannedConnectors.length);
    for (const contract of contracts) {
      expect(contract.stages.map((stage) => stage.id)).toEqual(
        plannedConnectorReadinessStageOrder,
      );
      expect(contract.automationEnabled).toBe(false);
      expect(contract.nextBlockedStage).toBe("backupImplemented");
      expect(
        contract.stages.filter((stage) => stage.state === "ready").map(
          (stage) => stage.id,
        ),
      ).toEqual(["detected", "manualGuide"]);
      expect(
        contract.stages.filter((stage) => stage.state === "blocked").map(
          (stage) => stage.id,
        ),
      ).toEqual([
        "backupImplemented",
        "applyImplemented",
        "verifyImplemented",
        "rollbackImplemented",
        "offCleanupImplemented",
      ]);
    }
  });

  it("keeps readiness evidence tied to the connector metadata", () => {
    const qwen = plannedConnectors.find(
      (connector) => connector.id === "qwen_code",
    );
    expect(qwen).toBeTruthy();

    const contract = getPlannedConnectorReadinessContract(qwen!);

    expect(contract.connectorName).toBe("Qwen Code");
    expect(contract.setupPhase).toBe("Guide");
    expect(
      contract.stages.find((stage) => stage.id === "manualGuide")?.evidence,
    ).toMatch(/provider routing manual/i);
    expect(
      contract.stages.find((stage) => stage.id === "offCleanupImplemented")
        ?.evidence,
    ).toMatch(/Off mode cleanup/i);
  });

  it("derives roadmap readiness badges without enabling planned automation", () => {
    for (const connector of plannedConnectors) {
      const badges = getPlannedConnectorReadinessBadges(connector);
      const badgeLabels = badges.map((badge) => badge.label);

      expect(badgeLabels).toContain("Automation gated");
      expect(badgeLabels).not.toContain("Verified automation");
      expect(badges.every((badge) => badge.detail.length > 30)).toBe(true);
    }

    expect(
      getPlannedConnectorReadinessBadges(getPlannedConnector("cursor")!).map(
        (badge) => badge.label,
      ),
    ).toContain("Manual only");
    expect(
      getPlannedConnectorReadinessBadges(getPlannedConnector("amazon_q")!).map(
        (badge) => badge.label,
      ),
    ).toContain("Unsupported account/model");
    expect(
      getPlannedConnectorReadinessBadges(getPlannedConnector("grok_cli")!).map(
        (badge) => badge.label,
      ),
    ).toContain("Unsupported account/model");
  });
});
