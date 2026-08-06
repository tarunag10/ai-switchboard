import type { Dispatch, MouseEvent, SetStateAction } from "react";
import { Info } from "@phosphor-icons/react";

import { getPlannedConnector } from "../lib/plannedConnectors";
import {
  connectorControlState,
  sortClientConnectors,
} from "../lib/dashboardHelpers";
import {
  connectorSetupDetails,
  getConnectorDetectionWarning,
  getConnectorUnavailableReason,
} from "../lib/settingsConnectorCopy";
import type { ClientConnectorStatus } from "../lib/types";
import { ConnectorLogo } from "./ConnectorLogo";
import { LauncherShell } from "./LauncherShell";

const connectorSupportWarnings: Record<string, string> = {};

interface LauncherClientSetupStepProps {
  appSemver: string;
  connectors: ClientConnectorStatus[];
  connectorsBusy: boolean;
  connectorsError: string | null;
  openConnectorHelpId: string | null;
  openConnectorWarningId: string | null;
  setOpenConnectorHelpId: Dispatch<SetStateAction<string | null>>;
  setOpenConnectorWarningId: Dispatch<SetStateAction<string | null>>;
  setLauncherStage: (stage: "install" | "client_setup" | "proxy_verify" | "post_install") => void;
  toggleConnector: (
    connector: ClientConnectorStatus,
    enabled: boolean,
  ) => Promise<void>;
  onMouseDown: (event: MouseEvent<HTMLElement>) => void;
  onContinue: () => void | Promise<void>;
}

