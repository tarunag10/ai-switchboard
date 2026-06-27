import { describe, expect, it } from "vitest";

import {
  formatPlannedConnectorConfigCreationPlansMarkdown,
  getPlannedConnector,
  getPlannedConnectorConfigCreationPlan,
  getPlannedConnectorConfigCreationPlans,
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorReadinessContract,
  getPlannedConnectorReadinessContracts,
  getPlannedConnectorSafetyDossier,
  getPlannedConnectorSafetyDossiers,
  getPlannedConnectorSetupChecklistScript,
  getPlannedConnectorSetupGuide,
  formatPlannedConnectorSafetyDossierMarkdown,
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

  it("documents config, provider, account, and rollback strategy per connector", () => {
    const dossiers = getPlannedConnectorSafetyDossiers();

    expect(dossiers.map((dossier) => dossier.connectorId)).toEqual(
      plannedConnectors.map((connector) => connector.id),
    );
    for (const connector of plannedConnectors) {
      const dossier = getPlannedConnectorSafetyDossier(connector.id);

      expect(dossier).toBeTruthy();
      expect(dossier?.configPathStrategy).toMatch(
        /detect|PATH|config|settings|profile|app/i,
      );
      expect(dossier?.providerSemantics).toMatch(
        /provider|base-url|routing|handoff|proxy/i,
      );
      expect(dossier?.accountCaveat).toMatch(
        /account|credential|credentials|profile|secrets?|model|AWS|SSO/i,
      );
      expect(dossier?.rollbackStrategy).toMatch(
        /restore|remove|rollback|backup|unchanged|preserving/i,
      );

      const markdown = formatPlannedConnectorSafetyDossierMarkdown(connector);
      expect(markdown).toContain(`## ${connector.name}`);
      expect(markdown).toContain("Provider/base-url semantics");
      expect(markdown).toContain("Rollback strategy");
    }
  });

  it("defines config-creation plans for every connector before enabling writes", () => {
    const plans = getPlannedConnectorConfigCreationPlans();

    expect(plans.map((plan) => plan.connectorId)).toEqual(
      plannedConnectors.map((connector) => connector.id),
    );
    for (const plan of plans) {
      expect(plan.automationEnabled).toBe(false);
      expect(plan.safetyNote).toMatch(/gated/i);
      expect(plan.steps.map((step) => step.id)).toEqual([
        "detect",
        "dryRunDiff",
        "backup",
        "apply",
        "verify",
        "rollback",
        "offCleanup",
      ]);
      expect(plan.steps.map((step) => `${step.label} ${step.detail}`).join(" ")).toMatch(
        /detect|dry-run|backup|provider|Doctor|rollback|Off mode/i,
      );
    }
  });

  it("carries OpenCode, Grok, and Cursor config-creation details explicitly", () => {
    const opencode = getPlannedConnectorConfigCreationPlan(
      getPlannedConnector("opencode")!,
    );
    const grok = getPlannedConnectorConfigCreationPlan(
      getPlannedConnector("grok_cli")!,
    );
    const cursor = getPlannedConnectorConfigCreationPlan(
      getPlannedConnector("cursor")!,
    );

    expect(opencode.steps.find((step) => step.id === "detect")?.detail).toMatch(
      /opencode/i,
    );
    expect(opencode.steps.find((step) => step.id === "backup")?.detail).toMatch(
      /backup|restore point/i,
    );
    expect(grok.steps.find((step) => step.id === "verify")?.detail).toMatch(
      /model|account/i,
    );
    expect(cursor.steps.find((step) => step.id === "rollback")?.detail).toMatch(
      /profile settings backup/i,
    );
  });

  it("formats copyable config-creation plans for handoff", () => {
    const markdown = formatPlannedConnectorConfigCreationPlansMarkdown([
      getPlannedConnector("opencode")!,
      getPlannedConnector("grok_cli")!,
      getPlannedConnector("cursor")!,
    ]);

    expect(markdown).toContain(
      "# Mac AI Switchboard Connector Config Creation Plans",
    );
    expect(markdown).toContain("Automation stays disabled");
    expect(markdown).toContain("## OpenCode");
    expect(markdown).toContain("Detect config surface: Detect PATH: opencode");
    expect(markdown).toContain("## Grok / xAI CLI");
    expect(markdown).toContain("Doctor guardrails");
    expect(markdown).toContain("## Cursor");
    expect(markdown).toContain("Rollback safely");
    expect(markdown).toContain("Off mode removes only Switchboard-managed");
  });
});
