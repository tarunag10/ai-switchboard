import {
  CurrencyCircleDollar,
  Cpu,
  Info,
} from "@phosphor-icons/react";
import {
  aggregateClientConnectors,
  compactNumber,
  connectorDashboardStatus,
  currency,
  sortClientConnectors,
} from "../lib/dashboardHelpers";
import {
  upgradePlanIntentLabel,
  tierRecommendationSourceLabel,
  shouldOfferRuntimeRestartAction,
  type UpgradePlanId,
} from "../lib/appHelpers";
import { shouldShowCodexNudge } from "../lib/trayHelpers";
import type {
  ClientConnectorStatus,
  DashboardState,
  DoctorReport,
  HeadroomPricingStatus,
  ManagedFootprintReport,
  OutputReduction,
  RuntimeStatus,
  SavingsAttributionEvent,
  SavingsMode,
  SwitchboardMode,
  TierMismatch,
} from "../lib/types";
import type { SavingsCalculatorScope } from "../lib/savingsCalculator";
import type { RepoSavingsEstimate } from "../lib/repoIntelligence";
import type { ActivityFeedResponse } from "../lib/types";
import { SwitchboardPanel } from "./SwitchboardPanel";
import { SwitchboardDoctorPanel } from "./SwitchboardDoctorPanel";
import { SavingsCalculatorCard } from "./SavingsCalculatorCard";
import { ClientSavingsTrendsCard } from "./ClientSavingsTrendsCard";
import { DailySavingsChart } from "./DailySavingsChart";
import { OutputReductionChip } from "./OutputReductionChip";
import type { SavingsChartMode } from "./SavingsChartTooltip";

export interface HomeViewProps {
  tierMismatch: TierMismatch | null;
  upgradeActionError: string | null;
  upgradeActionBusy: UpgradePlanId | null;
  handleUpgradeAction: (planId: UpgradePlanId) => void;
  calloutBanner: {
    tone: string;
    title: string;
  };
  calloutTitle: string;
  platformPreviewNotice: string | null;
  showRuntimeRestartAction: boolean;
  handleResumeRuntime: () => void;
  resuming: boolean;
  resumeError: string | null;
  connectorPhase: "disabled" | "verifying" | "healthy";
  beginProxyVerificationStep: () => void;
  connectors: ClientConnectorStatus[];
  pricingStatus: HeadroomPricingStatus | null;
  codexNudgeDismissed: boolean;
  localOnlyMode: boolean;
  connectorsBusy: boolean;
  toggleConnector: (connector: ClientConnectorStatus, enabled: boolean) => void;
  dismissCodexNudge: () => void;
  switchboardMode: SwitchboardMode;
  switchboardEffectiveMode: SwitchboardMode;
  switchboardNeedsAttention: boolean;
  switchboardModeCopy: string;
  switchboardLocalOnly: boolean;
  switchboardProxyStatus: string;
  switchboardHeadroomLabel: string;
  switchboardRtkLabel: string;
  switchboardRtkDetail: string;
  switchboardConnectors: ClientConnectorStatus[];
  dashboard: DashboardState;
  savingsMode: SavingsMode;
  savingsModeBusy: SavingsMode | null;
  runtimeStatus: RuntimeStatus | null;
  switchboardModeBusy: SwitchboardMode | null;
  switchboardModeError: string | null;
  switchboardInspectorRows: Array<{
    label: string;
    status: string;
    detail: string;
    actionLabel?: string;
    actionBusyLabel?: string;
    actionDisabled?: boolean;
    onAction?: () => void;
  }>;
  switchboardRemoteServicesEnabled: boolean;
  handleSetSwitchboardMode: (mode: SwitchboardMode) => void;
  handleSetSavingsMode: (mode: SavingsMode) => void;
  setActiveView: (view: string) => void;
  doctorReport: DoctorReport | null;
  doctorRepairBusy: string | null;
  doctorRepairError: string | null;
  doctorRepairSuccess: string | null;
  managedFootprintReport: ManagedFootprintReport | null;
  handleDoctorRepair: (action: string) => void;
  chartMode: SavingsChartMode;
  setChartMode: (mode: SavingsChartMode) => void;
  setShowSavingsInfo: (show: boolean) => void;
  savingsDashboard: DashboardState;
  savingsCalculatorRepoEstimate: RepoSavingsEstimate | null;
  activityFeed: ActivityFeedResponse;
  savingsAttributionEvents: SavingsAttributionEvent[];
  cavemanSavingsEstimate: import("../lib/savingsCalculator").AddonSavingsEstimate | null;
  ponytailSavingsEstimate: import("../lib/savingsCalculator").AddonSavingsEstimate | null;
  markitdownSavingsEstimate: import("../lib/savingsCalculator").AddonSavingsEstimate | null;
  savingsCalculatorScope: SavingsCalculatorScope;
  setSavingsCalculatorScope: (scope: SavingsCalculatorScope) => void;
  historyLoadTimedOut: boolean;
  chartResetSignal: number;
}

