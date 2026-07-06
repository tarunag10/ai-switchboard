import { type RefObject } from "react";
import { percent1 } from "../lib/dashboardHelpers";
import type { AppUpdateConfiguration, RuntimeStatus } from "../lib/types";

interface SettingsRuntimeStatusCardProps {
  appSemver: string;
  appUpdateConfig: AppUpdateConfiguration | null;
  appUpdateBusy: boolean;
  appUpdateInstallBusy: boolean;
  appUpdateStatusCopy: string | null;
  checkForAppUpdate: () => void;
  headroomVersion: string;
  headroomLifetimeSavingsPct: number | null;
  runtimeStatus: RuntimeStatus | null;
  kompressWarming: boolean;
  runtimeActionError?: string | null;
  runtimeLabel: string;
  proxyPortLabel?: string;
  showHeadroomDetails: boolean;
  headroomLogLines: string[];
  headroomLogRef: RefObject<HTMLPreElement | null>;
  onOpenHeadroomDashboard: () => void;
  onToggleHeadroomDetails: () => void;
  showLogsLabel?: string;
  hideLogsLabel?: string;
}

export function SettingsRuntimeStatusCard({
  appSemver,
  appUpdateConfig,
  appUpdateBusy,
  appUpdateInstallBusy,
  appUpdateStatusCopy,
  checkForAppUpdate,
  headroomVersion,
  headroomLifetimeSavingsPct,
  runtimeStatus,
  kompressWarming,
  runtimeActionError = null,
  runtimeLabel,
  proxyPortLabel = "6767",
  showHeadroomDetails,
  headroomLogLines,
  headroomLogRef,
  onOpenHeadroomDashboard,
  onToggleHeadroomDetails,
  showLogsLabel = "Show runtime logs",
  hideLogsLabel = "Hide runtime logs",
}: SettingsRuntimeStatusCardProps) {
  const statusItems = [
    {
      name: "Runtime",
      ok: runtimeStatus?.running === true,
    },
    {
      name: "Proxy",
      ok: runtimeStatus?.proxyReachable === true,
      suffix: proxyPortLabel,
      onClick: onOpenHeadroomDashboard,
    },
    {
      name: "MCP",
      ok:
        runtimeStatus?.mcpConfigured === true
          ? true
          : runtimeStatus?.mcpConfigured === false
            ? false
            : null,
    },
    {
      name: "Kompress",
      ok: kompressWarming
        ? null
        : runtimeStatus?.kompressEnabled === true
          ? true
          : runtimeStatus?.kompressEnabled === false
            ? false
            : null,
      suffix: kompressWarming ? "warming up" : undefined,
    },
  ] as {
    name: string;
    ok: boolean | null;
    suffix?: string;
    onClick?: () => void;
  }[];

  return (
    <article className="soft-card panel-card">
      <div className="panel-card__header">
        <div>
          <h3>Tools status</h3>
        </div>
      </div>
      <div className="runtime-status">
        <div className="runtime-status__topline">
          <span className="runtime-status__section-title">
            AI Switchboard for Mac app ({appSemver})
            {appUpdateConfig?.betaChannelEnabled ? (
              <span className="runtime-status__channel-pill">beta channel</span>
            ) : null}
          </span>
        </div>
        <div className="runtime-status__section-action-row">
          <button
            className="secondary-button secondary-button--small"
            disabled={appUpdateBusy || appUpdateInstallBusy}
            onClick={checkForAppUpdate}
            type="button"
          >
            {appUpdateBusy ? "Checking…" : "Check for updates"}
          </button>
          {appUpdateStatusCopy ? (
            <p className="app-update-card__summary runtime-status__summary">
              {appUpdateStatusCopy}
            </p>
          ) : null}
        </div>
        <div className="runtime-status__meta">
          <span className="runtime-status__section-title">
            {runtimeLabel} ({headroomVersion})
            {headroomLifetimeSavingsPct !== null ? (
              <span className="runtime-status__section-context">
                {" "}
                ({percent1(headroomLifetimeSavingsPct)}% all-time savings)
              </span>
            ) : null}
          </span>
        </div>
        {runtimeActionError ? (
          <p className="runtime-status__error">{runtimeActionError}</p>
        ) : null}
        <div className="runtime-status__grid runtime-status__grid--4">
          {statusItems.map((s) => {
            const indicatorClass =
              s.ok === true
                ? "runtime-status__indicator--ok"
                : s.ok === false
                  ? "runtime-status__indicator--off"
                  : "runtime-status__indicator--unknown";
            const indicatorSymbol =
              s.ok === true ? "✔" : s.ok === false ? "✖" : "–";
            return (
              <span
                key={s.name}
                className={`runtime-status__item${s.onClick ? " runtime-status__item--clickable" : ""}`}
                onClick={s.onClick}
                title={s.ok === null ? `${s.name} status unknown` : undefined}
              >
                <span className="runtime-status__label">{s.name}:</span>
                <span className={`runtime-status__indicator ${indicatorClass}`}>
                  {indicatorSymbol}
                </span>
                {s.suffix && (
                  <span className="runtime-status__suffix">({s.suffix})</span>
                )}
              </span>
            );
          })}
        </div>
        <button
          className="link-button runtime-status__section-action"
          onClick={onToggleHeadroomDetails}
          type="button"
        >
          {showHeadroomDetails ? hideLogsLabel : showLogsLabel}
        </button>
        {showHeadroomDetails ? (
          <pre
            className="runtime-log"
            ref={headroomLogRef as RefObject<HTMLPreElement>}
          >
            {headroomLogLines.length > 0
              ? headroomLogLines.join("\n")
              : "No log output yet."}
          </pre>
        ) : null}
      </div>
    </article>
  );
}
