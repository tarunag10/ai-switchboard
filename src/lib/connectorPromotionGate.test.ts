import { describe, expect, it } from "vitest";

import { evaluateConnectorPromotionGate } from "./connectorPromotionGate";
import type { PlannedConnectorReadinessContract } from "./plannedConnectors";

function contract(
  overrides: Partial<PlannedConnectorReadinessContract> = {},
): PlannedConnectorReadinessContract {
  return {
    connectorId: "cursor",
    connectorName: "Cursor",
    setupPhase: "Guide",
    automationEnabled: false,
    nextBlockedStage: "backupImplemented",
    stages: [
      {
        id: "detected",
        label: "Detected",
        state: "ready",
        evidence: "detection ok",
      },
      {
        id: "manualGuide",
        label: "Manual Guide",
        state: "ready",
        evidence: "guide ok",
      },
      {
        id: "backupImplemented",
        label: "Backup",
        state: "blocked",
        evidence: "missing",
      },
      {
        id: "applyImplemented",
        label: "Apply",
        state: "blocked",
        evidence: "missing",
      },
      {
        id: "verifyImplemented",
        label: "Verify",
        state: "blocked",
        evidence: "missing",
      },
      {
        id: "rollbackImplemented",
        label: "Rollback",
        state: "blocked",
        evidence: "missing",
      },
      {
        id: "offCleanupImplemented",
        label: "Off cleanup",
        state: "blocked",
        evidence: "missing",
      },
    ],
    nativeAutomationEnabled: false,
    nativeNextBlockedStage: "backupImplemented",
    nativeWriteEvidence: "Cursor native writes remain gated.",
    ...overrides,
  };
}

describe("evaluateConnectorPromotionGate", () => {
  it("marks managed sidecars ready when every lifecycle stage is ready", () => {
    const readyStages = contract({
      connectorId: "goose",
      connectorName: "Goose",
      automationEnabled: true,
      nextBlockedStage: null,
      stages: contract().stages.map((stage) => ({ ...stage, state: "ready" as const })),
    });
    expect(evaluateConnectorPromotionGate(readyStages).verdict).toBe("sidecar_ready");
  });

  it("keeps gated connectors blocked with the next lifecycle stage", () => {
    const evaluation = evaluateConnectorPromotionGate(contract());
    expect(evaluation.verdict).toBe("blocked");
    expect(evaluation.nextBlockedStage).toBe("backupImplemented");
  });

  it("reports native promotion separately from sidecar readiness", () => {
    const evaluation = evaluateConnectorPromotionGate(
      contract({
        connectorId: "goose",
        connectorName: "Goose",
        automationEnabled: true,
        nativeAutomationEnabled: true,
        nativeNextBlockedStage: null,
        stages: contract().stages.map((stage) => ({ ...stage, state: "ready" as const })),
      }),
    );
    expect(evaluation.verdict).toBe("native_promoted");
  });
});
