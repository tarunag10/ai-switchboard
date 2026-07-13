import { type Dispatch, type SetStateAction } from "react";
import { Copy, Info, Sparkle } from "@phosphor-icons/react";
import {
  aggregateClientConnectors,
  connectorCompatibilityReport,
  connectorCompatibilityRoutingEvidenceLabel,
  connectorControlState,
  connectorSupportsAutomaticSetup,
  formatPlannedConnectorConfigGateSummary,
  sortClientConnectors,
} from "../lib/dashboardHelpers";
import {
  formatPlannedConnectorConfigCreationPlansMarkdown,
  getPlannedConnector,
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorReadinessContract,
  getPlannedConnectorSetupChecklistScript,
  getPlannedConnectorSetupGuide,
} from "../lib/plannedConnectors";
import {
  connectorSetupDetails,
  formatBackendConnectorConfigPlan,
  getConnectorDetectionWarning,
  getConnectorUnavailableReason,
  getPlannedConnectorNextStep,
} from "../lib/settingsConnectorCopy";
import type { ClientConnectorStatus } from "../lib/types";

export interface PlannedConnectorReadinessSummary {
  headline: string;
  detail: string;
  detectedCount: number;
  manualOnlyCount: number;
  notDetectedCount: number;
  safeTodayCount: number;
  automationGateCount: number;
}

export interface SettingsConnectorPanelProps {
  connectors: ClientConnectorStatus[];
  plannedConnectorReadiness: PlannedConnectorReadinessSummary;
  plannedConnectorCopyNotice: string | null;
  connectorsBusy: boolean;
  connectorsError: string | null;
  openConnectorHelpId: string | null;
  setOpenConnectorHelpId: Dispatch<SetStateAction<string | null>>;
  toggleConnector: (
    connector: ClientConnectorStatus,
    enabled: boolean,
  ) => Promise<void>;
  copyPlannedConnectorCommand: (
    command: string,
    connectorName: string,
  ) => Promise<void>;
}

function renderConnectorLogo(clientId: string) {
  return <Sparkle className="client-logo__glyph" size={20} weight="duotone" />;
}

