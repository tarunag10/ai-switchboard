import type {
  PlannedConnectorReadinessContract,
  PlannedConnectorReadinessStageId,
} from "./plannedConnectors";
import { plannedConnectorReadinessStageOrder } from "./plannedConnectors";

export type ConnectorPromotionVerdict =
  | "blocked"
  | "sidecar_ready"
  | "native_promoted";

export interface ConnectorPromotionEvaluation {
  verdict: ConnectorPromotionVerdict;
  reasons: string[];
  nextBlockedStage: PlannedConnectorReadinessStageId | null;
}

export function evaluateConnectorPromotionGate(
  contract: PlannedConnectorReadinessContract,
): ConnectorPromotionEvaluation {
  const blockedStages = contract.stages.filter((stage) => stage.state === "blocked");
  const nativePromotionReady =
    contract.nativeAutomationEnabled &&
    contract.nativeWriteEvidence.trim().length > 0 &&
    contract.nativeNextBlockedStage === null &&
    blockedStages.length === 0;
  if (nativePromotionReady) {
    return {
      verdict: "native_promoted",
      reasons: [contract.nativeWriteEvidence],
      nextBlockedStage: null,
    };
  }

  if (contract.automationEnabled && blockedStages.length === 0) {
    return {
      verdict: "sidecar_ready",
      reasons: [
        `${contract.connectorName} sidecar lifecycle is proven on fixture homes with dry-run, backup, apply, verify, rollback, and Off cleanup evidence.`,
      ],
      nextBlockedStage: null,
    };
  }

  const nextBlockedStage =
    (contract.nativeAutomationEnabled ? contract.nativeNextBlockedStage : null) ??
    contract.nextBlockedStage ??
    blockedStages[0]?.id ??
    plannedConnectorReadinessStageOrder[0];

  return {
    verdict: "blocked",
    reasons: [
      contract.nativeAutomationEnabled && !nativePromotionReady
        ? `${contract.connectorName} native promotion contract is contradictory or incomplete.`
        : blockedStages.length > 0
        ? `${contract.connectorName} is blocked at ${blockedStages.map((stage) => stage.label).join(", ")}.`
        : `${contract.connectorName} automation is not enabled yet.`,
      contract.nativeWriteEvidence,
    ],
    nextBlockedStage,
  };
}

export function canPromoteConnectorPastSidecar(
  contract: PlannedConnectorReadinessContract,
): boolean {
  const { verdict } = evaluateConnectorPromotionGate(contract);
  return verdict === "sidecar_ready" || verdict === "native_promoted";
}
