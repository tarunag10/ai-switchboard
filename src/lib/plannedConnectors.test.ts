import { describe, expect, it } from "vitest";

import {
  connectorManifests,
  connectorSupportMatrixRows,
  formatPlannedConnectorConfigCreationPlansMarkdown,
  getConnectorManifest,
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
  managedConnectorDossiers,
  pendingPlannedConnectors,
  plannedConnectorReadinessStageOrder,
  plannedConnectors,
  summarizePlannedConnectorSupport,
} from "./plannedConnectors";

describe("planned connectors", () => {
  it("tracks popular coding CLIs and editor agents beyond Claude Code and Codex", () => {
    expect(plannedConnectors.map((connector) => connector.id)).toEqual([
      "cursor",
      "grok_cli",
      "aider",
      "continue",
      "goose",
      "qwen_code",
      "amazon_q",
    ]);
  });

  it("tracks Gemini CLI, OpenCode, Windsurf, and Zed AI as managed connector dossiers", () => {
    expect(managedConnectorDossiers.map((connector) => connector.id)).toEqual([
      "gemini_cli",
      "opencode",
      "windsurf",
      "zed_ai",
    ]);
    for (const connector of managedConnectorDossiers) {
      expect(connector.statusLabel).toBe("Managed");
      expect(connector.setupPhase).toBe("Managed");
      expect(connector.supportedModes).toEqual(["Full", "Headroom", "Off"]);
      expect(connector.capabilityRows.every((row) => row.state === "Available now")).toBe(
        true,
      );
      expect(connector.notes).toMatch(/Doctor|rollback|Off cleanup/i);
    }
    const windsurf = managedConnectorDossiers.find(
      (connector) => connector.id === "windsurf",
    );
    expect(windsurf?.integrationTarget).toContain("editor settings routing");
    expect(windsurf?.notes).toContain("editor settings routing");
    expect(windsurf?.safeToday).toContain("editor settings routing");
    expect(windsurf?.capabilityRows[0]?.detail).toContain(
      "editor settings routing",
    );
    const zed = managedConnectorDossiers.find(
      (connector) => connector.id === "zed_ai",
    );
    expect(zed?.integrationTarget).toContain("assistant settings routing");
    expect(zed?.notes).toContain("assistant settings routing");
    expect(zed?.automationGates.join(" ")).toContain(
      "assistant settings routing",
    );
  });

  it("derives the shared support matrix from connector manifests", () => {
    const rows = connectorSupportMatrixRows();

    expect(rows.map((row) => row.id)).toEqual(
      connectorManifests.map((manifest) => manifest.id),
    );
    expect(getConnectorManifest("codex")?.support_status).toBe("managed");
    expect(getConnectorManifest("cursor")?.support_status).toBe("planned");
    expect(getConnectorManifest("missing")).toBeNull();
    expect(rows.find((row) => row.id === "gemini_cli")).toMatchObject({
      name: "Gemini CLI",
      category: "cli",
      supportStatus: "managed",
    });
    expect(
      rows.find((row) => row.id === "opencode")?.detectionSources,
    ).toContain("PATH: opencode");
  });

  it("keeps rich frontend connector identity and status aligned with manifests", () => {
    for (const connector of [
      ...managedConnectorDossiers,
      ...plannedConnectors,
    ]) {
      const manifest = getConnectorManifest(connector.id);

      expect(manifest).toBeTruthy();
      expect(connector.name).toBe(manifest?.name);
      expect(connector.category).toBe(manifest?.category);
      expect(connector.supportStatus).toBe(manifest?.support_status);
    }
  });

  it("keeps every planned connector explicit about local reversible setup", () => {
    for (const connector of plannedConnectors) {
      expect(connector.statusLabel).toBe("Gated");
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
    expect(badges).toContain("Backup/restore gated");
    expect(badges).toContain("Repo packs gated");
    expect(badges).toContain("Provider routing gated");
  });

  it("summarizes safe today and gated planned capabilities", () => {
    const summary = summarizePlannedConnectorSupport();

    expect(pendingPlannedConnectors).toHaveLength(0);
    expect(summary).toMatchObject({
      connectorCount: 0,
      safeTodayCount: 0,
      manualTodayCount: 0,
      plannedCount: 0,
      automationGateCount: 0,
      safeTodayLabels: [],
      plannedLabels: [],
    });

    const fullMetadataSummary = summarizePlannedConnectorSupport(plannedConnectors);
    expect(fullMetadataSummary.connectorCount).toBe(plannedConnectors.length);
    expect(fullMetadataSummary.safeTodayCount).toBeGreaterThan(0);
    expect(fullMetadataSummary.manualTodayCount).toBeGreaterThan(0);
    expect(fullMetadataSummary.plannedCount).toBeGreaterThan(0);
    expect(fullMetadataSummary.automationGateCount).toBe(
      plannedConnectors.reduce(
        (total, connector) => total + connector.automationGates.length,
        0,
      ),
    );
    expect(fullMetadataSummary.safeTodayLabels.join(" ")).toContain("Cursor");
    expect(fullMetadataSummary.plannedLabels.join(" ")).toContain("Provider");
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
          (capability) =>
            capability.state === "Available now" ||
            capability.state === "Manual today",
        ),
      ).toBe(true);

      for (const capability of connector.capabilityRows) {
        expect(capability.label.length).toBeGreaterThan(4);
        expect(capability.detail.length).toBeGreaterThan(30);
        expect(["Available now", "Manual today", "Gated"]).toContain(
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
    ).toHaveLength(3);
    expect(
      plannedConnectors.filter((connector) => connector.setupPhase === "Adapt"),
    ).toHaveLength(2);
  });

  it("looks up connector metadata by id", () => {
    expect(getPlannedConnector("aider")?.name).toBe("Aider");
    expect(getPlannedConnector("qwen_code")?.name).toBe("Qwen Code");
    expect(getPlannedConnector("amazon_q")?.name).toBe(
      "Amazon Q Developer CLI",
    );
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
    const contracts = getPlannedConnectorReadinessContracts(plannedConnectors);

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

  it("marks promoted managed editor connectors automation-ready", () => {
    const contracts = getPlannedConnectorReadinessContracts(
      managedConnectorDossiers.filter((connector) =>
        ["windsurf", "zed_ai"].includes(connector.id),
      ),
    );

    expect(contracts.map((contract) => contract.connectorId)).toEqual([
      "windsurf",
      "zed_ai",
    ]);
    for (const contract of contracts) {
      expect(contract.setupPhase).toBe("Managed");
      expect(contract.automationEnabled).toBe(true);
      expect(contract.nextBlockedStage).toBeNull();
      expect(contract.stages.every((stage) => stage.state === "ready")).toBe(
        true,
      );
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
    const dossiers = getPlannedConnectorSafetyDossiers(plannedConnectors);

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
    const plans = getPlannedConnectorConfigCreationPlans(plannedConnectors);

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
      expect(plan.steps.find((step) => step.id === "dryRunDiff")?.detail).toMatch(
        /target path|before\/after|managed marker|rollback preview|confirmation phrase/i,
      );
      expect(plan.steps.find((step) => step.id === "backup")?.requiredEvidence.join(" ")).toMatch(
        /Fixture-home restore test/i,
      );
      expect(plan.steps.find((step) => step.id === "rollback")?.requiredEvidence.join(" ")).toMatch(
        /Fixture-home rollback test/i,
      );
      expect(plan.steps.find((step) => step.id === "offCleanup")?.requiredEvidence.join(" ")).toMatch(
        /Fixture-home Off-mode cleanup/i,
      );
      for (const step of plan.steps) {
        expect(step.requiredEvidence.length).toBeGreaterThanOrEqual(2);
        expect(step.requiredEvidence.join(" ")).toMatch(
          /read-only|dry-run|backup|consent|Doctor|rollback|Off-mode|fixture|diff|manual|RTK-only/i,
        );
      }
    }
  });

  it("reports promoted managed editor config plans as enabled", () => {
    const plans = getPlannedConnectorConfigCreationPlans(
      managedConnectorDossiers.filter((connector) =>
        ["windsurf", "zed_ai"].includes(connector.id),
      ),
    );

    expect(plans.map((plan) => plan.connectorId)).toEqual([
      "windsurf",
      "zed_ai",
    ]);
    for (const plan of plans) {
      expect(plan.automationEnabled).toBe(true);
      expect(plan.safetyNote).toMatch(/managed routing is enabled/i);
      expect(plan.steps.map((step) => step.id)).toEqual([
        "detect",
        "dryRunDiff",
        "backup",
        "apply",
        "verify",
        "rollback",
        "offCleanup",
      ]);
    }
  });

  it("carries Grok, Cursor, and Aider config-creation details explicitly", () => {
    const grok = getPlannedConnectorConfigCreationPlan(
      getPlannedConnector("grok_cli")!,
    );
    const cursor = getPlannedConnectorConfigCreationPlan(
      getPlannedConnector("cursor")!,
    );
    const aider = getPlannedConnectorConfigCreationPlan(
      getPlannedConnector("aider")!,
    );

    expect(aider.steps.find((step) => step.id === "detect")?.detail).toMatch(
      /aider/i,
    );
    expect(aider.steps.find((step) => step.id === "backup")?.detail).toMatch(
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
      getPlannedConnector("aider")!,
      getPlannedConnector("grok_cli")!,
      getPlannedConnector("cursor")!,
    ]);

    expect(markdown).toContain(
      "# Mac AI Switchboard Connector Config Creation Plans",
    );
    expect(markdown).toContain("Automation stays disabled");
    expect(markdown).toContain("## Aider");
    expect(markdown).toContain("Detect config surface: Detect PATH: aider");
    expect(markdown).toContain("Required evidence:");
    expect(markdown).toContain("No files, profiles, credentials, or account state changed");
    expect(markdown).toContain("managed marker boundary");
    expect(markdown).toContain("confirmation phrase");
    expect(markdown).toContain("## Grok / xAI CLI");
    expect(markdown).toContain("Doctor guardrails");
    expect(markdown).toContain("## Cursor");
    expect(markdown).toContain("Rollback safely");
    expect(markdown).toContain("Off mode removes only Switchboard-managed");
  });

  it("formats a single connector config-creation plan for card-level copy", () => {
    const markdown = formatPlannedConnectorConfigCreationPlansMarkdown([
      getPlannedConnector("aider")!,
    ]);

    expect(markdown).toContain(
      "# Mac AI Switchboard Connector Config Creation Plan",
    );
    expect(markdown).not.toContain("## Grok / xAI CLI");
    expect(markdown).toContain("## Aider");
    expect(markdown).toContain("Automation enabled: no");
    expect(markdown).toContain("Show dry-run diff");
  });
});