export function HomeView({
  tierMismatch,
  upgradeActionError,
  upgradeActionBusy,
  handleUpgradeAction,
  calloutBanner,
  calloutTitle,
  platformPreviewNotice,
  showRuntimeRestartAction,
  handleResumeRuntime,
  resuming,
  resumeError,
  connectorPhase,
  beginProxyVerificationStep,
  connectors,
  pricingStatus,
  codexNudgeDismissed,
  localOnlyMode,
  connectorsBusy,
  toggleConnector,
  dismissCodexNudge,
  switchboardMode,
  switchboardEffectiveMode,
  switchboardNeedsAttention,
  switchboardModeCopy,
  switchboardLocalOnly,
  switchboardProxyStatus,
  switchboardHeadroomLabel,
  switchboardRtkLabel,
  switchboardRtkDetail,
  switchboardConnectors,
  dashboard,
  savingsMode,
  savingsModeBusy,
  runtimeStatus,
  switchboardModeBusy,
  switchboardModeError,
  switchboardInspectorRows,
  switchboardRemoteServicesEnabled,
  handleSetSwitchboardMode,
  handleSetSavingsMode,
  setActiveView,
  doctorReport,
  doctorRepairBusy,
  doctorRepairError,
  doctorRepairSuccess,
  managedFootprintReport,
  handleDoctorRepair,
  chartMode,
  setChartMode,
  setShowSavingsInfo,
  savingsDashboard,
  savingsCalculatorRepoEstimate,
  activityFeed,
  savingsAttributionEvents,
  cavemanSavingsEstimate,
  ponytailSavingsEstimate,
  markitdownSavingsEstimate,
  savingsCalculatorScope,
  setSavingsCalculatorScope,
  historyLoadTimedOut,
  chartResetSignal,
}: HomeViewProps) {
  return (
    <div className="tray-content">
      {tierMismatch ? (
        <section className="tier-mismatch-banner" role="alert">
          <div className="tier-mismatch-banner__body">
            <h2 className="tier-mismatch-banner__title">
              Upgrade your Headroom plan
            </h2>
            <p className="tier-mismatch-banner__message">
              {tierMismatch.clamped
                ? `Your Headroom ${upgradePlanIntentLabel(tierMismatch.paidTier)} plan no longer matches your ${tierRecommendationSourceLabel(tierMismatch.recommendedSource)} usage, which needs ${upgradePlanIntentLabel(tierMismatch.recommendedTier)}, so weekly usage limits now apply. Upgrade to restore unlimited optimization.`
                : `You're on the Headroom ${upgradePlanIntentLabel(tierMismatch.paidTier)} plan but your ${tierRecommendationSourceLabel(tierMismatch.recommendedSource)} usage needs ${upgradePlanIntentLabel(tierMismatch.recommendedTier)}. Upgrade to match.`}
            </p>
            {upgradeActionError && upgradeActionBusy === null ? (
              <p className="tier-mismatch-banner__error" role="status">
                {upgradeActionError}
              </p>
            ) : null}
          </div>
          <button
            type="button"
            className="tier-mismatch-banner__action"
            disabled={upgradeActionBusy === tierMismatch.recommendedTier}
            onClick={() =>
              void handleUpgradeAction(tierMismatch.recommendedTier)
            }
          >
            {upgradeActionBusy === tierMismatch.recommendedTier
              ? "Updating…"
              : `Upgrade to ${upgradePlanIntentLabel(tierMismatch.recommendedTier)}`}
          </button>
        </section>
      ) : null}
      <section
        className={`callout-banner callout-banner--${calloutBanner.tone}`}
      >
        <span
          className={`callout-banner__dot callout-banner__dot--${calloutBanner.tone}`}
          aria-hidden="true"
        />
        <div className="callout-banner__body">
          <h1>{calloutTitle}</h1>
          {platformPreviewNotice ? (
            <p className="callout-banner__subtitle">
              {platformPreviewNotice}
            </p>
          ) : null}
          {calloutBanner.tone === "healthy" &&
            savingsDashboard.lifetimeEstimatedTokensSaved < 1_000_000 && (
              <p className="callout-banner__subtitle">
                Now use your connected tools as normal, and check back later
                to see how much you are saving by using Headroom.
              </p>
            )}
          {showRuntimeRestartAction ? (
            <div className="callout-banner__resume">
              <button
                type="button"
                className="callout-banner__action"
                onClick={() => void handleResumeRuntime()}
                disabled={resuming}
              >
                {resuming
                  ? "Restarting…"
                  : calloutBanner.tone === "paused" ||
                      calloutBanner.tone === "auto-paused"
                    ? "Resume"
                    : "Start runtime"}
              </button>
              {resumeError ? (
                <p
                  className="callout-banner__subtitle callout-banner__error"
                  role="status"
                >
                  {resumeError}
                </p>
              ) : null}
            </div>
          ) : null}
          {calloutBanner.tone === "starting" &&
          connectorPhase === "verifying" ? (
            <div className="callout-banner__resume">
              <button
                type="button"
                className="callout-banner__action"
                onClick={() => void beginProxyVerificationStep()}
              >
                Test setup
              </button>
            </div>
          ) : null}
        </div>
        {(() => {
          const homeConnectors = sortClientConnectors(
            aggregateClientConnectors(connectors),
          ).filter((connector) => connector.installed || connector.enabled);
          if (homeConnectors.length === 0) {
            return null;
          }
          return (
            <div className="callout-banner__connectors">
              {homeConnectors.map((connector) => {
                const status = connectorDashboardStatus(connector);
                return (
                  <span
                    className="callout-banner__chip"
                    key={connector.clientId}
                    title={status.label}
                  >
                    <span
                      className={`callout-banner__chip-dot callout-banner__chip-dot--${status.tone}`}
                      aria-hidden="true"
                    />
                    <span className="callout-banner__chip-name">
                      {connector.name}
                    </span>
                    <span className="visually-hidden">{status.label}</span>
                  </span>
                );
              })}
            </div>
          );
        })()}
      </section>

      {(() => {
        const codexConnector = aggregateClientConnectors(connectors).find(
          (connector) => connector.clientId === "codex",
        );
        const showCodexNudge = shouldShowCodexNudge(
          codexConnector,
          pricingStatus,
          codexNudgeDismissed,
          localOnlyMode,
        );
        if (!showCodexNudge || !codexConnector) {
          return null;
        }
        return (
          <section
            className="connector-nudge"
            aria-label="Codex now supported"
          >
            <div className="connector-nudge__body">
              <p className="connector-nudge__title">
                Headroom now supports Codex
              </p>
              <p className="connector-nudge__message">
                Route Codex through Headroom to trim its token costs too,
                the same way it already does for Claude Code.
              </p>
            </div>
            <button
              type="button"
              className="connector-nudge__action"
              disabled={connectorsBusy}
              onClick={() => void toggleConnector(codexConnector, true)}
            >
              Turn on Codex
            </button>
            <button
              type="button"
              className="connector-nudge__dismiss"
              aria-label="Dismiss Codex suggestion"
              onClick={dismissCodexNudge}
            >
              Dismiss
            </button>
          </section>
        );
      })()}

      <SwitchboardPanel
        mode={switchboardMode}
        effectiveMode={switchboardEffectiveMode}
        needsAttention={switchboardNeedsAttention}
        summary={switchboardModeCopy}
        localOnly={switchboardLocalOnly}
        proxyStatus={switchboardProxyStatus}
        headroomDetail={switchboardHeadroomLabel}
        rtkStatus={switchboardRtkLabel}
        rtkDetail={switchboardRtkDetail}
        connectors={switchboardConnectors}
        recentUsage={dashboard.recentUsage}
        savedHistory={dashboard.dailySavings}
        inspectorRows={switchboardInspectorRows}
        remoteServicesEnabled={switchboardRemoteServicesEnabled}
        savingsMode={savingsMode}
        savingsModeBusy={savingsModeBusy}
        paused={runtimeStatus?.paused === true}
        runtimeActionVisible={showRuntimeRestartAction}
        runtimeActionLabel={
          calloutBanner.tone === "paused" ||
          calloutBanner.tone === "auto-paused"
            ? "Resume runtime"
            : "Start runtime"
        }
        resuming={resuming}
        modeBusy={switchboardModeBusy}
        modeError={switchboardModeError}
        onSetMode={(mode) => void handleSetSwitchboardMode(mode)}
        onSetSavingsMode={(mode) => void handleSetSavingsMode(mode)}
        onResume={() => void handleResumeRuntime()}
        onManageClients={() => setActiveView("settings")}
        onManageRtk={() => setActiveView("addons")}
      />

      <section className="home-sector-grid" aria-label="Switchboard sectors">
        <button
          type="button"
          className="home-sector home-sector--primary"
          onClick={() => setActiveView("doctor")}
        >
          <span className="home-sector__tag">Health</span>
          <strong>Doctor</strong>
          <span>Repairs, evidence, and managed footprint checks.</span>
        </button>
        <button
          type="button"
          className="home-sector"
          onClick={() => setActiveView("usage")}
        >
          <span className="home-sector__tag">Savings</span>
          <strong>{currency(savingsDashboard.lifetimeEstimatedSavingsUsd)}</strong>
          <span>{compactNumber(savingsDashboard.lifetimeEstimatedTokensSaved)} tokens saved all-time.</span>
        </button>
        <button
          type="button"
          className="home-sector"
          onClick={() => setActiveView("addons")}
        >
          <span className="home-sector__tag">Tools</span>
          <strong>Add-ons</strong>
          <span>RTK, connectors, and planned tool support.</span>
        </button>
        <button
          type="button"
          className="home-sector"
          onClick={() => setActiveView("optimization")}
        >
          <span className="home-sector__tag">Learn</span>
          <strong>Optimization</strong>
          <span>Project learning, history, and routing guidance.</span>
        </button>
      </section>
    </div>
  );
}
