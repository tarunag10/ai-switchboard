import type { MouseEvent } from "react";
import { Cpu, CurrencyDollar } from "@phosphor-icons/react";

import { compactNumber, currency } from "../lib/dashboardHelpers";
import type { DashboardState } from "../lib/types";
import { LauncherShell } from "./LauncherShell";

interface LauncherPostInstallStepProps {
  appSemver: string;
  dashboard: DashboardState;
  savingsDashboard: DashboardState;
  lifetimeDataDays: number;
  lifetimeDataDaysLabel: string;
  onMouseDown: (event: MouseEvent<HTMLElement>) => void;
  onBack: () => void | Promise<void>;
  onGetStarted: () => void;
}

export function LauncherPostInstallStep({
  appSemver,
  dashboard,
  savingsDashboard,
  lifetimeDataDays,
  lifetimeDataDaysLabel,
  onMouseDown,
  onBack,
  onGetStarted,
}: LauncherPostInstallStepProps) {
  return (
    <LauncherShell
      shellClassName="intro-shell intro-shell--post-install"
      spinnerClassName="intro-shell__spinner intro-shell__spinner--post-install"
      copyClassName="intro-shell__copy intro-shell__copy--post-install"
      onMouseDown={onMouseDown}
      version={appSemver}
    >
      <div className="post-install__lead">
        <h1>
          AI Switchboard is ready
          <br />
          in the menu bar
        </h1>
        {dashboard.launchExperience === "first_run" ? (
          <p>
            Use Test setup to send a first prompt automatically where supported,
            or send your first prompt from a connected tool. Switchboard will
            route through the local Headroom engine and track savings
            automatically.
          </p>
        ) : (
          <>
            <p>
              Switchboard will trim prompt bloat whenever you use enabled
              clients such as Claude Code or Codex.
            </p>
            <div className="post-install__metrics">
              <article className="soft-card stat-card">
                <span className="stat-card__label">
                  <CurrencyDollar
                    aria-hidden="true"
                    className="stat-card__icon"
                    size={15}
                    weight="bold"
                  />
                  Savings all-time
                </span>
                <strong className="stat-value--green">
                  {currency(savingsDashboard.lifetimeEstimatedSavingsUsd)}
                </strong>
                <p>{lifetimeDataDaysLabel}</p>
              </article>
              <article className="soft-card stat-card">
                <span className="stat-card__label">
                  <Cpu
                    aria-hidden="true"
                    className="stat-card__icon"
                    size={15}
                    weight="bold"
                  />
                  Tokens saved all-time
                </span>
                <strong className="stat-value--blue">
                  {compactNumber(savingsDashboard.lifetimeEstimatedTokensSaved)}
                </strong>
                <p>
                  Across{" "}
                  {lifetimeDataDays > 0
                    ? `${lifetimeDataDays} tracked day${lifetimeDataDays === 1 ? "" : "s"}`
                    : "all recorded usage"}
                </p>
              </article>
            </div>
          </>
        )}
      </div>
      <div className="post-install__actions">
        <button
          className="secondary-button post-install__reopen-setup"
          onClick={() => void onBack()}
          type="button"
        >
          Back
        </button>
        <button
          className="primary-button primary-button--large primary-button--success"
          onClick={onGetStarted}
          type="button"
        >
          Get started
        </button>
        <p>Headroom stays active in your menu bar while you work.</p>
      </div>
    </LauncherShell>
  );
}
