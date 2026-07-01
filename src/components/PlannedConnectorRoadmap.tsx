import { Copy } from "@phosphor-icons/react";
import type { PlannedConnector } from "../lib/plannedConnectors";
import {
  getPlannedConnectorReadinessContract,
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorConfigCreationPlan,
} from "../lib/plannedConnectors";

function connectorCategoryLabel(category: PlannedConnector["category"]) {
  switch (category) {
    case "cli":
      return "CLI";
    case "editor":
      return "Editor";
    case "agent":
      return "Agent";
  }
}

export function PlannedConnectorRoadmap({
  connectors,
  onCopyConfigPlan,
}: {
  connectors: PlannedConnector[];
  onCopyConfigPlan: (connector: PlannedConnector) => void;
}) {
  return (
    <div className="planned-connectors" aria-label="Planned connector roadmap">
      <div className="planned-connectors__intro">
        <span>Expansion path</span>
        <strong>Detect first, adapt only when reversible.</strong>
      </div>
      <div
        className="planned-connectors__steps"
        aria-label="Connector setup phases"
      >
        <span>Read-only detection</span>
        <span>Guided setup</span>
        <span>Doctor-backed cleanup</span>
      </div>
      <ul className="planned-connectors__list">
        {connectors.map((connector) => {
          const readiness = getPlannedConnectorReadinessContract(connector);
          const readinessBadges = getPlannedConnectorReadinessBadges(connector);
          const configPlan = getPlannedConnectorConfigCreationPlan(connector);
          return (
          <li className="planned-connectors__item" key={connector.id}>
            <div className="planned-connectors__item-head">
              <strong>{connector.name}</strong>
              <span>{connectorCategoryLabel(connector.category)}</span>
            </div>
            <p>{connector.integrationTarget}</p>
            <div className="planned-connectors__capabilities">
              {connector.capabilityBadges.map((badge) => (
                <span key={badge}>{badge}</span>
              ))}
            </div>
            <div
              className="planned-connectors__badges"
              aria-label={`${connector.name} safety badges`}
            >
              {readinessBadges.map((badge) => (
                <span
                  className={`planned-connectors__badge planned-connectors__badge--${badge.kind}`}
                  key={badge.kind}
                  title={badge.detail}
                >
                  {badge.label}
                </span>
              ))}
            </div>
            <div
              className="planned-connectors__modes"
              aria-label={`${connector.name} supported modes`}
            >
              {connector.supportedModes.map((mode) => (
                <span key={mode}>{mode}</span>
              ))}
            </div>
            <div className="planned-connectors__readiness">
              <div>
                <span>Config surface</span>
                <strong>{connector.configSurfaces[0]}</strong>
              </div>
              <div>
                <span>Next gate</span>
                <strong>
                  {readiness.stages.find(
                    (stage) => stage.id === readiness.nextBlockedStage,
                  )?.label ?? "Automation ready"}
                </strong>
              </div>
            </div>
            <div
              className="planned-connectors__config-plan"
              aria-label={`${connector.name} config creation plan`}
              title={configPlan.safetyNote}
            >
              <div className="planned-connectors__config-plan-head">
                <span>Config creation</span>
                <button
                  type="button"
                  className="planned-connectors__copy"
                  onClick={() => onCopyConfigPlan(connector)}
                  aria-label={`Copy ${connector.name} config creation plan`}
                >
                  <Copy size={13} weight="bold" />
                </button>
              </div>
              <div>
                {configPlan.steps.map((step) => (
                  <strong key={step.id} title={step.detail}>
                    {step.label}
                  </strong>
                ))}
              </div>
            </div>
            <div
              className="planned-connectors__stage-row"
              aria-label={`${connector.name} readiness stages`}
            >
              {readiness.stages.slice(0, 4).map((stage) => (
                <span
                  className={`planned-connectors__stage planned-connectors__stage--${stage.state}`}
                  key={stage.id}
                  title={stage.evidence}
                >
                  {stage.label}
                </span>
              ))}
            </div>
            <p className="planned-connectors__manual">
              Today: {connector.safeToday}
            </p>
            <p className="planned-connectors__manual">
              Next: {connector.firstAutomation}
            </p>
            <div className="planned-connectors__meta">
              <span>{connector.setupPhase}</span>
              <span>{connector.statusLabel}</span>
            </div>
          </li>
          );
        })}
      </ul>
    </div>
  );
}
