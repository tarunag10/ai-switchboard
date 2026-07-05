import type { Dispatch, SetStateAction } from "react";
import { Cpu, CurrencyCircleDollar, Info } from "@phosphor-icons/react";
import { compactNumber, currency } from "../lib/dashboardHelpers";
import type { RepoSavingsEstimate } from "../lib/repoIntelligence";
import type {
  AddonSavingsEstimate,
  SavingsCalculatorScope,
} from "../lib/savingsCalculator";
import type {
  ActivityFeedResponse,
  DashboardState,
  RuntimeStatus,
  SavingsAttributionEvent,
} from "../lib/types";
import { ClientSavingsTrendsCard } from "./ClientSavingsTrendsCard";
import { DailySavingsChart } from "./DailySavingsChart";
import { OutputReductionChip } from "./OutputReductionChip";
import { SavingsCalculatorCard } from "./SavingsCalculatorCard";
import type { SavingsChartMode } from "./SavingsChartTooltip";

interface UsageSavingsViewProps {
  hidden: boolean;
  chartMode: SavingsChartMode;
  setChartMode: Dispatch<SetStateAction<SavingsChartMode>>;
  setShowSavingsInfo: Dispatch<SetStateAction<boolean>>;
  savingsDashboard: DashboardState;
  dashboard: DashboardState;
  savingsCalculatorRepoEstimate: RepoSavingsEstimate;
  runtimeStatus: RuntimeStatus | null;
  activityFeed: ActivityFeedResponse;
  savingsAttributionEvents: SavingsAttributionEvent[];
  cavemanSavingsEstimate: AddonSavingsEstimate | null;
  ponytailSavingsEstimate: AddonSavingsEstimate | null;
  markitdownSavingsEstimate: AddonSavingsEstimate | null;
  savingsCalculatorScope: SavingsCalculatorScope;
  setSavingsCalculatorScope: Dispatch<SetStateAction<SavingsCalculatorScope>>;
  historyLoadTimedOut: boolean;
  chartResetSignal: number;
}

export function UsageSavingsView({
  hidden,
  chartMode,
  setChartMode,
  setShowSavingsInfo,
  savingsDashboard,
  dashboard,
  savingsCalculatorRepoEstimate,
  runtimeStatus,
  activityFeed,
  savingsAttributionEvents,
  cavemanSavingsEstimate,
  ponytailSavingsEstimate,
  markitdownSavingsEstimate,
  savingsCalculatorScope,
  setSavingsCalculatorScope,
  historyLoadTimedOut,
  chartResetSignal,
}: UsageSavingsViewProps) {
  return (
    <div className="tray-content" hidden={hidden}>
      <section className="repo-intelligence-view">
        <header className="repo-intelligence-view__header">
          <div>
            <h1>Usage and Savings</h1>
            <p className="repo-intelligence-view__subtitle">
              Review token savings, estimated cost savings, source breakdowns,
              and copyable savings summaries.
            </p>
          </div>
          <span className="repo-intelligence-view__badge">Menu bar</span>
        </header>
        <section className="stat-grid stat-grid--2col">
          <article
            className={`soft-card stat-card stat-card--clickable${chartMode === "usd" ? " is-active" : ""}`}
            onClick={() => setChartMode("usd")}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => e.key === "Enter" && setChartMode("usd")}
          >
            <span className="stat-card__label">
              <CurrencyCircleDollar
                aria-hidden="true"
                className="stat-card__icon"
                size={15}
                weight="bold"
              />
              All-time costs saved (estimate)
              <button
                className="stat-card__info-button"
                onClick={(e) => {
                  e.stopPropagation();
                  setShowSavingsInfo(true);
                }}
                type="button"
                aria-label="How savings are calculated"
              >
                <Info size={13} weight="bold" />
              </button>
            </span>
            <strong className="stat-value--green">
              {currency(savingsDashboard.lifetimeEstimatedSavingsUsd)}
            </strong>
          </article>
          <article
            className={`soft-card stat-card stat-card--clickable${chartMode === "tokens" ? " is-active" : ""}`}
            onClick={() => setChartMode("tokens")}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => e.key === "Enter" && setChartMode("tokens")}
          >
            <span className="stat-card__label">
              <Cpu
                aria-hidden="true"
                className="stat-card__icon"
                size={15}
                weight="bold"
              />
              All-time input tokens saved
            </span>
            <div className="stat-value-row">
              <strong className="stat-value--blue">
                {compactNumber(savingsDashboard.lifetimeEstimatedTokensSaved)}
              </strong>
              {savingsDashboard.outputReduction ? (
                <OutputReductionChip
                  reduction={savingsDashboard.outputReduction}
                />
              ) : null}
            </div>
          </article>
        </section>

        <SavingsCalculatorCard
          dashboard={dashboard}
          repoSavings={savingsCalculatorRepoEstimate}
          runtimeStatus={runtimeStatus}
          rtkToday={activityFeed.tiles.rtkToday}
          attributionEvents={savingsAttributionEvents}
          cavemanSavings={cavemanSavingsEstimate}
          ponytailSavings={ponytailSavingsEstimate}
          markitdownSavings={markitdownSavingsEstimate}
          scope={savingsCalculatorScope}
          onScopeChange={setSavingsCalculatorScope}
        />

        <ClientSavingsTrendsCard dashboard={dashboard} />

        {dashboard.savingsHistoryLoaded || historyLoadTimedOut ? (
          <DailySavingsChart
            data={savingsDashboard.dailySavings}
            hourlyData={savingsDashboard.hourlySavings}
            resetSignal={chartResetSignal}
            chartMode={chartMode}
            setChartMode={setChartMode}
          />
        ) : (
          <div className="savings-chart__skeleton" role="status">
            <p className="loading-copy">Loading savings history...</p>
          </div>
        )}
      </section>
    </div>
  );
}