export function SettingsConnectorPanel({
  connectors,
  plannedConnectorReadiness,
  plannedConnectorCopyNotice,
  connectorsBusy,
  connectorsError,
  openConnectorHelpId,
  setOpenConnectorHelpId,
  toggleConnector,
  copyPlannedConnectorCommand,
}: SettingsConnectorPanelProps) {
  return (
    <article className="soft-card panel-card">
      <div className="panel-card__header">
        <div />
      </div>
      <div className="connector-readiness">
        <div>
          <span className="connector-readiness__eyebrow">
            Connector readiness
          </span>
          <strong>{plannedConnectorReadiness.headline}</strong>
          <p>{plannedConnectorReadiness.detail}</p>
        </div>
        <div className="connector-readiness__actions">
          <div
            className="connector-readiness__metrics"
            aria-label="Connector readiness summary"
          >
            <span>
              <strong>{plannedConnectorReadiness.detectedCount}</strong>
              detected
            </span>
            <span>
              <strong>{plannedConnectorReadiness.manualOnlyCount}</strong>
              approval
            </span>
            <span>
              <strong>{plannedConnectorReadiness.notDetectedCount}</strong>
              missing
            </span>
            <span>
              <strong>{plannedConnectorReadiness.safeTodayCount}</strong>
              safe now
            </span>
            <span>
              <strong>{plannedConnectorReadiness.automationGateCount}</strong>
              gates
            </span>
          </div>
          <button
            type="button"
            className="connector-readiness__copy"
            onClick={() =>
              void copyPlannedConnectorCommand(
                getPlannedConnectorSetupChecklistScript(),
                "Connector checklist",
              )
            }
          >
            <Copy size={13} weight="bold" />
            Copy checks
          </button>
          <button
            type="button"
            className="connector-readiness__copy"
            onClick={() =>
              void copyPlannedConnectorCommand(
                formatPlannedConnectorConfigCreationPlansMarkdown(),
                "Connector config plans",
              )
            }
          >
            <Copy size={13} weight="bold" />
            Copy config plans
          </button>
        </div>
      </div>
      <div className="connector-list">
        {sortClientConnectors(aggregateClientConnectors(connectors)).map(
          (connector) => {
            const connectorLabel =
              connector.clientId === "claude_code"
                ? "Claude Code connection"
                : connector.clientId === "codex"
                  ? "Codex connection"
                  : connector.name;
            const controlState = connectorControlState(connector);
            const unavailableReason = getConnectorUnavailableReason(connector);
            const detectionWarning = getConnectorDetectionWarning(connector);
            const toggleDisabled = connectorsBusy || controlState.disabled;
            const plannedConnector = getPlannedConnector(connector.clientId);
            const plannedSetupGuide = plannedConnector
              ? getPlannedConnectorSetupGuide(plannedConnector.id)
              : null;
            const plannedReadiness = plannedConnector
              ? getPlannedConnectorReadinessContract(plannedConnector)
              : null;
            const plannedReadinessBadges = plannedConnector
              ? getPlannedConnectorReadinessBadges(plannedConnector)
              : [];
            const connectorSetupPhase =
              connector.setupPhase ?? plannedConnector?.setupPhase ?? null;
            const connectorSetupHint =
              connector.setupHint ?? plannedConnector?.notes ?? null;
            const compatibilityReport = connectorCompatibilityReport(connector);
            const configGateSummary =
              formatPlannedConnectorConfigGateSummary(connector);
            return (
              <article className="connector-item" key={connector.clientId}>
                <div>
                  <h3>
                    <span className="client-logo" aria-hidden="true">
                      {renderConnectorLogo(connector.clientId)}
                    </span>
                    {connectorLabel}
                    {connector.supportStatus === "planned" ? (
                      <span className="connector-item__badge connector-item__badge--planned">
                        Gated
                      </span>
                    ) : null}
                    <button
                      className="connector-help"
                      onClick={() =>
                        setOpenConnectorHelpId((current) =>
                          current === connector.clientId
                            ? null
                            : connector.clientId,
                        )
                      }
                      type="button"
                      aria-controls={`connector-setup-details-${connector.clientId}`}
                      aria-label={`${openConnectorHelpId === connector.clientId ? "Hide" : "Show"} setup details for ${connector.name}`}
                      aria-expanded={openConnectorHelpId === connector.clientId}
                    >
                      <Info size={11} weight="bold" />
                    </button>
                  </h3>
                  {openConnectorHelpId === connector.clientId ? (
                    <p
                      className="connector-tooltip"
                      id={`connector-setup-details-${connector.clientId}`}
                    >
                      {connectorSetupHint ??
                        connectorSetupDetails[connector.clientId] ??
                        "Switchboard applies local connector configuration."}
                    </p>
                  ) : null}
                  {openConnectorHelpId === connector.clientId &&
                  (detectionWarning ?? unavailableReason) ? (
                    <p className="connector-item__reason">
                      {detectionWarning ?? unavailableReason}
                    </p>
                  ) : null}
                  <p className="connector-item__summary">
                    {connector.enabled
                      ? connector.verified
                        ? "Enabled and verified."
                        : "Enabled; verification still needs attention."
                      : connectorSupportsAutomaticSetup(connector)
                        ? "Automatic setup is available."
                        : "Detected or supported as manual setup."}
                  </p>
                  {openConnectorHelpId === connector.clientId &&
                  plannedConnector ? (
                    <div className="connector-plan">
                      <div className="connector-plan__meta">
                        <span>{connectorSetupPhase}</span>
                        <span>
                          {connector.category ?? plannedConnector.category}
                        </span>
                      </div>
                      <p className="connector-plan__target">
                        {plannedConnector.integrationTarget}
                      </p>
                      {plannedReadiness ? (
                        <div className="connector-plan__readiness">
                          <div>
                            <strong>Readiness contract</strong>
                            <span>
                              Next gate:{" "}
                              {plannedReadiness.stages.find(
                                (stage) =>
                                  stage.id ===
                                  plannedReadiness.nextBlockedStage,
                              )?.label ?? "Automation ready"}
                              </span>
                            </div>
                          <p className="connector-plan__native-boundary">
                            <strong>Native provider/editor writes:</strong>{" "}
                            {plannedReadiness.nativeAutomationEnabled
                              ? "Promoted for this documented connector surface. Credentials, account state, and model selection remain manual."
                              : `${plannedReadiness.nativeWriteEvidence} Native gate: ${plannedReadiness.stages.find((stage) => stage.id === plannedReadiness.nativeNextBlockedStage)?.label ?? "manual"}.`}
                          </p>
                          <div
                            className="connector-plan__stage-row"
                            aria-label={`${connector.name} readiness contract`}
                          >
                            {plannedReadiness.stages.map((stage) => (
                              <span
                                className={`connector-plan__stage connector-plan__stage--${stage.state}`}
                                key={stage.id}
                                title={stage.evidence}
                              >
                                {stage.label}
                              </span>
                            ))}
                          </div>
                        </div>
                      ) : null}
                      {plannedReadinessBadges.length ? (
                        <div
                          className="connector-plan__badges"
                          aria-label={`${connector.name} safety badges`}
                        >
                          {plannedReadinessBadges.map((badge) => (
                            <span
                              className={`connector-plan__badge connector-plan__badge--${badge.kind}`}
                              key={badge.kind}
                              title={badge.detail}
                            >
                              {badge.label}
                            </span>
                          ))}
                        </div>
                      ) : null}
                      {compatibilityReport ? (
                        <div className="connector-plan__compatibility">
                          <strong>{compatibilityReport.title}</strong>
                          {compatibilityReport.binaryPath ? (
                            <span>
                              {compatibilityReport.primaryPathLabel}{" "}
                              {compatibilityReport.binaryPath}
                            </span>
                          ) : null}
                          {compatibilityReport.version ? (
                            <span>Version {compatibilityReport.version}</span>
                          ) : null}
                          {compatibilityReport.configSurface ? (
                            <span>
                              Config {compatibilityReport.configSurface}
                            </span>
                          ) : null}
                          {compatibilityReport.routingBlocker ? (
                            <span>
                              {connectorCompatibilityRoutingEvidenceLabel(
                                compatibilityReport,
                              )}{" "}
                              {compatibilityReport.routingBlocker}
                            </span>
                          ) : null}
                          {compatibilityReport.configCreationGates.length ? (
                            <span>
                              Config gates{" "}
                              {compatibilityReport.configCreationGates
                                .map((gate) => gate.label)
                                .join(" -> ")}
                            </span>
                          ) : null}
                          <span>
                            Automation{" "}
                            {compatibilityReport.automationEnabled
                              ? "enabled"
                              : "approval required"}
                          </span>
                        </div>
                      ) : null}
                      {configGateSummary ? (
                        <div className="connector-plan__config-gates">
                          <strong>{configGateSummary.title}</strong>
                          <span>{configGateSummary.detail}</span>
                          <span>Next: {configGateSummary.nextGateLabel}</span>
                          <span>{configGateSummary.safetyNote}</span>
                        </div>
                      ) : null}
                      {connector.detectionSources?.length ||
                      connector.configLocations?.length ||
                      connector.detectionEvidence?.length ||
                      connector.automationGates?.length ||
                      connector.manualWorkflow?.length ||
                      connector.configCreationStepDetails?.length ||
                      connector.configCreationSteps?.length ||
                      connector.automationPath?.length ? (
                        <div className="connector-plan__backend">
                          <strong>Backend checks</strong>
                          {connector.detectionSources?.length ? (
                            <span>
                              Detects{" "}
                              {connector.detectionSources
                                .slice(0, 3)
                                .join(", ")}
                            </span>
                          ) : null}
                          {connector.configLocations?.length ? (
                            <span>
                              Watches{" "}
                              {connector.configLocations.slice(0, 2).join(", ")}
                            </span>
                          ) : null}
                          {connector.detectionEvidence?.length ? (
                            <span>
                              Evidence{" "}
                              {connector.detectionEvidence
                                .slice(0, 2)
                                .join(" · ")}
                            </span>
                          ) : null}
                          {connector.automationGates?.length ? (
                            <span>
                              Safety checks needed{" "}
                              {connector.automationGates
                                .slice(0, 2)
                                .join(" · ")}
                            </span>
                          ) : null}
                          {connector.manualWorkflow?.length ? (
                            <span>
                              Approval needed{" "}
                              {connector.manualWorkflow.slice(0, 2).join(" · ")}
                            </span>
                          ) : null}
                          {connector.configCreationSteps?.length ? (
                            <span>
                              Automatic setup off until safe backup, apply,
                              verification, rollback, and Off cleanup are
                              available.
                            </span>
                          ) : null}
                          {connector.automationPath?.length ? (
                            <span>
                              Automation path{" "}
                              {connector.automationPath
                                .slice(0, 7)
                                .map(
                                  (stage) => `${stage.label}: ${stage.status}`,
                                )
                                .join(" -> ")}
                            </span>
                          ) : null}
                        </div>
                      ) : null}
                      <div className="connector-plan__capabilities">
                        {plannedConnector.capabilityRows.map((capability) => (
                          <div
                            className="connector-plan__capability"
                            key={`${plannedConnector.id}-${capability.label}`}
                          >
                            <div>
                              <strong>{capability.label}</strong>
                              <span>{capability.detail}</span>
                            </div>
                            <span
                              className={`connector-plan__state connector-plan__state--${capability.state
                                .toLowerCase()
                                .replace(/\s+/g, "-")}`}
                            >
                              {capability.state}
                            </span>
                          </div>
                        ))}
                      </div>
                      <p className="connector-plan__next">
                        {getPlannedConnectorNextStep(
                          connector,
                          plannedConnector,
                        )}
                      </p>
                      {plannedSetupGuide ? (
                        <div className="connector-plan__guide">
                          <div>
                            <strong>{plannedSetupGuide.label}</strong>
                            <code>{plannedSetupGuide.command}</code>
                          </div>
                          <button
                            type="button"
                            className="connector-plan__copy"
                            onClick={() =>
                              void copyPlannedConnectorCommand(
                                plannedSetupGuide.command,
                                connector.name,
                              )
                            }
                            aria-label={`Copy ${connector.name} setup check command`}
                          >
                            <Copy size={13} weight="bold" />
                          </button>
                          <button
                            type="button"
                            className="connector-plan__copy"
                            onClick={() =>
                              void copyPlannedConnectorCommand(
                                formatBackendConnectorConfigPlan(
                                  connector,
                                  plannedConnector,
                                ),
                                `${connector.name} config plan`,
                              )
                            }
                            aria-label={`Copy ${connector.name} config creation plan`}
                          >
                            <Copy size={13} weight="duotone" />
                          </button>
                        </div>
                      ) : null}
                      {plannedSetupGuide ? (
                        <p className="connector-plan__note">
                          {plannedSetupGuide.notes}
                        </p>
                      ) : null}
                    </div>
                  ) : null}
                  {connector.enabled &&
                  !connector.verified &&
                  connector.installed ? (
                    <p className="connector-item__restart">
                      Restart {connector.name} to start routing through
                      Headroom.
                    </p>
                  ) : null}
                </div>
                <div className="connector-item__controls">
                  <button
                    className="connector-item__action connector-item__action--primary"
                    disabled={toggleDisabled}
                    onClick={() =>
                      void toggleConnector(connector, !connector.enabled)
                    }
                    title={
                      controlState.reason ?? unavailableReason ?? undefined
                    }
                    type="button"
                  >
                    {connector.enabled
                      ? "Disable"
                      : connectorSupportsAutomaticSetup(connector)
                        ? "Enable"
                        : "Manual setup"}
                  </button>
                  <button
                    aria-checked={connector.enabled}
                    aria-label={`${connector.enabled ? "Disable" : "Enable"} ${connector.name} connector`}
                    className={`connector-switch${connector.enabled ? " is-on" : ""}`}
                    disabled={toggleDisabled}
                    onClick={() =>
                      void toggleConnector(connector, !connector.enabled)
                    }
                    role="switch"
                    title={
                      controlState.reason ?? unavailableReason ?? undefined
                    }
                    type="button"
                  >
                    <span className="connector-switch__thumb" />
                  </button>
                </div>
              </article>
            );
          },
        )}
      </div>
      {connectorsError ? (
        <p className="install-progress__error">{connectorsError}</p>
      ) : null}
      {plannedConnectorCopyNotice ? (
        <p className="connector-copy-notice">{plannedConnectorCopyNotice}</p>
      ) : null}
    </article>
  );
}
