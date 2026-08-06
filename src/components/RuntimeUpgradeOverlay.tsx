import { invoke } from "@tauri-apps/api/core";
import { buildUpgradeIssueUrl } from "../lib/appSupport";
import type { RuntimeUpgradeFailure, RuntimeUpgradeProgress } from "../lib/types";

export interface RuntimeUpgradeOverlayProps {
  runtimeUpgradeProgress: RuntimeUpgradeProgress;
  upgradeFailure: RuntimeUpgradeFailure | null;
  proxyReachable: boolean;
  supportIssuesUrl: string;
  maxUpgradeAutoRetries: number;
}

export function RuntimeUpgradeOverlay({
  runtimeUpgradeProgress,
  upgradeFailure,
  proxyReachable,
  supportIssuesUrl,
  maxUpgradeAutoRetries,
}: RuntimeUpgradeOverlayProps) {
  const showUpgradeModal =
    runtimeUpgradeProgress.running &&
    !runtimeUpgradeProgress.complete &&
    !runtimeUpgradeProgress.failed;
  const showUpgradeBanner =
    !runtimeUpgradeProgress.running && upgradeFailure !== null;
  const upgradeExhausted =
    upgradeFailure !== null &&
    upgradeFailure.attempts >= maxUpgradeAutoRetries;
  const canDismissUpgradeFailure =
    upgradeFailure !== null &&
    upgradeFailure.rollbackRestored &&
    proxyReachable;

  return (
    <>
      {showUpgradeModal && (
        <div
          className="modal-backdrop runtime-upgrade-backdrop"
          role="dialog"
          aria-modal="true"
        >
          <div className="modal-card runtime-upgrade-modal">
            <h3>
              {runtimeUpgradeProgress.toVersion
                ? `Finishing Headroom engine update to ${runtimeUpgradeProgress.toVersion}…`
                : "Finishing Headroom engine update…"}
            </h3>
            <p className="runtime-upgrade-modal__sub">
              {runtimeUpgradeProgress.fromVersion
                ? `From ${runtimeUpgradeProgress.fromVersion}`
                : ""}
            </p>
            <div className="install-progress__bar-track">
              <div
                className="install-progress__bar-fill"
                style={{ width: `${runtimeUpgradeProgress.overallPercent}%` }}
              />
            </div>
            <p className="runtime-upgrade-modal__step">
              {runtimeUpgradeProgress.currentStep}
            </p>
            <p className="runtime-upgrade-modal__message">
              {runtimeUpgradeProgress.message}
            </p>
          </div>
        </div>
      )}
      {showUpgradeBanner && upgradeFailure && (
        <div
          className={`runtime-upgrade-banner runtime-upgrade-banner--${upgradeFailure.failurePhase}`}
          role="alert"
        >
          <div className="runtime-upgrade-banner__body">
            <strong>
              {upgradeFailure.failurePhase === "boot_validation"
                ? `headroom-ai ${upgradeFailure.targetHeadroomVersion} installed but didn't start.`
                : "Headroom engine update didn't finish."}
            </strong>
            <span>
              {upgradeFailure.errorHint ??
                (upgradeFailure.failurePhase === "boot_validation" &&
                upgradeFailure.fallbackHeadroomVersion
                  ? `Reverted to headroom-ai ${upgradeFailure.fallbackHeadroomVersion}.`
                  : "Running the previous headroom-ai version.")}
            </span>
            {upgradeExhausted && (
              <span className="runtime-upgrade-banner__note">
                We won't auto-retry on launch. Click Retry to try again.
              </span>
            )}
          </div>
          <div className="runtime-upgrade-banner__actions">
            <button
              type="button"
              className="primary-button primary-button--small"
              onClick={() => void invoke("retry_runtime_upgrade")}
              disabled={runtimeUpgradeProgress.running}
            >
              Retry now
            </button>
            {upgradeFailure.failurePhase === "boot_validation" && (
              <button
                type="button"
                className="secondary-button secondary-button--small"
                onClick={() =>
                  void invoke("retry_runtime_upgrade_with_rebuild")
                }
                disabled={runtimeUpgradeProgress.running}
              >
                Retry with full rebuild
              </button>
            )}
            {upgradeFailure.failurePhase === "boot_validation" && (
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
            )}
            {canDismissUpgradeFailure && (
              <button
                type="button"
                className="secondary-button secondary-button--small"
                onClick={() =>
                  void invoke("dismiss_runtime_upgrade_failure").catch(() => {})
                }
              >
                Dismiss
              </button>
            )}
          </div>
        </div>
      )}
    </>
  );
}