export function LauncherClientSetupStep({
  appSemver,
  connectors,
  connectorsBusy,
  connectorsError,
  openConnectorHelpId,
  openConnectorWarningId,
  setOpenConnectorHelpId,
  setOpenConnectorWarningId,
  setLauncherStage,
  toggleConnector,
  onMouseDown,
  onContinue,
}: LauncherClientSetupStepProps) {
  const sortedLauncherConnectors = sortClientConnectors(connectors);
  const availableConnectors = sortedLauncherConnectors.filter(
    (connector) => !connectorControlState(connector).disabled,
  );
  const unavailableConnectors = sortedLauncherConnectors.filter((connector) =>
    connectorControlState(connector).disabled,
  );
  const enabledConnectorCount = connectors.filter(
    (connector) => connector.enabled,
  ).length;
  const requireSelection = availableConnectors.length > 0;

  return (
    <LauncherShell
      shellClassName="intro-shell intro-shell--post-install intro-shell--client-setup"
      spinnerClassName="intro-shell__spinner intro-shell__spinner--post-install"
      copyClassName="intro-shell__copy intro-shell__copy--post-install"
      onMouseDown={onMouseDown}
      version={appSemver}
    >
      <div className="post-install__lead">
        <h1>Connect your coding tools</h1>
        <p>Toggle each tool to automatically route it through Headroom.</p>
        <div className="connector-list">
          {availableConnectors.map((connector) => {
            const unavailableReason = getConnectorUnavailableReason(connector);
            const detectionWarning = getConnectorDetectionWarning(connector);
            const supportWarning =
              connectorSupportWarnings[connector.clientId] ?? null;
            const needsRestart = connector.enabled && !connector.verified;
            const plannedConnector = getPlannedConnector(connector.clientId);
            return (
              <article className="connector-item" key={connector.clientId}>
                <div>
                  <h3>
                    <span className="client-logo" aria-hidden="true">
                      <ConnectorLogo clientId={connector.clientId} />
                    </span>
                    {connector.name}
                    {supportWarning ? (
                      <button
                        className="connector-warning-help"
                        onClick={() =>
                          setOpenConnectorWarningId((current) =>
                            current === connector.clientId
                              ? null
                              : connector.clientId,
                          )
                        }
                        type="button"
                        aria-label={`Show warning for ${connector.name}`}
                        aria-expanded={
                          openConnectorWarningId === connector.clientId
                        }
                      >
                        !
                      </button>
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
                      aria-label={`Show setup details for ${connector.name}`}
                      aria-expanded={openConnectorHelpId === connector.clientId}
                    >
                      <Info size={11} weight="bold" />
                    </button>
                  </h3>
                  {openConnectorHelpId === connector.clientId ? (
                    <p className="connector-tooltip">
                      {plannedConnector?.notes ??
                        connectorSetupDetails[connector.clientId] ??
                        "Switchboard applies local connector configuration."}
                    </p>
                  ) : null}
                  {openConnectorWarningId === connector.clientId &&
                  supportWarning ? (
                    <p className="connector-tooltip connector-tooltip--warning">
                      {supportWarning}
                    </p>
                  ) : null}
                  {needsRestart ? (
                    <p className="connector-item__restart">
                      Restart {connector.name} to apply changes.
                    </p>
                  ) : null}
                  {(detectionWarning ?? unavailableReason) ? (
                    <p className="connector-item__reason">
                      {detectionWarning ?? unavailableReason}
                    </p>
                  ) : null}
                </div>
                <div className="connector-item__controls">
                  <button
                    aria-checked={connector.enabled}
                    aria-label={`${connector.enabled ? "Disable" : "Enable"} ${connector.name} connector`}
                    className={`connector-switch${connector.enabled ? " is-on" : ""}`}
                    disabled={connectorsBusy}
                    onClick={() =>
                      void toggleConnector(connector, !connector.enabled)
                    }
                    role="switch"
                    title={unavailableReason ?? undefined}
                    type="button"
                  >
                    <span className="connector-switch__thumb" />
                  </button>
                </div>
              </article>
            );
          })}
        </div>
        {unavailableConnectors.length > 0 ? (
          <div className="connector-list connector-list--unavailable">
            <p className="connector-list__section-label">
              Not detected on this machine
            </p>
            {unavailableConnectors.map((connector) => {
              const unavailableReason = getConnectorUnavailableReason(connector);
              const supportWarning =
              connectorSupportWarnings[connector.clientId] ?? null;
              return (
                <article
                  className="connector-item is-unavailable"
                  key={connector.clientId}
                >
                  <div>
                    <h3>
                      <span className="client-logo" aria-hidden="true">
                        <ConnectorLogo clientId={connector.clientId} />
                      </span>
                      {connector.name}
                      {supportWarning ? (
                        <button
                          className="connector-warning-help"
                          onClick={() =>
                            setOpenConnectorWarningId((current) =>
                              current === connector.clientId
                                ? null
                                : connector.clientId,
                            )
                          }
                          type="button"
                          aria-label={`Show warning for ${connector.name}`}
                          aria-expanded={
                            openConnectorWarningId === connector.clientId
                          }
                        >
                          !
                        </button>
                      ) : null}
                    </h3>
                    {openConnectorWarningId === connector.clientId &&
                    supportWarning ? (
                      <p className="connector-tooltip connector-tooltip--warning">
                        {supportWarning}
                      </p>
                    ) : null}
                    {unavailableReason ? (
                      <p className="connector-item__reason">
                        {unavailableReason}
                      </p>
                    ) : null}
                  </div>
                </article>
              );
            })}
          </div>
        ) : null}
        {connectorsError ? (
          <p className="install-progress__error">{connectorsError}</p>
        ) : null}
      </div>
      <div className="post-install__actions">
        <button
          className="secondary-button post-install__reopen-setup"
          onClick={() => {
            setLauncherStage("install");
          }}
          type="button"
        >
          Back
        </button>
        <button
          className="primary-button primary-button--large primary-button--success"
          disabled={
            connectorsBusy || (requireSelection && enabledConnectorCount === 0)
          }
          onClick={() => void onContinue()}
          type="button"
        >
          Continue
        </button>
      </div>
    </LauncherShell>
  );
}
