import type { MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

import { buildUpgradeIssueUrl } from "../lib/appSupport";
import type { RuntimeUpgradeFailure, RuntimeUpgradeProgress } from "../lib/types";
import { LauncherShell } from "./LauncherShell";

interface LauncherRuntimeUpgradeStepProps {
  appSemver: string;
  runtimeUpgradeProgress: RuntimeUpgradeProgress;
  showUpgradeModal: boolean;
  showUpgradeSuccess: boolean;
  upgradeFailure: RuntimeUpgradeFailure | null;
  upgradeExhausted: boolean;
  supportIssuesUrl: string;
  onMouseDown: (event: MouseEvent<HTMLElement>) => void;
  onFirstLaunchContinue: () => void | Promise<void>;
}

export function LauncherRuntimeUpgradeStep({
  appSemver,
  runtimeUpgradeProgress,
  showUpgradeModal,
  showUpgradeSuccess,
  upgradeFailure,
  upgradeExhausted,
  supportIssuesUrl,
  onMouseDown,
  onFirstLaunchContinue,
}: LauncherRuntimeUpgradeStepProps) {
  return (
    <LauncherShell
      shellClassName="intro-shell intro-shell--post-install"
      spinnerClassName="intro-shell__spinner intro-shell__spinner--post-install"
      copyClassName="intro-shell__copy intro-shell__copy--post-install"
      onMouseDown={onMouseDown}
      version={appSemver}
      showSpinner={showUpgradeModal}
    >
      {showUpgradeSuccess ? (
        <>
          <h1>
            {`Headroom ${runtimeUpgradeProgress.toVersion ?? ""} is ready`}
          </h1>
          <p className="launcher-install-notice">
            {runtimeUpgradeProgress.message}
          </p>
          <div className="install-progress-shell">
            <div className="install-progress" aria-live="polite">
              <div className="install-progress__bar-track">
                <div
                  className="install-progress__bar-fill"
                  style={{ width: "100%" }}
                />
              </div>
            </div>
          </div>
        </>
      ) : showUpgradeModal ? (
        <>
          <h1>
            {runtimeUpgradeProgress.toVersion
              ? `Finishing Headroom engine ${runtimeUpgradeProgress.toVersion} update…`
              : "Finishing Headroom engine update…"}
          </h1>
          <p className="launcher-install-notice">
            {runtimeUpgradeProgress.message ||
              "Wrapping up the Headroom engine update."}
          </p>
          <div className="install-progress-shell">
            <div className="install-progress" aria-live="polite">
              <div className="install-progress__bar-track">
                <div
                  className="install-progress__bar-fill"
                  style={{
                    width: `${runtimeUpgradeProgress.overallPercent}%`,
                  }}
                />
              </div>
              <div className="install-progress__meta">
                <p>{runtimeUpgradeProgress.currentStep}</p>
              </div>
            </div>
          </div>
        </>
      ) : upgradeFailure ? (
        <>
          <h1>
            {`Headroom ${upgradeFailure.appVersion} couldn't finish updating`}
          </h1>
          <p className="launcher-install-notice">
            {upgradeFailure.errorHint ??
              (upgradeFailure.fallbackHeadroomVersion
                ? "Running the previous version while we wait for you to retry."
                : "Running the previous version.")}
            {upgradeExhausted
              ? " We won't auto-retry on launch — click Retry to try again."
              : ""}
          </p>
          <div className="launcher-install-buttons">
            <button
              type="button"
              className="primary-button primary-button--large"
              onClick={() => void invoke("retry_runtime_upgrade")}
              disabled={runtimeUpgradeProgress.running}
            >
              Retry update
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => void onFirstLaunchContinue()}
            >
              Continue with previous version
            </button>
            {upgradeFailure.failurePhase === "boot_validation" ? (
              <button
                type="button"
                className="secondary-button"
                onClick={() => void invoke("retry_runtime_upgrade_with_rebuild")}
                disabled={runtimeUpgradeProgress.running}
              >
                Retry with full rebuild
              </button>
            ) : null}
            {upgradeFailure.failurePhase === "boot_validation" ? (
              <button
                type="button"
                className="secondary-button secondary-button--small"
                onClick={() =>
                  void invoke("open_external_link", {
                    url: buildUpgradeIssueUrl(supportIssuesUrl, upgradeFailure),
                  }).catch(() => {})
                }
              >
                Report issue
              </button>
            ) : null}
          </div>
        </>
      ) : null}
    </LauncherShell>
  );
}
