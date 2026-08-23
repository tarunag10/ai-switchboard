import type { Dispatch, ReactElement, ReactNode, SetStateAction } from "react";
import { CaretLeft } from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import { ActivityFeed } from "./ActivityFeed";
import { AddonsView, type AddonsViewProps } from "./AddonsView";
import { AgentMemoryInspector } from "./AgentMemoryInspector";
import { DailyUsageBriefingView } from "./DailyUsageBriefingView";
import { DoctorView } from "./DoctorView";
import { HomeView, type HomeViewProps } from "./HomeView";
import { OptimizationView, type OptimizationViewProps } from "./OptimizationView";
import { RepoIntelligencePreview } from "./RepoIntelligencePreview";
import { RepoMapView } from "./RepoMapView";
import { RoutingModelsView } from "./RoutingModelsView";
import { SavingsInfoDialog } from "./SavingsInfoDialog";
import { TokenXrayView } from "./TokenXrayView";
import { TraySidebar } from "./TraySidebar";
import { UpgradeView, type UpgradeViewProps } from "./UpgradeView";
import { UsageSavingsView } from "./UsageSavingsView";
import { buildDoctorTimelinePreview } from "../lib/appSupport";
import { currency, formatDateTime } from "../lib/dashboardHelpers";
import {
  getPlanRenewalPriceLabel,
  isTierDowngrade,
  upgradePlanIntentLabel,
  type BillingPeriod,
} from "../lib/appHelpers";
import type { TrayView } from "../lib/trayHelpers";
import type {
  AvailableAppUpdate,
  HeadroomPricingStatus,
  HeadroomSubscriptionTier,
} from "../lib/types";
import { estimateRepoIntelligenceSavings, type RepoIntelligenceSummary } from "../lib/repoIntelligence";
import type { SavingsChartMode } from "./SavingsChartTooltip";
import type { SavingsCalculatorScope } from "../lib/savingsCalculator";
import { repoIntelligencePreview } from "./RepoIntelligencePreview";

type TrayShellHomeProps = Omit<HomeViewProps, "hidden">;

interface TrayShellModalProps {
  upgradeOverlay: ReactNode;
  settingsView: ReactNode;
  pricingAuthCard: ReactElement;
  activityFeedError: string | null;
  activityFeedLoaded: boolean;
  setLatestRepoIntelligenceSummary: Dispatch<SetStateAction<RepoIntelligenceSummary>>;
  showSavingsInfo: boolean;
  showUninstallDialog: boolean;
  setShowUninstallDialog: Dispatch<SetStateAction<boolean>>;
  uninstallBusy: boolean;
  uninstallDisclosureTitle: string;
  uninstallDisclosureItems: { id: string; text: string; paths: string[] }[];
  uninstallDisclosureFooter: string;
  uninstallCopyNotice: string | null;
  uninstallError: string | null;
  copyUninstallDryRunReport: () => Promise<void>;
  handleUninstall: () => Promise<void>;
  pendingPlanChange: {
    fromTier: HeadroomSubscriptionTier;
    toTier: HeadroomSubscriptionTier;
    billingPeriod: BillingPeriod;
  } | null;
  cancelPlanChange: () => void;
  confirmPlanChange: () => Promise<void>;
  planChangeError: string | null;
  planChangeBusy: boolean;
  pricingStatus: HeadroomPricingStatus | null;
  showAppUpdateDialog: boolean;
  setShowAppUpdateDialog: Dispatch<SetStateAction<boolean>>;
  appUpdateAvailable: AvailableAppUpdate | null;
  appUpdateReadyToRestart: boolean;
  appUpdateInstallBusy: boolean;
  restartIntoInstalledUpdate: () => void;
  installAvailableUpdate: () => Promise<void>;
}

export interface TrayAppShellProps
  extends TrayShellHomeProps,
    OptimizationViewProps,
    Omit<UpgradeViewProps, "hidden">,
    AddonsViewProps,
    TrayShellModalProps {
  localOnlyMode: boolean;
}

