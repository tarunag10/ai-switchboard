import type { MouseEvent } from "react";

import type { ProxyVerificationRow } from "../lib/proxyVerification";
import { ConnectorLogo } from "./ConnectorLogo";
import { LauncherShell } from "./LauncherShell";

interface LauncherProxyVerifyStepProps {
  appSemver: string;
  proxyVerificationRows: ProxyVerificationRow[];
  proxyVerificationHint: string | null;
  connectorSmokeBusyId: string | null;
  onMouseDown: (event: MouseEvent<HTMLElement>) => void;
  onBack: () => void;
  onContinue: () => void;
  runAllSupportedConnectorSmokeTests: () => void | Promise<void>;
  runConnectorSmokeTest: (row: ProxyVerificationRow) => void | Promise<void>;
}

export function LauncherProxyVerifyStep({
  appSemver,
  proxyVerificationRows,
  proxyVerificationHint,
  connectorSmokeBusyId,
  onMouseDown,
  onBack,
  onContinue,
  runAllSupportedConnectorSmokeTests,
  runConnectorSmokeTest,
}: LauncherProxyVerifyStepProps) {
  const hasEnabledApps = proxyVerificationRows.length > 0;
  const hasOneClickTests = proxyVerificationRows.some(
    (row) => row.oneClickSupported && row.state !== "verified",
  );

  return (
    <LauncherShell
      shellClassName="intro-shell intro-shell--post-install"
      spinnerClassName="intro-shell__spinner intro-shell__spinner--post-install"
      copyClassName="intro-shell__copy intro-shell__copy--post-install"
      onMouseDown={onMouseDown}
      version={appSemver}
    >
      <div className="post-install__lead">
        <h1>Test your setup</h1>
        <p>
          Send automatic test prompts for Claude Code and Codex, then watch for
          a verified badge. For tools without automatic tests, open the tool and
          send one tiny prompt. Restart tools that were already open so they
          reload the managed config.
        </p>
        {hasOneClickTests ? (
          <button
            className="primary-button primary-button--large"
            disabled={connectorSmokeBusyId !== null}
            onClick={() => void runAllSupportedConnectorSmokeTests()}
            type="button"
          >
            {connectorSmokeBusyId !== null
              ? "Sending test prompts..."
              : "Send all test prompts"}
          </button>
        ) : null}
        {hasEnabledApps ? (
          <div className="connector-list">
            {proxyVerificationRows.map((row) => (
              <article className="connector-item" key={row.clientId}>
                <div>
                  <h3>
                    <span className="client-logo" aria-hidden="true">
                      <ConnectorLogo clientId={row.clientId} />
                    </span>
                    {row.name}
                  </h3>
                  <div className="proxy-verify-item__message">
                    <span>{row.message}</span>
                    {row.state === "verified" ? (
                      <span className="proxy-verified-pill">verified</span>
                    ) : null}
                  </div>
                </div>
                {row.oneClickSupported && row.state !== "verified" ? (
                  <button
                    className="secondary-button connector-item__action"
                    disabled={connectorSmokeBusyId !== null}
                    onClick={() => void runConnectorSmokeTest(row)}
                    type="button"
                  >
                    {connectorSmokeBusyId === row.clientId
                      ? "Sending..."
                      : "Send test prompt"}
                  </button>
                ) : null}
              </article>
            ))}
          </div>
        ) : (
          <p className="launcher-restart-hint">
            No tools are enabled yet. Go back to the previous step and enable
            one.
          </p>
        )}
        {proxyVerificationHint ? (
          <p className="install-progress__error">{proxyVerificationHint}</p>
        ) : null}
      </div>
      <div className="post-install__actions">
        <button
          className="secondary-button post-install__reopen-setup"
          onClick={onBack}
          type="button"
        >
          Back
        </button>
        <button
          className="primary-button primary-button--large primary-button--success"
          onClick={onContinue}
          type="button"
        >
          Continue
        </button>
      </div>
    </LauncherShell>
  );
}