export function TrayAppShell({
  upgradeOverlay,
  settingsView,
  activeView,
  setActiveView,
  localOnlyMode,
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
  masterActivationState,
  masterActivationProgress,
  masterFeatureStates,
  onActivateEverything,
  onDeactivateEverything,
  onActivateMasterFeature,
  onDeactivateMasterFeature,
  onOpenMasterFeature,
  masterActivationIsActive,
  masterOperation,
  onActivateMaxCompression,
  maxCompressionBusy,
  maxCompressionDisclosure,
  exactCacheRecommended,
  semanticCacheEnabled,
  onOpenCompressionPlaybook,
  headroomLearnSupported,
  headroomLearnDisabledReason,
  headroomLearnPrereq,
  headroomLearnStatus,
  headroomLearnBusy,
  claudeLearnEnabled,
  codexLearnEnabled,
  claudeProjectsBusy,
  claudeProjects,
  visibleClaudeProjects,
  sortedClaudeProjects,
  showAllClaudeProjects,
  setShowAllClaudeProjects,
  handleRunHeadroomLearn,
  copyLearnInstallCommand,
  openLearnInstallDocsLink,
  refreshHeadroomLearnPrereq,
  learnInstallCopyNotice,
  optimizeAppliedByProject,
  setOptimizeAppliedRefreshTick,
  claudeProjectsError,
  learnBlurb,
  prepareRepoMemoryMcp,
  setRepoMemoryMcpActive,
  activityFeedError,
  activityFeedLoaded,
  setLatestRepoIntelligenceSummary,
  addonError,
  addonCopy,
  addonInfoId,
  setAddonInfoId,
  addonBusyId,
  addonBusyLabel,
  addonResult,
  setAddonResult,
  rtkAvgSavingsPct,
  rtkBusy,
  openExternalLink,
  runAddonAction,
  onMeasuredAddonSavingsRecorded,
  handleRtkToggle,
  setCavemanLevel,
  copyPlannedConnectorCommand,
  onSelectiveActivationComplete,
  pricingAudience,
  setPricingAudience,
  setUpgradeActionError,
  billingPeriod,
  setBillingPeriod,
  upgradeTrialCallout,
  authRequestBusy,
  authVerifyBusy,
  upgradePlansState,
  visibleUpgradePlans,
  activeHeadroomPlanId,
  handleContactSubmit,
  contactEmail,
  setContactEmail,
  contactSubmitError,
  setContactSubmitError,
  contactSubmitSuccess,
  setContactSubmitSuccess,
  contactMessage,
  setContactMessage,
  contactEmailValid,
  contactSubmitBusy,
  handleReactivateSubscription,
  reactivateBusy,
  hasHiddenUpgradePlans,
  showAllUpgradePlans,
  setShowAllUpgradePlans,
  reactivateError,
  pricingAuthCard,
  showSavingsInfo,
  showUninstallDialog,
  setShowUninstallDialog,
  uninstallBusy,
  uninstallDisclosureTitle,
  uninstallDisclosureItems,
  uninstallDisclosureFooter,
  uninstallCopyNotice,
  uninstallError,
  copyUninstallDryRunReport,
  handleUninstall,
  pendingPlanChange,
  cancelPlanChange,
  confirmPlanChange,
  planChangeError,
  planChangeBusy,
  showAppUpdateDialog,
  setShowAppUpdateDialog,
  appUpdateAvailable,
  appUpdateReadyToRestart,
  appUpdateInstallBusy,
  restartIntoInstalledUpdate,
  installAvailableUpdate,
}: TrayAppShellProps) {
  return (
    <main className="tray-shell">
      {upgradeOverlay}
      <TraySidebar
        activeView={activeView}
        localOnlyMode={localOnlyMode}
        onSelectView={setActiveView}
      />

      <section className="tray-panel">
        <HomeView
          hidden={activeView !== "home"}
          tierMismatch={tierMismatch}
          upgradeActionError={upgradeActionError}
          upgradeActionBusy={upgradeActionBusy}
          handleUpgradeAction={(planId) => void handleUpgradeAction(planId)}
          calloutBanner={calloutBanner}
          calloutTitle={calloutTitle}
          platformPreviewNotice={platformPreviewNotice}
          showRuntimeRestartAction={showRuntimeRestartAction}
          handleResumeRuntime={() => void handleResumeRuntime()}
          resuming={resuming}
          resumeError={resumeError}
          connectorPhase={connectorPhase}
          beginProxyVerificationStep={() => void beginProxyVerificationStep()}
          connectors={connectors}
          pricingStatus={pricingStatus}
          codexNudgeDismissed={codexNudgeDismissed}
          localOnlyMode={localOnlyMode}
          connectorsBusy={connectorsBusy}
          toggleConnector={(connector, enabled) => void toggleConnector(connector, enabled)}
          dismissCodexNudge={dismissCodexNudge}
          switchboardMode={switchboardMode}
          switchboardEffectiveMode={switchboardEffectiveMode}
          switchboardNeedsAttention={switchboardNeedsAttention}
          switchboardModeCopy={switchboardModeCopy}
          switchboardLocalOnly={switchboardLocalOnly}
          switchboardProxyStatus={switchboardProxyStatus}
          switchboardHeadroomLabel={switchboardHeadroomLabel}
          switchboardRtkLabel={switchboardRtkLabel}
          switchboardRtkDetail={switchboardRtkDetail}
          switchboardConnectors={switchboardConnectors}
          dashboard={dashboard}
          savingsMode={savingsMode}
          savingsModeBusy={savingsModeBusy}
          runtimeStatus={runtimeStatus}
          switchboardModeBusy={switchboardModeBusy}
          switchboardModeError={switchboardModeError}
          switchboardInspectorRows={switchboardInspectorRows}
          switchboardRemoteServicesEnabled={switchboardRemoteServicesEnabled}
          handleSetSwitchboardMode={(mode) => void handleSetSwitchboardMode(mode)}
          handleSetSavingsMode={(mode) => void handleSetSavingsMode(mode)}
          setActiveView={setActiveView}
          doctorReport={doctorReport}
          doctorRepairBusy={doctorRepairBusy}
          doctorRepairError={doctorRepairError}
          doctorRepairSuccess={doctorRepairSuccess}
          managedFootprintReport={managedFootprintReport}
          handleDoctorRepair={(action) => void handleDoctorRepair(action)}
          chartMode={chartMode}
          setChartMode={setChartMode}
          setShowSavingsInfo={setShowSavingsInfo}
          savingsDashboard={savingsDashboard}
          savingsCalculatorRepoEstimate={savingsCalculatorRepoEstimate}
          activityFeed={activityFeed}
          savingsAttributionEvents={savingsAttributionEvents}
          cavemanSavingsEstimate={cavemanSavingsEstimate}
          ponytailSavingsEstimate={ponytailSavingsEstimate}
          markitdownSavingsEstimate={markitdownSavingsEstimate}
          savingsCalculatorScope={savingsCalculatorScope}
          setSavingsCalculatorScope={setSavingsCalculatorScope}
          historyLoadTimedOut={historyLoadTimedOut}
          chartResetSignal={chartResetSignal}
          masterActivationState={masterActivationState}
          masterActivationProgress={masterActivationProgress}
          masterFeatureStates={masterFeatureStates}
          onActivateEverything={onActivateEverything}
          onDeactivateEverything={onDeactivateEverything}
          onActivateMasterFeature={onActivateMasterFeature}
          onDeactivateMasterFeature={onDeactivateMasterFeature}
          onOpenMasterFeature={onOpenMasterFeature}
          masterActivationIsActive={masterActivationIsActive}
          masterOperation={masterOperation}
          onActivateMaxCompression={onActivateMaxCompression}
          maxCompressionBusy={maxCompressionBusy}
          maxCompressionDisclosure={maxCompressionDisclosure}
          exactCacheRecommended={exactCacheRecommended}
          semanticCacheEnabled={semanticCacheEnabled}
          onOpenCompressionPlaybook={onOpenCompressionPlaybook}
        />

        <UsageSavingsView
          hidden={activeView !== "usage"}
          chartMode={chartMode}
          setChartMode={
            setChartMode as Dispatch<SetStateAction<SavingsChartMode>>
          }
          setShowSavingsInfo={
            setShowSavingsInfo as Dispatch<SetStateAction<boolean>>
          }
          savingsDashboard={savingsDashboard}
          dashboard={dashboard}
          savingsCalculatorRepoEstimate={
            savingsCalculatorRepoEstimate ??
            estimateRepoIntelligenceSavings(repoIntelligencePreview)
          }
          runtimeStatus={runtimeStatus}
          activityFeed={activityFeed}
          savingsAttributionEvents={savingsAttributionEvents}
          cavemanSavingsEstimate={cavemanSavingsEstimate}
          ponytailSavingsEstimate={ponytailSavingsEstimate}
          markitdownSavingsEstimate={markitdownSavingsEstimate}
          savingsCalculatorScope={savingsCalculatorScope}
          setSavingsCalculatorScope={
            setSavingsCalculatorScope as Dispatch<
              SetStateAction<SavingsCalculatorScope>
            >
          }
          historyLoadTimedOut={historyLoadTimedOut}
          chartResetSignal={chartResetSignal}
        />

        <TokenXrayView hidden={activeView !== "xray"} />

        <DailyUsageBriefingView
          hidden={activeView !== "briefing"}
          onNavigate={(view) => setActiveView(view as TrayView)}
        />

        <AgentMemoryInspector hidden={activeView !== "agentMemory"} />

        <DoctorView
          hidden={activeView !== "doctor"}
          report={doctorReport}
          busyAction={doctorRepairBusy}
          error={doctorRepairError}
          successMessage={doctorRepairSuccess}
          footprintReport={managedFootprintReport}
          onRepair={(action) => void handleDoctorRepair(action)}
          timelineEvents={buildDoctorTimelinePreview(doctorReport, doctorRepairSuccess)}
        />

        <OptimizationView
          activeView={activeView}
          setActiveView={setActiveView}
          headroomLearnSupported={headroomLearnSupported}
          headroomLearnDisabledReason={headroomLearnDisabledReason}
          headroomLearnPrereq={headroomLearnPrereq}
          headroomLearnStatus={headroomLearnStatus}
          headroomLearnBusy={headroomLearnBusy}
          claudeLearnEnabled={claudeLearnEnabled}
          codexLearnEnabled={codexLearnEnabled}
          claudeProjectsBusy={claudeProjectsBusy}
          claudeProjects={claudeProjects}
          visibleClaudeProjects={visibleClaudeProjects}
          sortedClaudeProjects={sortedClaudeProjects}
          showAllClaudeProjects={showAllClaudeProjects}
          setShowAllClaudeProjects={setShowAllClaudeProjects}
          handleRunHeadroomLearn={handleRunHeadroomLearn}
          copyLearnInstallCommand={copyLearnInstallCommand}
          openLearnInstallDocsLink={openLearnInstallDocsLink}
          refreshHeadroomLearnPrereq={refreshHeadroomLearnPrereq}
          learnInstallCopyNotice={learnInstallCopyNotice}
          optimizeAppliedByProject={optimizeAppliedByProject}
          setOptimizeAppliedRefreshTick={setOptimizeAppliedRefreshTick}
          claudeProjectsError={claudeProjectsError}
          learnBlurb={learnBlurb}
          prepareRepoMemoryMcp={prepareRepoMemoryMcp}
          setRepoMemoryMcpActive={setRepoMemoryMcpActive}
        />

        <div className="tray-content" hidden={activeView !== "notifications"}>
            <ActivityFeed
              feed={activityFeed}
              error={activityFeedError}
              loaded={activityFeedLoaded}
              onNavigateToOptimize={() => setActiveView("optimization")}
            />
          </div>

          <div className="tray-content" hidden={activeView !== "repoMap"}>
            <RepoMapView
              onOpenDoctor={() => setActiveView("doctor")}
              onOpenRepoIntelligence={() => setActiveView("repoIntelligence")}
            />
          </div>

          <div
            className="tray-content tray-content--repo-intelligence"
            hidden={activeView !== "repoIntelligence"}
          >
          <section className="repo-intelligence-view">
            <header className="repo-intelligence-view__header">
              <div>
                <h1>Repo Intelligence</h1>
                <p className="repo-intelligence-view__subtitle">
                  Index a local repository, review graph signals, and copy
                  bounded context packs for coding agents.
                </p>
              </div>
              <span className="repo-intelligence-view__badge">Local only</span>
            </header>
            <RepoIntelligencePreview
              headroomHealthy={
                runtimeStatus?.proxyReachable === true &&
                runtimeStatus.running === true &&
                runtimeStatus.paused === false
              }
              onSummaryChange={setLatestRepoIntelligenceSummary}
              rtkHealthy={
                runtimeStatus?.rtk.installed === true &&
                runtimeStatus.rtk.enabled === true
              }
            />
          </section>
        </div>

        <RoutingModelsView hidden={activeView !== "routingModels"} />

        <AddonsView
          activeView={activeView}
          setActiveView={setActiveView}
          addonError={addonError}
          runtimeStatus={runtimeStatus}
          dashboard={dashboard}
          savingsAttributionEvents={savingsAttributionEvents}
          connectors={connectors}
          addonCopy={addonCopy}
          addonInfoId={addonInfoId}
          setAddonInfoId={setAddonInfoId}
          addonBusyId={addonBusyId}
          addonBusyLabel={addonBusyLabel}
          addonResult={addonResult}
          setAddonResult={setAddonResult}
          rtkAvgSavingsPct={rtkAvgSavingsPct}
          rtkBusy={rtkBusy}
          openExternalLink={openExternalLink}
          runAddonAction={runAddonAction}
          handleRtkToggle={handleRtkToggle}
          onMeasuredAddonSavingsRecorded={onMeasuredAddonSavingsRecorded}
          setCavemanLevel={setCavemanLevel}
          copyPlannedConnectorCommand={copyPlannedConnectorCommand}
          onSelectiveActivationComplete={onSelectiveActivationComplete}
        />

        <UpgradeView
          hidden={activeView !== "upgrade"}
          pricingAudience={pricingAudience}
          setPricingAudience={setPricingAudience}
          setUpgradeActionError={setUpgradeActionError}
          billingPeriod={billingPeriod}
          setBillingPeriod={setBillingPeriod}
          pricingStatus={pricingStatus}
          upgradeTrialCallout={upgradeTrialCallout}
          authRequestBusy={authRequestBusy}
          authVerifyBusy={authVerifyBusy}
          upgradeActionBusy={upgradeActionBusy}
          upgradePlansState={upgradePlansState}
          visibleUpgradePlans={visibleUpgradePlans}
          activeHeadroomPlanId={activeHeadroomPlanId}
          handleContactSubmit={handleContactSubmit}
          contactEmail={contactEmail}
          setContactEmail={setContactEmail}
          contactSubmitError={contactSubmitError}
          setContactSubmitError={setContactSubmitError}
          contactSubmitSuccess={contactSubmitSuccess}
          setContactSubmitSuccess={setContactSubmitSuccess}
          contactMessage={contactMessage}
          setContactMessage={setContactMessage}
          contactEmailValid={contactEmailValid}
          contactSubmitBusy={contactSubmitBusy}
          handleReactivateSubscription={() => void handleReactivateSubscription()}
          reactivateBusy={reactivateBusy}
          handleUpgradeAction={(planId) => void handleUpgradeAction(planId)}
          hasHiddenUpgradePlans={hasHiddenUpgradePlans}
          showAllUpgradePlans={showAllUpgradePlans}
          setShowAllUpgradePlans={setShowAllUpgradePlans}
          upgradeActionError={upgradeActionError}
          reactivateError={reactivateError}
        />

        <div
          className="tray-content tray-content--upgrade"
          hidden={activeView !== "upgradeAuth"}
        >
          <section className="upgrade-auth-view">
            <div className="upgrade-auth-view__header">
              <div className="upgrade-auth-view__title-row">
                <button
                  aria-label="Back to upgrade plans"
                  className="upgrade-auth-view__back"
                  onClick={() => setActiveView("upgrade")}
                  type="button"
                >
                  <CaretLeft size={16} weight="bold" />
                </button>
                <h1>Create account</h1>
              </div>
            </div>
            {pricingAuthCard}
          </section>
        </div>

        {settingsView}

        {showSavingsInfo && (
          <SavingsInfoDialog
            minimumEstimatedSavingsLabel={currency(
              savingsDashboard.lifetimeEstimatedSavingsUsd * 0.5,
            )}
            onClose={() => setShowSavingsInfo(false)}
          />
        )}

        {showUninstallDialog ? (
          <div
            className="modal-backdrop"
            role="dialog"
            aria-modal="true"
            onClick={() => {
              if (!uninstallBusy) {
                setShowUninstallDialog(false);
              }
            }}
          >
            <div className="modal-card" onClick={(e) => e.stopPropagation()}>
              <h3>{uninstallDisclosureTitle}</h3>
              <p>This will:</p>
              <ul className="api-key-guide">
                {uninstallDisclosureItems.map((item) => (
                  <li key={item.id}>
                    {item.text}
                    {item.paths.length > 0 ? (
                      <>
                        {" "}
                        {item.paths.map((path) => (
                          <code key={path}>{path}</code>
                        ))}
                      </>
                    ) : null}
                  </li>
                ))}
              </ul>
              <p>{uninstallDisclosureFooter}</p>
              {uninstallCopyNotice ? (
                <p className="rollback-center-card__notice">
                  {uninstallCopyNotice}
                </p>
              ) : null}
              {uninstallError ? (
                <p className="install-progress__error">{uninstallError}</p>
              ) : null}
              <div className="modal-actions">
                <button
                  className="secondary-button"
                  disabled={uninstallBusy}
                  onClick={() => void copyUninstallDryRunReport()}
                  type="button"
                >
                  Copy dry-run
                </button>
                <button
                  className="secondary-button"
                  disabled={uninstallBusy}
                  onClick={() => setShowUninstallDialog(false)}
                  type="button"
                >
                  Cancel
                </button>
                <button
                  className="primary-button"
                  disabled={uninstallBusy}
                  onClick={() => void handleUninstall()}
                  type="button"
                >
                  {uninstallBusy ? "Uninstalling…" : "Uninstall and quit"}
                </button>
              </div>
            </div>
          </div>
        ) : null}

        {pendingPlanChange
          ? (() => {
              const isDowngrade = isTierDowngrade(
                pendingPlanChange.fromTier,
                pendingPlanChange.toTier,
              );
              const action = isDowngrade ? "downgrade" : "upgrade";
              const actionTitle = isDowngrade ? "Downgrade" : "Upgrade";
              const currentPriceLabel = getPlanRenewalPriceLabel(
                pendingPlanChange.fromTier,
                pendingPlanChange.billingPeriod,
                {
                  fromTier: pendingPlanChange.fromTier,
                  currentPaidCents:
                    pricingStatus?.account?.subscriptionAmountCents,
                },
              );
              const newPriceLabel = getPlanRenewalPriceLabel(
                pendingPlanChange.toTier,
                pendingPlanChange.billingPeriod,
                {
                  fromTier: pendingPlanChange.fromTier,
                  currentPaidCents:
                    pricingStatus?.account?.subscriptionAmountCents,
                },
              );
              return (
                <div
                  className="modal-backdrop"
                  role="dialog"
                  aria-modal="true"
                  onClick={cancelPlanChange}
                >
                  <div
                    className="modal-card"
                    onClick={(e) => e.stopPropagation()}
                  >
                    <h3>Confirm your {action}</h3>
                    <p>
                      You'll {action} from your{" "}
                      <strong>{currentPriceLabel}</strong>{" "}
                      <strong>
                        {upgradePlanIntentLabel(pendingPlanChange.fromTier)}
                      </strong>{" "}
                      plan to the <strong>{newPriceLabel}</strong>{" "}
                      <strong>
                        {upgradePlanIntentLabel(pendingPlanChange.toTier)}
                      </strong>{" "}
                      plan, billed{" "}
                      {pendingPlanChange.billingPeriod === "annual"
                        ? "annually"
                        : "monthly"}
                      .
                    </p>
                    <p>
                      {isDowngrade
                        ? "You'll receive a prorated credit toward your next billing cycle for the unused time on your current plan."
                        : "You'll be charged a prorated amount today for the remaining time in your current billing period, with your existing discount applied."}
                    </p>
                    {pricingStatus?.account?.subscriptionRenewsAt ? (
                      <p>
                        Your subscription will then renew on{" "}
                        <strong>
                          {new Date(
                            pricingStatus.account.subscriptionRenewsAt,
                          ).toLocaleDateString(undefined, {
                            year: "numeric",
                            month: "long",
                            day: "numeric",
                          })}
                        </strong>
                        .
                      </p>
                    ) : null}
                    {planChangeError ? (
                      <p className="install-progress__error">
                        {planChangeError}
                      </p>
                    ) : null}
                    <div className="modal-actions">
                      <button
                        className="secondary-button"
                        disabled={planChangeBusy}
                        onClick={cancelPlanChange}
                        type="button"
                      >
                        Cancel
                      </button>
                      <button
                        className="primary-button"
                        disabled={planChangeBusy}
                        onClick={() => void confirmPlanChange()}
                        type="button"
                      >
                        {planChangeBusy
                          ? isDowngrade
                            ? "Downgrading…"
                            : "Upgrading…"
                          : `Confirm ${action}`}
                      </button>
                    </div>
                  </div>
                </div>
              );
            })()
          : null}

        {showAppUpdateDialog && appUpdateAvailable ? (
          <div className="modal-backdrop" role="dialog" aria-modal="true">
            <div className="modal-card">
              <h3>
                {appUpdateReadyToRestart
                  ? `Restart to finish updating ${appUpdateAvailable.version}`
                  : `AI Switchboard for Mac ${appUpdateAvailable.version} is available`}
              </h3>
              <p>
                {appUpdateReadyToRestart
                  ? "The new version has been installed. Restart AI Switchboard for Mac when you are ready to switch over."
                  : "AI Switchboard for Mac found a new release in the background. Nothing will install until you confirm it here."}
              </p>
              <ul className="api-key-guide">
                <li>Current version: {appUpdateAvailable.currentVersion}</li>
                <li>New version: {appUpdateAvailable.version}</li>
                <li>
                  Published:{" "}
                  {formatDateTime(appUpdateAvailable.publishedAt ?? null)}
                </li>
              </ul>
              {appUpdateAvailable.notes && appUpdateAvailable.notes.trim() ? (
                <div className="release-notes">
                  <h4>What&apos;s new</h4>
                  <pre>{appUpdateAvailable.notes.trim()}</pre>
                </div>
              ) : null}
              <div className="modal-actions">
                <button
                  className="secondary-button"
                  disabled={appUpdateInstallBusy}
                  onClick={() => setShowAppUpdateDialog(false)}
                  type="button"
                >
                  Later
                </button>
                <button
                  className="primary-button"
                  disabled={appUpdateInstallBusy}
                  onClick={() =>
                    appUpdateReadyToRestart
                      ? restartIntoInstalledUpdate()
                      : void installAvailableUpdate()
                  }
                  type="button"
                >
                  {appUpdateInstallBusy
                    ? "Installing…"
                    : appUpdateReadyToRestart
                      ? "Restart now"
                      : `Install ${appUpdateAvailable.version}`}
                </button>
              </div>
            </div>
          </div>
        ) : null}
      </section>
    </main>
  );
}
