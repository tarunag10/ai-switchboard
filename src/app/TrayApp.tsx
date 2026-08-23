import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent,
  type ReactElement,
} from "react";
import {
  ArrowClockwise,
  CaretLeft,
  Cpu,
  CurrencyDollar,
  SignOut,
} from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import {
  refreshDoctorReport as refreshDoctorReportController,
  runDoctorRepairAction,
} from "../lib/doctorRepairController";
import {
  runLocalReleaseEvidenceSequence as runLocalReleaseEvidenceSequenceController,
  runReleaseEvidenceCommand as runReleaseEvidenceCommandController,
  type ReleaseEvidenceCommandResult,
  type ReleaseReadinessReportPayload,
} from "../lib/releaseEvidenceController";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  formatAppUpdateProgressCopy,
  getAppUpdateInstallStatusCopy,
  getBlockedAppUpdateCheckPatch,
  loadAppUpdateConfiguration,
  runAppUpdateCheck,
  runAppUpdateInstall,
  sendAppUpdateNotification,
  shouldNotifyAboutAvailableAppUpdate,
  maybeFireStaleAppUpdateNotification,
  type AppUpdateStatePatch,
} from "../lib/appUpdate";
import { useHeadroomLearnController } from "../lib/headroomLearnController";
import { maybeFireTrialNotifications } from "../lib/trialNotifications";
import {
  maybeFireUrgentPricingNotifications,
  maybeFireUrgentRuntimeNotification,
} from "../lib/urgentNotifications";
import {
  estimateRepoIntelligenceSavings,
  type RepoIntelligenceSummary,
  type RepoSavingsEstimate,
} from "../lib/repoIntelligence";
import {
  RepoIntelligencePreview,
  repoIntelligencePreview,
} from "../components/RepoIntelligencePreview";
import { repoMemoryMcpLifecycle } from "../lib/repoMemoryMcp";
import {
  buildSwitchboardInspectorRows,
} from "../lib/trayInspectorRows";
import {
  delay,
  loadDashboard,
  loadSavingsAttributionEvents,
} from "../lib/trayLoaders";
import { isCurrentConnectorRefresh } from "../lib/connectorRefresh";
import { useMasterActivationController } from "../lib/useMasterActivationController";
import { useTrayPricingController } from "../lib/useTrayPricingController";
import {
  accountDisplayEmailFromPricing,
  accountPlanNameFromPricing,
  localGraceHoursRemainingFromPricing,
  trialDaysRemainingFromPricing,
  upgradeTrialCalloutFromPricing,
} from "../lib/trayPricingPresentation";
import {
  formatPlannedConnectorConfigCreationPlansMarkdown,
  getPlannedConnector,
  getPlannedConnectorConfigCreationPlan,
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorReadinessContract,
  getPlannedConnectorSetupChecklistScript,
  getPlannedConnectorSetupGuide,
  type PlannedConnector,
} from "../lib/plannedConnectors";
import {
  formatLocalReleaseEvidenceSequenceCopy,
  releaseReadinessCommand,
  formatReleaseReadinessCommandCopy,
  formatReleaseReadinessReportSnapshot,
  localReleaseEvidenceCommandIds,
  releaseLocalEvidenceRowsFromReport,
  releaseReadinessEvidenceSummary,
  releaseReadinessNextAction,
  releaseReadinessRowsFromReport,
  releaseReadinessStatusCounts,
  type ReleaseReadinessReportSnapshot,
} from "../lib/releaseReadiness";
import {
  describeInvokeError,
  getNextLowerUpgradePlanId,
  getPlanRenewalPriceLabel,
  getUpgradePlans,
  isTierDowngrade,
  shouldOfferRuntimeRestartAction,
  tierRecommendationSourceLabel,
  upgradePlanIntentLabel,
  type BillingPeriod,
  type PricingAudience,
  type UpgradePlanId,
} from "../lib/appHelpers";
import {
  bootstrapFailureSignature,
  buildBootstrapFailureReport,
  buildBootstrapInvokeFailureReport,
  reportBootstrapFailure,
} from "../lib/bootstrapSentry";
import {
  compactNumber,
  connectorControlState,
  connectorCompatibilityReport,
  connectorCompatibilityRoutingEvidenceLabel,
  connectorSupportsAutomaticSetup,
  currency,
  currencyExact,
  formatDateTime,
  formatDayKey,
  formatPlannedConnectorConfigGateSummary,
  getEnabledSupportedConnectors,
  hasEnabledConnector,
  percent1,
  sortClientConnectors,
  summarizePlannedConnectorReadiness,
  aggregateClientConnectors,
} from "../lib/dashboardHelpers";
import {
  buildInitialProxyVerificationRows,
  getContactRequestValidationError,
  getClaudeConnector,
  getInitialLauncherStage,
  getLauncherAutoConfigureDecision,
  hasPendingOneClickProxyVerification,
  isValidEmailAddress,
  needsTermsAcceptance,
  nextAutoConfigureStep,
  nextAutoConfigureStepAfterApply,
  shouldApplyConnectorSmokeResult,
  type LauncherStage,
} from "../lib/launcherHelpers";
import { mockDashboard } from "../lib/mockData";
import {
  cachePricingStatus,
  type CachedPricing,
  formatPercentValue,
  formatRemainingDays,
  readCachedPricing,
  subscriptionTierLabel,
  writeCachedPricing,
} from "../lib/pricing";
import {
  activityFeedSignature,
  notificationActionTargetId,
  safeNotificationActionView,
  safeTrayViewForMode,
  serializeState,
  type TrayView,
} from "../lib/trayHelpers";
import {
  trackAnalyticsEvent,
  trackInstallMilestoneOnce,
} from "../lib/analytics";
import type { ProxyVerificationRow } from "../lib/proxyVerification";
import { launcherConnectorFallback } from "../lib/launcherConnectorFallback";
import { localOnlyModeEnabled } from "../lib/localMode";
import {
  buildManagedRollbackExecutionPreview,
  buildManagedRollbackPlan,
  buildManagedRollbackUndoAllPreview,
  canExecuteNativeManagedRollbackPreview,
  buildManagedConfigDiffPreview,
  formatManagedFootprintReport,
  formatManagedRollbackExecutionPreview,
  formatManagedConfigDiffPreview,
  formatManagedRollbackPlan,
  formatManagedRollbackUndoAllPreview,
  formatManagedRollbackInventory,
  managedChangeRecords,
  supportsDedicatedCleanupRollbackRecord,
  type ManagedChangeRecord,
} from "../lib/managedChanges";
import {
  buildDoctorTimelinePreview,
  sampleManagedBlock,
} from "../lib/appSupport";
import {
  buildSettingsExportBundle,
  formatSettingsExportBundle,
  parseSettingsImport,
  type SettingsImportPreview,
} from "../lib/settingsTransfer";
import {
  CONTACT_FORM_URL,
  SALES_CONTACT_URL,
  SUPPORT_ISSUES_URL,
} from "../lib/supportUrls";
import {
  connectorSetupDetails,
  firstManagedConfigTarget,
  formatBackendConnectorConfigPlan,
  getConnectorDetectionWarning,
  getConnectorUnavailableReason,
  getPlannedConnectorNextStep,
  supportsNativeConfigApply,
  supportsNativeManagedRollback,
} from "../lib/settingsConnectorCopy";
import {
  formatBackendUninstallDryRunReport,
  formatUninstallDryRunReport,
  uninstallDisclosureFooter,
  uninstallDisclosureItems,
  uninstallDisclosureTitle,
} from "../lib/uninstallDisclosure";
import {
  deriveSwitchboardMode,
  switchboardModeSummary,
} from "../lib/switchboardDisplay";
import {
  buildAddonSavingsEstimate,
  CAVEMAN_TEMPLATE_BASELINE_TOKENS,
  CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS,
  PONYTAIL_TEMPLATE_BASELINE_TOKENS,
  PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS,
  MARKITDOWN_TEMPLATE_BASELINE_TOKENS,
  MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS,
  type SavingsCalculatorScope,
} from "../lib/savingsCalculator";
import { ActivityFeed } from "../components/ActivityFeed";
import { AddonsView } from "../components/AddonsView";
import { DoctorView } from "../components/DoctorView";
import { HomeView } from "../components/HomeView";
import { LauncherClientSetupStep } from "../components/LauncherClientSetupStep";
import { LauncherInstallStep } from "../components/LauncherInstallStep";
import { LauncherPostInstallStep } from "../components/LauncherPostInstallStep";
import { LauncherProxyVerifyStep } from "../components/LauncherProxyVerifyStep";
import { LauncherRuntimeUpgradeStep } from "../components/LauncherRuntimeUpgradeStep";
import { OptimizationView } from "../components/OptimizationView";
import { PricingAuthCard } from "../components/PricingAuthCard";
import { RepoMapView } from "../components/RepoMapView";
import { TraySidebar } from "../components/TraySidebar";
import type { SavingsChartMode } from "../components/SavingsChartTooltip";
import { SettingsView } from "../components/SettingsView";
import { TrayAppShell } from "../components/TrayAppShell";
import { TermsGate } from "../components/TermsGate";
import { UpgradeView } from "../components/UpgradeView";
import { UsageSavingsView } from "../components/UsageSavingsView";
import { TokenXrayView } from "../components/TokenXrayView";
import { DailyUsageBriefingView } from "../components/DailyUsageBriefingView";
import { AgentMemoryInspector } from "../components/AgentMemoryInspector";
import {
  MasterActivationCard,
  type MasterFeatureId,
  type MasterFeatureState,
  type MasterFeatureStatus,
} from "../components/MasterActivationCard";
import { recommendExactCacheDefault } from "../lib/exactCacheDefaultPolicy";
import { createMaxCompressionActivationPlan } from "../lib/maxCompressionActivation";
import { resolveSwitchboardModeForCache } from "../lib/switchboardModeForCache";
import { getAgentMemorySnapshot } from "../lib/agentMemory";
import {
  loadDailyUsageBriefing,
  loadTokenXraySnapshot,
} from "../lib/usageAnalytics";
import type {
  AppUpdateConfiguration,
  AvailableAppUpdate,
  BootstrapProgress,
  ClaudePlanTier,
  HeadroomAuthCodeRequest,
  HeadroomPricingStatus,
  ClaudeCodeProject,
  ClientConnectorStatus,
  ClientSetupResult,
  DailySavingsPoint,
  DashboardState,
  DoctorReport,
  HeadroomSubscriptionTier,
  ManagedConfigApplyPreview,
  ManagedConfigApplyResult,
  ManagedFootprintReport,
  ManagedRollbackExecutionResult,
  ManagedRollbackPreview,
  ManagedRollbackUndoAllExecutionResult,
  ManagedRollbackUndoAllPreview,
  ActivityFeedResponse,
  HourlySavingsPoint,
  OutputReduction,
  RuntimeStatus,
  RuntimeUpgradeProgress,
  SavingsAttributionEvent,
  SavingsMode,
  SwitchboardMode,
  SwitchboardState,
  UninstallDryRunReport,
} from "../lib/types";
import { hasTauriEventRuntime, hasTauriRuntime } from "../lib/tauriRuntime";
import { RuntimeUpgradeOverlay } from "../components/RuntimeUpgradeOverlay";
import {
  addonCopy,
  connectorSupportWarnings,
  idleBootstrapProgress,
  idleRuntimeUpgradeProgress,
  localFirstReadinessSourceSignals,
  MAX_UPGRADE_AUTO_RETRIES,
} from "../lib/trayAddonCopy";

type StartupPhase = "window" | "dashboard" | "bootstrap" | "runtime" | "ready";

const APP_UPDATE_BACKGROUND_INITIAL_DELAY_MS = 12_000;
const APP_UPDATE_BACKGROUND_CHECK_INTERVAL_MS = 60 * 60 * 1000;

interface ConnectorSmokeTestResult {
  clientId: string;
  supported: boolean;
  launched: boolean;
  success: boolean;
  summary: string;
  stdoutTail: string;
  stderrTail: string;
}

export default function TrayApp() {
  const [dashboard, setDashboard] = useState<DashboardState>(mockDashboard);
  const [savingsAttributionEvents, setSavingsAttributionEvents] = useState<
    SavingsAttributionEvent[]
  >([]);
  const [addonBusyId, setAddonBusyId] = useState<string | null>(null);
  const [addonBusyLabel, setAddonBusyLabel] = useState<string | null>(null);
  const [addonInfoId, setAddonInfoId] = useState<string | null>(null);
  const [addonResult, setAddonResult] = useState<{
    id: string;
    message: string;
  } | null>(null);
  const [addonError, setAddonError] = useState<string | null>(null);
  const [bootstrapping, setBootstrapping] = useState(false);
  const [bootstrapProgress, setBootstrapProgress] = useState<BootstrapProgress>(
    idleBootstrapProgress,
  );
  const [runtimeUpgradeProgress, setRuntimeUpgradeProgress] =
    useState<RuntimeUpgradeProgress>(idleRuntimeUpgradeProgress);
  const [bootstrapError, setBootstrapError] = useState<string | null>(null);
  const [windowLabel, setWindowLabel] = useState<"main" | "launcher" | null>(
    null,
  );
  const [startupPhase, setStartupPhase] = useState<StartupPhase>("window");
  const [startupPercent, setStartupPercent] = useState(10);
  const [startupCopy, setStartupCopy] = useState("Opening launch window…");
  const [startupReady, setStartupReady] = useState(false);
  const [activeView, setActiveView] = useState<TrayView>("home");
  const [settingsFocusTarget, setSettingsFocusTarget] = useState<string | null>(
    null,
  );
  const [semanticCacheEnabled, setSemanticCacheEnabled] = useState(false);
  // Launcher stage is a single source of truth for which onboarding screen
  // is showing. Only one screen can be active at a time; transitions go
  // through `setLauncherStage` so implicit renders from bootstrap/dashboard
  // flags cannot bypass the install step's readiness gate.
  const [launcherStage, setLauncherStage] = useState<LauncherStage>("install");
  const [connectors, setConnectors] = useState<ClientConnectorStatus[]>([]);
  const [openConnectorHelpId, setOpenConnectorHelpId] = useState<string | null>(
    null,
  );
  const [openConnectorWarningId, setOpenConnectorWarningId] = useState<
    string | null
  >(null);
  const [plannedConnectorCopyNotice, setPlannedConnectorCopyNotice] = useState<
    string | null
  >(null);
  const [releaseReadinessCopyNotice, setReleaseReadinessCopyNotice] = useState<
    string | null
  >(null);
  const [releaseReadinessReport, setReleaseReadinessReport] =
    useState<ReleaseReadinessReportPayload | null>(null);
  const [releaseReadinessRefreshing, setReleaseReadinessRefreshing] =
    useState(false);
  const [releaseReadinessError, setReleaseReadinessError] = useState<
    string | null
  >(null);
  const [releaseEvidenceBusyId, setReleaseEvidenceBusyId] = useState<
    string | null
  >(null);
  const [releaseEvidenceResult, setReleaseEvidenceResult] =
    useState<ReleaseEvidenceCommandResult | null>(null);
  const [settingsTransferNotice, setSettingsTransferNotice] = useState<
    string | null
  >(null);
  const [settingsImportText, setSettingsImportText] = useState("");
  const [settingsImportPreview, setSettingsImportPreview] =
    useState<SettingsImportPreview | null>(null);
  const [settingsImportBusy, setSettingsImportBusy] = useState(false);
  const releaseReadinessRows = releaseReadinessRowsFromReport(
    releaseReadinessReport?.report,
  );
  const releaseReadinessCounts =
    releaseReadinessStatusCounts(releaseReadinessRows);
  const releaseReadinessEvidence = releaseReadinessEvidenceSummary(
    releaseReadinessRows,
    releaseReadinessReport?.report,
  );
  const releaseLocalEvidenceRows = releaseLocalEvidenceRowsFromReport(
    releaseReadinessReport?.report,
  );
  const releaseReadinessAction = releaseReadinessNextAction(releaseReadinessRows);
  const [connectorsBusy, setConnectorsBusy] = useState(false);
  const [connectorPhase, setConnectorPhase] = useState<
    "disabled" | "verifying" | "healthy"
  >("healthy");
  const [connectorsError, setConnectorsError] = useState<string | null>(null);
  const [codexNudgeDismissed, setCodexNudgeDismissed] = useState(() => {
    try {
      return (
        window.localStorage.getItem("headroom:codexNudgeDismissed") === "1"
      );
    } catch {
      return false;
    }
  });
  const [proxyVerificationRows, setProxyVerificationRows] = useState<
    ProxyVerificationRow[]
  >([]);
  const [proxyVerificationHint, setProxyVerificationHint] = useState<
    string | null
  >(null);
  const [connectorSmokeBusyId, setConnectorSmokeBusyId] = useState<
    string | null
  >(null);
  const proxyVerificationRequestAnchorRef = useRef<Record<
    string,
    number
  > | null>(null);
  const proxyVerificationSessionRef = useRef(0);
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus | null>(
    null,
  );
  const [resuming, setResuming] = useState(false);
  const [resumeError, setResumeError] = useState<string | null>(null);
  const [appUpdateConfig, setAppUpdateConfig] =
    useState<AppUpdateConfiguration | null>(null);
  const [appUpdateAvailable, setAppUpdateAvailable] =
    useState<AvailableAppUpdate | null>(null);
  const [appUpdateBusy, setAppUpdateBusy] = useState(false);
  const [appUpdateInstallBusy, setAppUpdateInstallBusy] = useState(false);
  const [appUpdateReadyToRestart, setAppUpdateReadyToRestart] = useState(false);
  const [showAppUpdateDialog, setShowAppUpdateDialog] = useState(false);
  const [appUpdateStatusCopy, setAppUpdateStatusCopy] = useState<string | null>(
    null,
  );
  const [showHeadroomDetails, setShowHeadroomDetails] = useState(false);
  const [headroomLogLines, setHeadroomLogLines] = useState<string[]>([]);
  const headroomLogRef = useRef<HTMLPreElement | null>(null);
  const [claudeProjects, setClaudeProjects] = useState<ClaudeCodeProject[]>([]);
  const [claudeProjectsBusy, setClaudeProjectsBusy] = useState(false);
  const [claudeProjectsError, setClaudeProjectsError] = useState<string | null>(
    null,
  );
  const [activityFeed, setActivityFeed] = useState<ActivityFeedResponse>({
    tiles: {
      transformation: null,
      record: null,
      rtkToday: null,
      learningsMilestone: null,
      weeklyRecap: null,
      trainSuggestion: null,
    },
    proxyReachable: false,
  });
  // Flipped true after the first activity feed fetch attempt resolves (success
  // OR failure). Before this the feed holds a placeholder value whose
  // `proxyReachable: false` would falsely render the "proxy unreachable"
  // empty state and make the tab feel like it's already in an error state.
  const [activityFeedLoaded, setActivityFeedLoaded] = useState(false);
  // Tray window focus proxies for visibility: the window auto-hides on blur
  // via `triggerHide`, so "not focused" ⇒ "hidden" for polling purposes.
  const [trayWindowFocused, setTrayWindowFocused] = useState(true);
  // Sticky flag: the user has visited a heavy-data tab (Activity or Optimize)
  // at least once this session. The tray-focus pre-warm is gated on this so
  // users who stay on Home don't pay its IPC/subprocess cost on every focus.
  const [heavyTabEverOpened, setHeavyTabEverOpened] = useState(false);
  const [activityFeedError, setActivityFeedError] = useState<string | null>(
    null,
  );
  const {
    claudeLearnEnabled,
    codexLearnEnabled,
    copyLearnInstallCommand,
    handleRunHeadroomLearn,
    headroomLearnBusy,
    headroomLearnDisabledReason,
    headroomLearnPrereq,
    headroomLearnStatus,
    headroomLearnSupported,
    learnBlurb,
    learnInstallCopyNotice,
    openLearnInstallDocsLink,
    optimizeAppliedByProject,
    refreshHeadroomLearnPrereq,
    setOptimizeAppliedRefreshTick,
    setShowAllClaudeProjects,
    showAllClaudeProjects,
    sortedClaudeProjects,
    visibleClaudeProjects,
  } = useHeadroomLearnController({
    activeView,
    claudeProjects,
    connectors,
    openExternalLink,
    refreshClaudeProjects,
    runtimeStatus,
    setClaudeProjects,
    trayWindowFocused,
  });
  const autoDisabledByGateRef = useRef<Set<string>>(new Set());
  const [stepSignature, setStepSignature] = useState("");
  const [stepStartedAtMs, setStepStartedAtMs] = useState<number | null>(null);
  const [stepEtaSeedSeconds, setStepEtaSeedSeconds] = useState(0);
  const [stepBasePercent, setStepBasePercent] = useState(0);
  const [chartResetSignal, setChartResetSignal] = useState(0);
  const [chartMode, setChartMode] = useState<SavingsChartMode>("usd");
  const [savingsCalculatorScope, setSavingsCalculatorScope] =
    useState<SavingsCalculatorScope>("session");
  const [latestRepoIntelligenceSummary, setLatestRepoIntelligenceSummary] =
    useState<RepoIntelligenceSummary>(repoIntelligencePreview);
  // Safety net: if native history never loads (backend unreachable), reveal the
  // chart anyway after this delay rather than spinning forever.
  const [historyLoadTimedOut, setHistoryLoadTimedOut] = useState(false);
  const [showSavingsInfo, setShowSavingsInfo] = useState(false);
  const savingsCalculatorRepoEstimate = estimateRepoIntelligenceSavings(
    latestRepoIntelligenceSummary,
  );
  const cavemanTool =
    dashboard.tools.find((tool) => tool.id === "caveman") ?? null;
  const cavemanToolEnabled = cavemanTool?.enabled ?? false;
  const cavemanSavingsEstimate = cavemanToolEnabled
    ? buildAddonSavingsEstimate(
        CAVEMAN_TEMPLATE_BASELINE_TOKENS,
        CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS,
      )
    : null;
  const ponytailToolEnabled =
    dashboard.tools.find((tool) => tool.id === "ponytail")?.enabled ?? false;
  const ponytailSavingsEstimate = ponytailToolEnabled
    ? buildAddonSavingsEstimate(
        PONYTAIL_TEMPLATE_BASELINE_TOKENS,
        PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS,
      )
    : null;
  const markitdownToolEnabled =
    dashboard.tools.find((tool) => tool.id === "markitdown")?.enabled ?? false;
  const markitdownSavingsEstimate = markitdownToolEnabled
    ? buildAddonSavingsEstimate(
        MARKITDOWN_TEMPLATE_BASELINE_TOKENS,
        MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS,
      )
    : null;
  const [autostartEnabled, setAutostartEnabled] = useState<boolean | null>(
    null,
  );
  const [autostartBusy, setAutostartBusy] = useState(false);
  const [rtkBusy, setRtkBusy] = useState(false);
  const [showUninstallDialog, setShowUninstallDialog] = useState(false);
  const [uninstallBusy, setUninstallBusy] = useState(false);
  const [uninstallError, setUninstallError] = useState<string | null>(null);
  const [uninstallCopyNotice, setUninstallCopyNotice] = useState<string | null>(
    null,
  );
  const [contactEmail, setContactEmail] = useState("");
  const [contactMessage, setContactMessage] = useState("");
  const [contactSubmitBusy, setContactSubmitBusy] = useState(false);
  const [contactSubmitError, setContactSubmitError] = useState<string | null>(
    null,
  );
  const [contactSubmitSuccess, setContactSubmitSuccess] = useState<
    string | null
  >(null);
  const [switchboardState, setSwitchboardState] =
    useState<SwitchboardState | null>(null);
  const [switchboardModeBusy, setSwitchboardModeBusy] =
    useState<SwitchboardMode | null>(null);
  const [savingsModeBusy, setSavingsModeBusy] = useState<SavingsMode | null>(
    null,
  );
  const [switchboardModeError, setSwitchboardModeError] = useState<
    string | null
  >(null);
  const [doctorReport, setDoctorReport] = useState<DoctorReport | null>(null);
  const [managedFootprintReport, setManagedFootprintReport] =
    useState<ManagedFootprintReport | null>(null);
  const [onboardingFootprintCopyNotice, setOnboardingFootprintCopyNotice] =
    useState<string | null>(null);
  const [doctorRepairBusy, setDoctorRepairBusy] = useState<string | null>(null);
  const [doctorRepairError, setDoctorRepairError] = useState<string | null>(
    null,
  );
  const [doctorRepairSuccess, setDoctorRepairSuccess] = useState<string | null>(
    null,
  );
  const localOnlyMode = localOnlyModeEnabled();

  function openSettingsFocus(targetId: string) {
    setSettingsFocusTarget(targetId);
    setActiveView("settings");
  }

  const {
    masterActivationState,
    masterFeatureStates,
    masterActivationProgress,
    masterActivationReceipt,
    masterOperation,
    maxCompressionBusy,
    masterActivationIsActive,
    activateEverything,
    deactivateEverything,
    activateMasterFeature,
    deactivateMasterFeature,
    activateMaxCompression,
    openCompressionPlaybook,
    masterFeatureView,
  } = useMasterActivationController({
    switchboardState,
    connectors,
    runtimeStatus,
    semanticCacheEnabled,
    setSemanticCacheEnabled,
    setActiveView,
    openSettingsFocus,
    handleSetSwitchboardMode,
    applyRuntimeStatusIfChanged,
    refreshRuntimeStatus,
    refreshConnectors,
    refreshDoctorReport,
    prepareRepoMemoryMcp,
    setRepoMemoryMcpActive,
  });
  const {
    pricingStatus,
    setPricingStatus,
    cachedPricing,
    pricingBusy,
    pricingError,
    authEmail,
    setAuthEmail,
    authCode,
    setAuthCode,
    authCodeRequestedFor,
    authCodeExpirySeconds,
    authRequestBusy,
    authVerifyBusy,
    authFlowError,
    authFlowSuccess,
    authEmailValid,
    pendingUpgradePlanId,
    showAllUpgradePlans,
    setShowAllUpgradePlans,
    checkoutPollingDeadline,
    pricingAudience,
    setPricingAudience,
    billingPeriod,
    setBillingPeriod,
    upgradeActionBusy,
    upgradeActionError,
    pendingPlanChange,
    planChangeBusy,
    planChangeError,
    reactivateBusy,
    reactivateError,
    setAuthFlowError,
    setUpgradeActionError,
    refreshPricingStatus,
    openUpgradeAuthView,
    resetUpgradeAuthStep,
    handleRequestAuthCode,
    handleVerifyAuthCode,
    handleSignOutHeadroomAccount,
    handleUpgradeAction,
    confirmPlanChange,
    cancelPlanChange,
    handleReactivateSubscription,
  } = useTrayPricingController({
    trayWindowFocused,
    runtimeStatus,
    connectorPhase,
    setActiveView,
    refreshConnectors,
    openExternalLink,
  });
  const appSemver = "0.0.0";
  const savingsDashboard = dashboard.savingsHistoryLoaded
    ? dashboard
    : {
        ...dashboard,
        lifetimeRequests: 0,
        lifetimeEstimatedSavingsUsd: 0,
        lifetimeEstimatedTokensSaved: 0,
        dailySavings: [],
        hourlySavings: [],
      };
  const bootstrapFailureSignatureRef = useRef("");
  const mainWindowLastBlurAtRef = useRef<number | null>(null);
  const mainWindowLastSeenDayRef = useRef(formatDayKey(new Date()));
  const appUpdateKnownVersionRef = useRef<string | null>(null);
  const appUpdateReadyToRestartRef = useRef(false);
  const appUpdateBusyRef = useRef(false);
  const appUpdateInstallBusyRef = useRef(false);
  const launcherHideAnimationMs = 320;
  const trayFocusPrewarmDelayMs = 250;
  const dashboardSignatureRef = useRef(serializeState(mockDashboard));
  const connectorsSignatureRef = useRef(
    serializeState([] as ClientConnectorStatus[]),
  );
  const connectorsRefreshGenerationRef = useRef(0);
  const runtimeStatusSignatureRef = useRef(
    serializeState(null as RuntimeStatus | null),
  );
  const switchboardSignatureRef = useRef(
    serializeState(null as SwitchboardState | null),
  );
  const claudeProjectsSignatureRef = useRef(
    serializeState([] as ClaudeCodeProject[]),
  );
  const upgradePlansState = getUpgradePlans(
    pricingAudience,
    pricingStatus?.claude.planTier ?? cachedPricing.planTier,
    pricingStatus?.recommendedSubscriptionTier ??
      cachedPricing.recommendedSubscriptionTier,
    pricingStatus?.account?.subscriptionTier ?? cachedPricing.subscriptionTier,
    pricingStatus?.account?.subscriptionActive ?? false,
    pricingStatus?.launchDiscountActive ?? false,
    billingPeriod,
    pricingStatus?.account?.subscriptionAmountCents,
    pricingStatus?.account?.subscriptionBillingPeriod,
    pricingStatus?.account?.subscriptionRenewsAt,
    pricingStatus?.account?.subscriptionStartedAt,
    pricingStatus?.account?.subscriptionDiscountDuration,
    pricingStatus?.account?.subscriptionDiscountDurationInMonths,
    pricingStatus?.account?.subscriptionCancelAtPeriodEnd ?? false,
    pricingStatus?.account?.subscriptionEndsAt,
    pricingStatus?.activePercentOff ?? 0,
  );
  const contactEmailValid = isValidEmailAddress(contactEmail);
  const showInstallProgress =
    bootstrapping ||
    bootstrapProgress.running ||
    bootstrapProgress.complete ||
    bootstrapProgress.failed ||
    bootstrapProgress.overallPercent > 0;

  const isLastScreen =
    windowLabel === "launcher" && launcherStage === "post_install";
  useEffect(() => {
    if (!showHeadroomDetails || !headroomLogRef.current) {
      return;
    }
    headroomLogRef.current.scrollTop = headroomLogRef.current.scrollHeight;
  }, [showHeadroomDetails, headroomLogLines]);

  useEffect(() => {
    const timer = window.setTimeout(() => setHistoryLoadTimedOut(true), 20000);
    return () => window.clearTimeout(timer);
  }, []);

  useEffect(() => {
    void invoke<ReleaseReadinessReportPayload>("load_release_readiness_report")
      .then(setReleaseReadinessReport)
      .catch(() => setReleaseReadinessReport(null));
  }, []);

  useEffect(() => {
    dashboardSignatureRef.current = serializeState(dashboard);
  }, [dashboard]);

  useEffect(() => {
    connectorsSignatureRef.current = serializeState(connectors);
  }, [connectors]);

  useEffect(() => {
    runtimeStatusSignatureRef.current = serializeState(runtimeStatus);
  }, [runtimeStatus]);

  useEffect(() => {
    switchboardSignatureRef.current = serializeState(switchboardState);
  }, [switchboardState]);

  useEffect(() => {
    claudeProjectsSignatureRef.current = serializeState(claudeProjects);
  }, [claudeProjects]);

  function applyDashboardIfChanged(next: DashboardState) {
    const nextSignature = serializeState(next);
    if (dashboardSignatureRef.current === nextSignature) {
      return;
    }
    dashboardSignatureRef.current = nextSignature;
    setDashboard(next);
  }

  async function refreshSavingsAttributionEvents() {
    const events = await loadSavingsAttributionEvents();
    setSavingsAttributionEvents((current) =>
      serializeState(current) === serializeState(events) ? current : events,
    );
  }

  function applyConnectorsIfChanged(next: ClientConnectorStatus[]) {
    const nextSignature = serializeState(next);
    if (connectorsSignatureRef.current === nextSignature) {
      return;
    }
    connectorsSignatureRef.current = nextSignature;
    setConnectors(next);
  }

  function applyRuntimeStatusIfChanged(next: RuntimeStatus | null) {
    const nextSignature = serializeState(next);
    if (runtimeStatusSignatureRef.current === nextSignature) {
      return;
    }
    runtimeStatusSignatureRef.current = nextSignature;
    setRuntimeStatus(next);
  }

  function applySwitchboardStateIfChanged(next: SwitchboardState | null) {
    const nextSignature = serializeState(next);
    if (switchboardSignatureRef.current === nextSignature) {
      return;
    }
    switchboardSignatureRef.current = nextSignature;
    setSwitchboardState(next);
  }

  function applyClaudeProjectsIfChanged(next: ClaudeCodeProject[]) {
    const nextSignature = serializeState(next);
    if (claudeProjectsSignatureRef.current === nextSignature) {
      return;
    }
    claudeProjectsSignatureRef.current = nextSignature;
    setClaudeProjects(next);
  }

  useEffect(() => {
    if (!hasTauriEventRuntime()) {
      return;
    }

    const unlistenPromise = listen<{ action: string | null }>(
      "notification-clicked",
      (event) => {
        const action = event.payload?.action ?? null;
        if (action === "update") {
          setShowAppUpdateDialog(true);
          return;
        }
        const view = safeNotificationActionView(action, localOnlyMode);
        if (view) {
          setActiveView(view);
          const targetId = notificationActionTargetId(action);
          if (targetId) {
            window.setTimeout(() => {
              document
                .getElementById(targetId)
                ?.scrollIntoView({ block: "start", behavior: "smooth" });
            }, 0);
          }
        }
      },
    );
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [localOnlyMode]);

  useEffect(() => {
    if (
      localOnlyMode &&
      (activeView === "upgrade" || activeView === "upgradeAuth")
    ) {
      setActiveView("home");
    }
  }, [activeView, localOnlyMode]);

  useEffect(() => {
    if (activeView !== "settings" || !settingsFocusTarget) {
      return;
    }
    const timeout = window.setTimeout(() => {
      document.getElementById(settingsFocusTarget)?.scrollIntoView({
        block: "start",
        behavior: "smooth",
      });
      setSettingsFocusTarget(null);
    }, 0);
    return () => window.clearTimeout(timeout);
  }, [activeView, settingsFocusTarget]);

  useEffect(() => {
    const claudeConnector = getClaudeConnector(connectors);
    if (!claudeConnector?.installed) {
      return;
    }
    trackInstallMilestoneOnce("claude_code_detected", {
      enabled: claudeConnector.enabled,
      verified: claudeConnector.verified,
    });
  }, [connectors]);

  useEffect(() => {
    const claudeConnector = getClaudeConnector(connectors);
    if (!claudeConnector?.enabled) {
      return;
    }
    trackInstallMilestoneOnce("optimization_enabled", {
      verified: claudeConnector.verified,
    });
  }, [connectors]);

  useEffect(() => {
    if (dashboard.lifetimeRequests <= 0) {
      return;
    }
    trackInstallMilestoneOnce("first_optimized_request", {
      lifetime_requests: dashboard.lifetimeRequests,
      launch_experience: dashboard.launchExperience,
    });
  }, [dashboard.launchExperience, dashboard.lifetimeRequests]);

  useEffect(() => {
    if (
      dashboard.lifetimeEstimatedTokensSaved <= 0 &&
      dashboard.lifetimeEstimatedSavingsUsd <= 0
    ) {
      return;
    }
    trackInstallMilestoneOnce("first_savings_recorded", {
      lifetime_tokens_saved: dashboard.lifetimeEstimatedTokensSaved,
      lifetime_savings_usd: Number(
        dashboard.lifetimeEstimatedSavingsUsd.toFixed(4),
      ),
    });
  }, [
    dashboard.lifetimeEstimatedSavingsUsd,
    dashboard.lifetimeEstimatedTokensSaved,
  ]);

  useEffect(() => {
    let active = true;

    const runStartupChecks = async () => {
      const updateStartup = (
        phase: StartupPhase,
        percent: number,
        message: string,
      ) => {
        if (!active) {
          return;
        }
        setStartupPhase(phase);
        setStartupPercent((current) => Math.max(current, percent));
        setStartupCopy(message);
      };

      updateStartup("window", 12, "Opening launch window…");
      const label = hasTauriRuntime() ? getCurrentWindow().label : "main";
      if (active) {
        if (label === "main" || label === "launcher") {
          setWindowLabel(label);
        } else {
          setWindowLabel("main");
        }
      }

      updateStartup("dashboard", 35, "Loading local dashboard state…");
      const dashboardResult = await loadDashboard();
      if (!active) {
        return;
      }
      applyDashboardIfChanged(dashboardResult);
      void refreshSavingsAttributionEvents();

      updateStartup("bootstrap", 58, "Checking runtime install state…");
      const bootstrapResult = await invoke<BootstrapProgress>(
        "get_bootstrap_progress",
      ).catch(() => idleBootstrapProgress);
      if (!active) {
        return;
      }
      setBootstrapProgress(bootstrapResult);
      if (bootstrapResult.running) {
        setBootstrapping(true);
      }
      const initialStage = getInitialLauncherStage(
        label,
        bootstrapResult.complete,
        dashboardResult.bootstrapComplete,
        dashboardResult.launchExperience,
      );
      if (initialStage) {
        setLauncherStage(initialStage);
      }

      updateStartup("runtime", 80, "Preparing local engine…");
      const [
        runtimeResult,
        switchboardResult,
        doctorResult,
        footprintResult,
        pricingResult,
      ] =
        await Promise.all([
          invoke<RuntimeStatus>("get_runtime_status").catch(() => null),
          invoke<SwitchboardState>("get_switchboard_state").catch(() => null),
          invoke<DoctorReport>("get_doctor_report").catch(() => null),
          invoke<ManagedFootprintReport>("get_managed_footprint").catch(
            () => null,
          ),
          localOnlyMode
            ? Promise.resolve(null)
            : invoke<HeadroomPricingStatus>(
                "get_headroom_pricing_status",
              ).catch(() => null),
          refreshConnectors(),
        ]);
      if (!active) {
        return;
      }
      if (runtimeResult) {
        applyRuntimeStatusIfChanged(runtimeResult);
      }
      if (switchboardResult) {
        applySwitchboardStateIfChanged(switchboardResult);
      }
      if (doctorResult) {
        setDoctorReport(doctorResult);
      }
      if (footprintResult) {
        setManagedFootprintReport(footprintResult);
      }
      if (pricingResult) {
        setPricingStatus(pricingResult);
      }

      updateStartup(
        "ready",
        95,
        label === "launcher"
          ? "Preparing launch checklist…"
          : "Preparing tray dashboard…",
      );
      window.setTimeout(() => {
        if (!active) {
          return;
        }
        setStartupPercent(100);
        setStartupCopy("AI Switchboard is ready.");
        setStartupReady(true);
      }, 120);
    };

    void runStartupChecks();

    return () => {
      active = false;
    };
  }, [localOnlyMode]);

  useEffect(() => {
    if (startupReady) {
      return;
    }

    const phaseCaps: Record<StartupPhase, number> = {
      window: 28,
      dashboard: 54,
      bootstrap: 76,
      runtime: 92,
      ready: 99,
    };
    const cap = phaseCaps[startupPhase];

    const interval = window.setInterval(() => {
      setStartupPercent((current) => {
        if (current >= cap) {
          return current;
        }
        return Math.min(cap, current + (current < 20 ? 2 : 1));
      });
    }, 260);

    return () => {
      window.clearInterval(interval);
    };
  }, [startupPhase, startupReady]);

  useEffect(() => {
    if (!bootstrapping) {
      return;
    }

    let active = true;
    let completionHandled = false;
    let unlisten: (() => void) | undefined;
    const detach = () => {
      const fn = unlisten;
      unlisten = undefined;
      fn?.();
    };

    const handleProgress = async (progress: BootstrapProgress) => {
      if (!active) {
        return;
      }

      setBootstrapProgress(progress);

      if (progress.failed) {
        const failureReport = buildBootstrapFailureReport(progress);
        const failureSignature = bootstrapFailureSignature(failureReport);
        if (bootstrapFailureSignatureRef.current !== failureSignature) {
          bootstrapFailureSignatureRef.current = failureSignature;
          reportBootstrapFailure(failureReport);
        }
        setBootstrapError(progress.message);
        setBootstrapping(false);
        completionHandled = true;
        detach();
        return;
      }

      if (progress.complete && !completionHandled) {
        completionHandled = true;
        detach();
        setBootstrapping(false);
        const latestDashboard = await loadDashboard();
        if (!active) {
          return;
        }
        applyDashboardIfChanged(latestDashboard);
        void refreshSavingsAttributionEvents();
        // Always land on the install step after a bootstrap completes during
        // this session, regardless of launchExperience. The install step's
        // Continue button is gated on runtime.running, so it handles both the
        // readiness wait and the "Local switchboard runtime is ready" confirmation
        // for Resume users whose launch_count > 1 (e.g., they reinstalled the
        // app without clearing ~/Library/Application Support/Headroom).
        if (windowLabel === "launcher") {
          setLauncherStage("install");
        }
      }
    };

    if (!hasTauriEventRuntime()) {
      return;
    }
    void listen<BootstrapProgress>("bootstrap_progress", (event) => {
      void handleProgress(event.payload);
    }).then((fn) => {
      if (!active || completionHandled) {
        fn();
        return;
      }
      unlisten = fn;
    });

    // Prime with the current state in case we subscribed mid-flight or the
    // bootstrap already completed before the listener attached.
    void invoke<BootstrapProgress>("get_bootstrap_progress")
      .then((progress) => handleProgress(progress))
      .catch(() => {});

    return () => {
      active = false;
      detach();
    };
  }, [bootstrapping]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    if (!hasTauriEventRuntime()) {
      return;
    }
    void listen<RuntimeUpgradeProgress>("runtime_upgrade_progress", (event) => {
      if (!active) return;
      setRuntimeUpgradeProgress(event.payload);
    }).then((fn) => {
      if (!active) {
        fn();
        return;
      }
      unlisten = fn;
    });

    void invoke<RuntimeUpgradeProgress>("get_runtime_upgrade_progress")
      .then((progress) => {
        if (active) setRuntimeUpgradeProgress(progress);
      })
      .catch(() => {});

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  // Hand off cleanly once the runtime upgrade finishes: show the success
  // state briefly, then drop the progress object back to idle so the
  // launcher stops rendering the upgrade UI and falls through to whichever
  // window content the user should see next. We also nudge the launcher
  // stage to post_install since bootstrapComplete only gets checked at
  // startup otherwise.
  useEffect(() => {
    if (!runtimeUpgradeProgress.complete || runtimeUpgradeProgress.failed) {
      return;
    }
    const timeout = window.setTimeout(() => {
      setRuntimeUpgradeProgress(idleRuntimeUpgradeProgress);
      if (windowLabel === "launcher") {
        setLauncherStage("post_install");
      }
      // Refresh runtime status so the rest of the app picks up the
      // freshly-installed version immediately.
      void invoke<RuntimeStatus>("get_runtime_status")
        .then((status) => applyRuntimeStatusIfChanged(status))
        .catch(() => {});
    }, 2500);
    return () => window.clearTimeout(timeout);
  }, [
    runtimeUpgradeProgress.complete,
    runtimeUpgradeProgress.failed,
    windowLabel,
  ]);

  useEffect(() => {
    if (windowLabel !== "launcher" || launcherStage !== "client_setup") {
      return;
    }
    void refreshConnectors();
  }, [windowLabel, launcherStage]);

  useEffect(() => {
    if (windowLabel !== "launcher" || launcherStage !== "proxy_verify") {
      return;
    }

    let active = true;
    const interval = window.setInterval(() => {
      void (async () => {
        try {
          const [runtime, counts] = await Promise.all([
            invoke<RuntimeStatus>("get_runtime_status"),
            invoke<Record<string, number> | null>(
              "get_headroom_request_counts_by_agent",
            ).catch(() => null),
          ]);

          if (!active) {
            return;
          }

          if (!runtime.proxyReachable || counts === null) {
            setProxyVerificationHint(
              "Headroom proxy is not reachable yet. Start Headroom runtime, then send a test message.",
            );
            return;
          }

          setProxyVerificationHint(null);

          // Capture the baseline on the first reachable poll. Anchoring on a
          // null/unreachable reading would let a later "proxy came up" jump
          // (0 → N) look like new traffic.
          if (proxyVerificationRequestAnchorRef.current === null) {
            proxyVerificationRequestAnchorRef.current = counts;
            return;
          }

          // Attribute traffic per client: a prompt sent to Claude Code must not
          // flip the Codex row (and vice versa). The proxy keys agents as
          // `claude-code` / `codex`; our rows use `claude_code` / `codex`.
          const anchor = proxyVerificationRequestAnchorRef.current;
          setProxyVerificationRows((current) =>
            current.map((row) => {
              if (row.state === "verified") {
                return row;
              }
              // One-click rows are verified only after the connector smoke
              // command returned the exact expected response. A request
              // counter tick alone can be unrelated traffic or a mismatched
              // model response, so it must not turn a failed smoke attempt
              // green.
              if (row.oneClickSupported && row.state !== "processing") {
                return row;
              }
              const agentKey = row.clientId.replace(/_/g, "-");
              const now = counts[agentKey] ?? 0;
              const base = anchor[agentKey] ?? 0;
              return now > base
                ? { ...row, state: "verified", message: "Request received" }
                : row;
            }),
          );
        } catch {
          if (active) {
            setProxyVerificationHint("Waiting for Headroom proxy activity...");
          }
        }
      })();
    }, 1000);

    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [windowLabel, launcherStage]);

  useEffect(() => {
    if (!showInstallProgress) {
      return;
    }

    const signature = `${bootstrapProgress.currentStep}|${bootstrapProgress.running}|${bootstrapProgress.complete}|${bootstrapProgress.failed}`;
    if (signature === stepSignature) {
      return;
    }

    setStepSignature(signature);
    setStepStartedAtMs(Date.now());
    setStepEtaSeedSeconds(bootstrapProgress.currentStepEtaSeconds);
    setStepBasePercent(bootstrapProgress.overallPercent);
  }, [bootstrapProgress, showInstallProgress, stepSignature]);

  useEffect(() => {
    if (!isLastScreen || !hasTauriRuntime()) return;
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (!focused) triggerHide();
      })
      .then((fn) => {
        unlisten = fn;
      });
    return () => unlisten?.();
  }, [isLastScreen]);

  useEffect(() => {
    if (windowLabel !== "main" || !trayWindowFocused) {
      return;
    }

    void refreshRuntimeStatus();
    const interval = window.setInterval(() => {
      void refreshRuntimeStatus();
    }, 3000);

    return () => window.clearInterval(interval);
  }, [windowLabel, trayWindowFocused]);

  // Poll runtime status while the install step is visible so the Continue
  // button unlocks as soon as headroom is fully running (same signal the
  // tray uses for its solid icon: installed && !paused && proxy_reachable).
  // On a cold first install the Gatekeeper scan can finish after
  // mark_bootstrap_complete fires, and the main-window poller doesn't run
  // on the launcher.
  useEffect(() => {
    if (windowLabel !== "launcher" || launcherStage !== "install") {
      return;
    }
    if (runtimeStatus?.running === true) {
      return;
    }

    void refreshRuntimeStatus();
    const interval = window.setInterval(() => {
      void refreshRuntimeStatus();
    }, 1000);

    return () => window.clearInterval(interval);
  }, [windowLabel, launcherStage, runtimeStatus?.running]);

  useEffect(() => {
    if (windowLabel !== "main" || !hasTauriRuntime()) {
      return;
    }

    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        setTrayWindowFocused(focused);
        const now = new Date();
        const nowDayKey = formatDayKey(now);

        if (!focused) {
          mainWindowLastBlurAtRef.current = now.getTime();
          mainWindowLastSeenDayRef.current = nowDayKey;
          return;
        }

        const inactiveForMs = mainWindowLastBlurAtRef.current
          ? now.getTime() - mainWindowLastBlurAtRef.current
          : null;
        // Skip `refreshConnectors` for quick alt-tabs: connectors only change
        // via user action (app enable/disable) or manual edits to
        // ~/.claude/settings.json — neither happens in the 30s window of a
        // fast context switch. On initial focus (`inactiveForMs === null`)
        // or after a real "came back from another app" gap, refresh to pick
        // up outside changes.
        if (inactiveForMs === null || inactiveForMs >= 30_000) {
          void refreshConnectors();
        }

        const dayRolledOver = nowDayKey !== mainWindowLastSeenDayRef.current;
        if ((inactiveForMs ?? 0) >= 3_600_000 || dayRolledOver) {
          setChartResetSignal((current) => current + 1);
        }

        mainWindowLastBlurAtRef.current = null;
        mainWindowLastSeenDayRef.current = nowDayKey;
      })
      .then((fn) => {
        unlisten = fn;
      });

    return () => unlisten?.();
  }, [windowLabel]);

  useEffect(() => {
    if (!startupReady) {
      return;
    }
    void refreshAppUpdateConfiguration();
  }, [startupReady]);

  useEffect(() => {
    if (!startupReady || windowLabel !== "main" || !appUpdateConfig) {
      return;
    }
    if (!appUpdateConfig.enabled || appUpdateConfig.configurationError) {
      return;
    }

    const runBackgroundCheck = () => {
      if (
        appUpdateReadyToRestartRef.current ||
        appUpdateBusyRef.current ||
        appUpdateInstallBusyRef.current
      ) {
        return;
      }
      void checkForAppUpdate({
        background: true,
        knownUpdateVersion: appUpdateKnownVersionRef.current,
      });
    };

    const timer = window.setTimeout(
      runBackgroundCheck,
      APP_UPDATE_BACKGROUND_INITIAL_DELAY_MS,
    );
    const interval = window.setInterval(
      runBackgroundCheck,
      APP_UPDATE_BACKGROUND_CHECK_INTERVAL_MS,
    );

    return () => {
      window.clearTimeout(timer);
      window.clearInterval(interval);
    };
  }, [appUpdateConfig, startupReady, windowLabel]);

  useEffect(() => {
    if (windowLabel !== "main" || !trayWindowFocused) {
      return;
    }
    void refreshSwitchboardState();
    void refreshDoctorReport();
    const interval = window.setInterval(() => {
      void refreshSwitchboardState();
      void refreshDoctorReport();
    }, 5_000);
    return () => window.clearInterval(interval);
  }, [trayWindowFocused, windowLabel]);

  useEffect(() => {
    appUpdateKnownVersionRef.current = appUpdateAvailable?.version ?? null;
  }, [appUpdateAvailable?.version]);

  useEffect(() => {
    appUpdateReadyToRestartRef.current = appUpdateReadyToRestart;
  }, [appUpdateReadyToRestart]);

  useEffect(() => {
    appUpdateBusyRef.current = appUpdateBusy;
  }, [appUpdateBusy]);

  useEffect(() => {
    appUpdateInstallBusyRef.current = appUpdateInstallBusy;
  }, [appUpdateInstallBusy]);

  useEffect(() => {
    if (activeView !== "settings") {
      return;
    }
    void Promise.all([
      refreshConnectors(),
      refreshRuntimeStatus(),
      appUpdateConfig ? Promise.resolve() : refreshAppUpdateConfiguration(),
    ]);
    void invoke<boolean>("get_autostart_enabled")
      .then((enabled) => setAutostartEnabled(enabled))
      .catch(() => setAutostartEnabled(false));
  }, [activeView]);

  async function handleAutostartToggle(nextEnabled: boolean) {
    setAutostartBusy(true);
    try {
      const enabled = await invoke<boolean>("set_autostart_enabled", {
        enabled: nextEnabled,
      });
      setAutostartEnabled(enabled);
    } catch (error) {
      console.error("Failed to update autostart", error);
    } finally {
      setAutostartBusy(false);
    }
  }

  async function handleRtkToggle(nextEnabled: boolean) {
    const copy = addonCopy.rtk;
    setRtkBusy(true);
    setAddonBusyId("rtk");
    setAddonBusyLabel((nextEnabled ? copy?.enabling : copy?.disabling) ?? null);
    setAddonResult(null);
    try {
      await invoke<boolean>("set_rtk_enabled", { enabled: nextEnabled });
      await refreshSwitchboardState();
      const message = nextEnabled ? undefined : copy?.disabled;
      if (message) {
        setAddonResult({ id: "rtk", message });
      }
    } catch (error) {
      console.error("Failed to update RTK", error);
      setAddonError("RTK could not be updated.");
    } finally {
      setRtkBusy(false);
      setAddonBusyId(null);
      setAddonBusyLabel(null);
    }
  }

  async function handleUninstall() {
    setUninstallBusy(true);
    setUninstallError(null);
    try {
      await invoke<string[]>("uninstall_and_quit");
    } catch (error) {
      setUninstallError(
        typeof error === "string"
          ? error
          : "Uninstall failed. Please try again.",
      );
      setUninstallBusy(false);
    }
  }

  async function copyUninstallDryRunReport() {
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      let report = formatUninstallDryRunReport();
      try {
        const backendReport = await invoke<UninstallDryRunReport>(
          "get_uninstall_dry_run_report",
        );
        report = formatBackendUninstallDryRunReport(backendReport);
      } catch (error) {
        console.warn("Falling back to static uninstall dry-run report", error);
      }
      await navigator.clipboard.writeText(report);
      setUninstallCopyNotice("Uninstall dry-run copied.");
      window.setTimeout(() => setUninstallCopyNotice(null), 2500);
    } catch {
      setUninstallCopyNotice("Copy failed. Uninstall list remains visible.");
      window.setTimeout(() => setUninstallCopyNotice(null), 3000);
    }
  }

  useEffect(() => {
    if (
      (activeView !== "home" && activeView !== "usage") ||
      !trayWindowFocused
    ) {
      return;
    }

    let active = true;
    const refreshDashboard = () => {
      void loadDashboard()
        .then((next) => {
          if (!active) return;
          applyDashboardIfChanged(next);
          void refreshSavingsAttributionEvents();
        })
        .catch(() => {
          // keep last known state
        });
    };

    refreshDashboard();
    const interval = window.setInterval(refreshDashboard, 5000);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [activeView, trayWindowFocused]);

  // Track whether the user has ever visited a heavy-data tab this session.
  // Once true, stays true until app restart — the pre-warm below is gated
  // on it so Home-only users don't pay its cost on every tray focus.
  useEffect(() => {
    if (activeView === "notifications" || activeView === "optimization") {
      setHeavyTabEverOpened(true);
    }
  }, [activeView]);

  // Pre-warm Optimize + Activity data the moment the tray gains focus, so
  // switching tabs reveals already-populated content instead of triggering
  // a fresh ~500ms Python subprocess spawn and layout flash. The tab-scoped
  // effects below still run and keep data fresh — they just hit the Rust
  // cache now instead of spawning a cold Python process. Gated on
  // `heavyTabEverOpened` so users who only use Home never trigger it.
  useEffect(() => {
    if (
      windowLabel !== "main" ||
      !trayWindowFocused ||
      !heavyTabEverOpened ||
      activeView === "notifications" ||
      activeView === "optimization"
    ) {
      return;
    }

    let active = true;
    const timeout = window.setTimeout(() => {
      if (!active) {
        return;
      }
      void refreshClaudeProjects();
      void refreshHeadroomLearnPrereq();
      invoke<ActivityFeedResponse>("get_activity_feed")
        .then((next) => {
          if (!active) return;
          setActivityFeed((prev) =>
            activityFeedSignature(prev) === activityFeedSignature(next)
              ? prev
              : next,
          );
          setActivityFeedError(null);
        })
        .catch(() => {
          // Swallow: the tab-active poll will surface any real error once the
          // user opens Activity. Pre-warm failures shouldn't flash a banner.
        })
        .finally(() => {
          if (!active) return;
          setActivityFeedLoaded(true);
        });
    }, trayFocusPrewarmDelayMs);

    return () => {
      active = false;
      window.clearTimeout(timeout);
    };
  }, [windowLabel, trayWindowFocused, heavyTabEverOpened, activeView]);

  useEffect(() => {
    if (activeView !== "notifications" || !trayWindowFocused) {
      return;
    }
    let active = true;
    const refreshFeed = () => {
      invoke<ActivityFeedResponse>("get_activity_feed")
        .then((next) => {
          if (!active) return;
          setActivityFeed((prev) =>
            activityFeedSignature(prev) === activityFeedSignature(next)
              ? prev
              : next,
          );
          setActivityFeedError(null);
        })
        .catch((err) => {
          if (!active) return;
          setActivityFeedError(
            err instanceof Error
              ? err.message
              : "Could not load activity feed.",
          );
        })
        .finally(() => {
          if (!active) return;
          setActivityFeedLoaded(true);
        });
    };
    refreshFeed();
    const interval = window.setInterval(refreshFeed, 4000);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [activeView, trayWindowFocused]);

  useEffect(() => {
    if (activeView !== "home" || !startupReady) {
      return;
    }
    void Promise.all([refreshConnectors(), refreshRuntimeStatus()]);
  }, [activeView, startupReady]);

  useEffect(() => {
    if (activeView !== "upgrade") {
      setUpgradeActionError(null);
    }
  }, [activeView]);

  // Keep connectorPhase in sync with the connector enabled state from the backend.
  // Any supported connector (Claude Code, Codex, ...) being enabled counts as
  // "connected" — the request-count poller below is connector-agnostic.
  const anyConnectorEnabled = hasEnabledConnector(connectors);
  const plannedConnectorReadiness =
    summarizePlannedConnectorReadiness(connectors);

  useEffect(() => {
    setConnectorPhase((prev) => {
      if (!anyConnectorEnabled) return "disabled";
      // Any transition from "disabled" → enabled (re-enable click, externally
      // toggled, or fresh app launch) drops into verifying, so the polling
      // effect below confirms via /stats that traffic is actually flowing
      // before the badge flips green.
      if (prev === "disabled") return "verifying";
      return prev; // keep "verifying" or "healthy"
    });
  }, [anyConnectorEnabled]);

  // When the pricing gate closes, pause optimization on every enabled
  // connector (not just Claude Code) one at a time. Each disable refreshes
  // `connectors`, re-running this effect until none remain enabled.
  useEffect(() => {
    if (localOnlyMode) {
      return;
    }
    if (!pricingStatus || pricingStatus.optimizationAllowed || connectorsBusy) {
      return;
    }
    const target = getEnabledSupportedConnectors(connectors)[0];
    if (!target) {
      return;
    }
    autoDisabledByGateRef.current.add(target.clientId);
    void toggleConnector(target, false);
  }, [connectors, connectorsBusy, localOnlyMode, pricingStatus]);

  // Companion to the auto-disable effect above: when the pricing gate
  // releases (e.g., user just signed up post-grace, or weekly usage
  // rolled over), bring back every connector we auto-disabled without forcing
  // a manual re-enable click. Scoped to our own prior auto-disables so a
  // user's manual disable during an ungated period is preserved.
  useEffect(() => {
    if (localOnlyMode) {
      return;
    }
    if (
      !pricingStatus?.optimizationAllowed ||
      autoDisabledByGateRef.current.size === 0
    ) {
      return;
    }
    if (connectorsBusy) {
      return;
    }
    const target = aggregateClientConnectors(connectors).find(
      (connector) =>
        autoDisabledByGateRef.current.has(connector.clientId) &&
        !connector.enabled,
    );
    if (!target) {
      autoDisabledByGateRef.current.clear();
      return;
    }
    void toggleConnector(target, true);
  }, [connectors, connectorsBusy, localOnlyMode, pricingStatus]);

  // While verifying, poll the proxy's /stats request counter and flip to
  // healthy when it ticks past the anchor we captured on the first reachable
  // poll. The previous implementation scanned the python proxy log for
  // /v1/messages lines, but Claude Code traffic actually flows through the
  // Rust front proxy on 6767 — the python log only sees background activity,
  // so the regex match could hang forever even while requests were being
  // optimized normally.
  useEffect(() => {
    if (connectorPhase !== "verifying") return;
    let active = true;
    let anchor: number | null = null;
    const interval = setInterval(() => {
      void (async () => {
        const count = await invoke<number | null>(
          "get_headroom_request_count",
        ).catch(() => null);
        if (!active) return;
        // null = proxy unreachable. Don't anchor on transient
        // unreachable readings — a later reachable reading would otherwise
        // jump from 0 → N and flip the badge healthy without observing
        // any new traffic.
        if (count === null) return;
        if (anchor === null) {
          anchor = count;
          return;
        }
        if (count > anchor) {
          setConnectorPhase("healthy");
        }
      })();
    }, 1000);
    return () => {
      active = false;
      clearInterval(interval);
    };
  }, [connectorPhase]);

  useEffect(() => {
    if (!anyConnectorEnabled || connectorPhase !== "verifying") {
      return;
    }
    let active = true;
    void invoke<number | null>("get_headroom_request_count")
      .then((count) => {
        if (active && count !== null && count > 0) {
          setConnectorPhase("healthy");
        }
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [anyConnectorEnabled, connectorPhase]);

  async function handleBootstrap() {
    bootstrapFailureSignatureRef.current = "";
    setBootstrapError(null);
    setBootstrapProgress({
      running: true,
      complete: false,
      failed: false,
      currentStep: "Preparing install",
      message: "Initializing installer workflow.",
      currentStepEtaSeconds: 3,
      overallPercent: 2,
    });
    setBootstrapping(true);
    try {
      await invoke("start_bootstrap");
    } catch (error) {
      const failureReport = buildBootstrapInvokeFailureReport(error);
      const failureSignature = bootstrapFailureSignature(failureReport);
      if (bootstrapFailureSignatureRef.current !== failureSignature) {
        bootstrapFailureSignatureRef.current = failureSignature;
        reportBootstrapFailure(failureReport, error);
      }
      setBootstrapError(failureReport.message);
      setBootstrapProgress({
        running: false,
        complete: false,
        failed: true,
        currentStep: failureReport.currentStep,
        message: failureReport.message,
        currentStepEtaSeconds: failureReport.currentStepEtaSeconds,
        overallPercent: failureReport.overallPercent,
      });
      setBootstrapping(false);
    } finally {
      // Most completion paths are still managed by progress polling.
    }
  }

  async function copyFirstRunFootprint() {
    if (!navigator.clipboard) {
      setOnboardingFootprintCopyNotice("Clipboard unavailable.");
      return;
    }

    const fallbackFootprint = [
      "# AI Switchboard for Mac first-run footprint",
      "",
      "Pre-install preview. Some paths are written only after you opt in to the relevant mode or connector.",
      "",
      "- App support storage: ~/Library/Application Support/Mac AI Switchboard",
      "- Local engine/tool storage: ~/.headroom and app-owned helper runtimes",
      "- Shell profile managed blocks: zsh/bash/profile files, with managed markers",
      "- Claude Code: ~/.claude/settings.json, hooks, and managed instruction blocks",
      "- Codex: ~/.codex/config.toml and AGENTS.md managed blocks",
      "- Add-ons: RTK, Ponytail, MarkItDown, Caveman, and Repo Intelligence state when enabled",
      "- Backups: timestamped sidecars before managed config edits",
      "- Off mode: removes Switchboard-owned routing hooks and managed blocks",
      "",
      "Local-free builds do not require telemetry, sign-in, checkout, or hosted pricing services.",
    ].join("\n");

    await navigator.clipboard.writeText(
      managedFootprintReport
        ? formatManagedFootprintReport(managedFootprintReport)
        : fallbackFootprint,
    );
    setOnboardingFootprintCopyNotice("Copied footprint.");
    window.setTimeout(() => setOnboardingFootprintCopyNotice(null), 2500);
  }

  function canConfigureConnectorWithoutDetection(
    connector: ClientConnectorStatus,
  ) {
    return !connectorControlState(connector).disabled;
  }

  function getConnectorSupportWarning(connector: ClientConnectorStatus) {
    return connectorSupportWarnings[connector.clientId] ?? null;
  }

  function applyAppUpdatePatch(patch: AppUpdateStatePatch) {
    if (Object.prototype.hasOwnProperty.call(patch, "config")) {
      setAppUpdateConfig(patch.config ?? null);
    }
    if (Object.prototype.hasOwnProperty.call(patch, "availableUpdate")) {
      setAppUpdateAvailable(patch.availableUpdate ?? null);
    }
    if (Object.prototype.hasOwnProperty.call(patch, "readyToRestart")) {
      setAppUpdateReadyToRestart(patch.readyToRestart ?? false);
    }
    if (Object.prototype.hasOwnProperty.call(patch, "showDialog")) {
      setShowAppUpdateDialog(patch.showDialog ?? false);
    }
    if (Object.prototype.hasOwnProperty.call(patch, "statusCopy")) {
      setAppUpdateStatusCopy(patch.statusCopy ?? null);
    }
  }

  async function refreshAppUpdateConfiguration() {
    applyAppUpdatePatch(await loadAppUpdateConfiguration());
  }

  async function checkForAppUpdate({
    background = false,
    knownUpdateVersion = null,
  }: {
    background?: boolean;
    knownUpdateVersion?: string | null;
  } = {}) {
    let config = appUpdateConfig;

    if (!config) {
      const configPatch = await loadAppUpdateConfiguration();
      applyAppUpdatePatch(configPatch);
      config = configPatch.config ?? null;
    }

    if (!config) {
      return;
    }

    const blockedPatch = getBlockedAppUpdateCheckPatch(config, background);
    if (blockedPatch) {
      applyAppUpdatePatch(blockedPatch);
      return;
    }

    setAppUpdateBusy(true);
    if (!background) {
      setAppUpdateStatusCopy("Checking for a new Headroom release…");
    }

    try {
      const patch = await runAppUpdateCheck({ background, knownUpdateVersion });
      applyAppUpdatePatch(patch);

      if (background && patch.availableUpdate) {
        const windowVisible = hasTauriRuntime()
          ? await getCurrentWindow()
              .isVisible()
              .catch(() => false)
          : true;
        if (
          shouldNotifyAboutAvailableAppUpdate({
            background,
            availableUpdate: patch.availableUpdate,
            knownUpdateVersion,
            windowVisible,
          })
        ) {
          await sendAppUpdateNotification(patch.availableUpdate.version);
        }
        if (!windowVisible) {
          await maybeFireStaleAppUpdateNotification(patch.availableUpdate);
        }
      }
    } finally {
      setAppUpdateBusy(false);
    }
  }

  async function installAvailableUpdate() {
    if (!appUpdateAvailable) {
      return;
    }

    setAppUpdateInstallBusy(true);
    const installStatusCopy = getAppUpdateInstallStatusCopy(appUpdateAvailable);
    if (installStatusCopy) {
      setAppUpdateStatusCopy(installStatusCopy);
    }

    try {
      const versionForCopy = appUpdateAvailable.version;
      applyAppUpdatePatch(
        await runAppUpdateInstall({
          availableUpdate: appUpdateAvailable,
          onProgress: (progress) => {
            setAppUpdateStatusCopy(
              formatAppUpdateProgressCopy(versionForCopy, progress),
            );
          },
        }),
      );
    } finally {
      setAppUpdateInstallBusy(false);
    }
  }

  function restartIntoInstalledUpdate() {
    void invoke("restart_app");
  }

  async function refreshConnectors() {
    const generation = ++connectorsRefreshGenerationRef.current;
    try {
      if (isCurrentConnectorRefresh(generation, connectorsRefreshGenerationRef.current)) {
        setConnectorsError(null);
      }
      const items = await invoke<ClientConnectorStatus[]>(
        "get_client_connectors",
      );
      if (!isCurrentConnectorRefresh(generation, connectorsRefreshGenerationRef.current)) {
        return;
      }
      applyConnectorsIfChanged(items);
    } catch (error) {
      if (!isCurrentConnectorRefresh(generation, connectorsRefreshGenerationRef.current)) {
        return;
      }
      setConnectorsError(
        error instanceof Error
          ? error.message
          : "Could not load connector status.",
      );
    }
  }

  async function refreshSwitchboardState() {
    const generation = ++connectorsRefreshGenerationRef.current;
    try {
      const state = await invoke<SwitchboardState>("get_switchboard_state");
      applySwitchboardStateIfChanged(state);
      applyRuntimeStatusIfChanged(state.runtime);
      if (isCurrentConnectorRefresh(generation, connectorsRefreshGenerationRef.current)) {
        applyConnectorsIfChanged(state.clients);
      }
    } catch {
      applySwitchboardStateIfChanged(null);
    }
  }

  async function refreshDoctorReport() {
    await refreshDoctorReportController({
      invoke,
      setDoctorReport,
      setManagedFootprintReport,
    });
  }

  async function handleSetSwitchboardMode(mode: SwitchboardMode) {
    if (switchboardModeBusy !== null) {
      return;
    }
    setSwitchboardModeBusy(mode);
    setSwitchboardModeError(null);
    setDoctorRepairSuccess(null);
    try {
      const state = await invoke<SwitchboardState>("set_switchboard_mode", {
        mode,
      });
      applySwitchboardStateIfChanged(state);
      applyRuntimeStatusIfChanged(state.runtime);
      applyConnectorsIfChanged(state.clients);
      await refreshDoctorReport();
    } catch (error) {
      setSwitchboardModeError(
        `${error instanceof Error ? error.message : "Could not switch optimization mode."} Switchboard and Doctor have been refreshed.`,
      );
      await Promise.allSettled([
        refreshSwitchboardState(),
        refreshDoctorReport(),
      ]);
    } finally {
      setSwitchboardModeBusy(null);
    }
  }

  async function handleSetSavingsMode(mode: SavingsMode) {
    if (savingsModeBusy !== null) {
      return;
    }
    setSavingsModeBusy(mode);
    setSwitchboardModeError(null);
    setDoctorRepairSuccess(null);
    try {
      const state = await invoke<SwitchboardState>("set_savings_mode", {
        mode,
      });
      applySwitchboardStateIfChanged(state);
      applyRuntimeStatusIfChanged(state.runtime);
      applyConnectorsIfChanged(state.clients);
      await refreshDoctorReport();
    } catch (error) {
      setSwitchboardModeError(
        `${error instanceof Error ? error.message : "Could not change savings profile."} Switchboard and Doctor have been refreshed.`,
      );
      await Promise.allSettled([
        refreshSwitchboardState(),
        refreshDoctorReport(),
      ]);
    } finally {
      setSavingsModeBusy(null);
    }
  }

  async function handleDoctorRepair(action: string) {
    await runDoctorRepairAction(action, {
      currentBusyAction: doctorRepairBusy,
      invoke,
      refreshSwitchboardState,
      setDoctorRepairBusy,
      setDoctorRepairError,
      setDoctorRepairSuccess,
      setDoctorReport,
    });
  }

  async function refreshRuntimeStatus() {
    try {
      const runtime = await invoke<RuntimeStatus>("get_runtime_status");
      applyRuntimeStatusIfChanged(runtime);
      void maybeFireUrgentRuntimeNotification(runtime);
      try {
        const cache = await invoke<{ enabled: boolean }>(
          "get_semantic_cache_status",
        );
        setSemanticCacheEnabled(cache.enabled);
      } catch {
        setSemanticCacheEnabled(false);
      }
    } catch (error) {
      setConnectorsError(
        error instanceof Error
          ? error.message
          : "Could not load runtime status.",
      );
    }
  }

  async function handleResumeRuntime() {
    if (resuming) {
      return;
    }
    setResuming(true);
    setResumeError(null);
    try {
      await invoke("force_restart_headroom");
      await refreshRuntimeStatus();
      await refreshDoctorReport();
    } catch (error) {
      setResumeError(
        error instanceof Error ? error.message : "Could not restart Headroom.",
      );
    } finally {
      setResuming(false);
    }
  }

  async function refreshClaudeProjects() {
    setClaudeProjectsBusy(true);
    try {
      setClaudeProjectsError(null);
      const projects = await invoke<ClaudeCodeProject[]>(
        "get_claude_code_projects",
      );
      applyClaudeProjectsIfChanged(projects);
    } catch (error) {
      setClaudeProjectsError(
        error instanceof Error
          ? error.message
          : "Could not load Claude Code projects.",
      );
    } finally {
      setClaudeProjectsBusy(false);
    }
  }

  async function copyPlannedConnectorCommand(
    command: string,
    connectorName: string,
  ) {
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(command);
      setPlannedConnectorCopyNotice(`${connectorName} copied.`);
      window.setTimeout(() => setPlannedConnectorCopyNotice(null), 2000);
    } catch {
      setPlannedConnectorCopyNotice(
        "Copy failed. Command remains visible below.",
      );
      window.setTimeout(() => setPlannedConnectorCopyNotice(null), 3000);
    }
  }

  async function copyReleaseReadinessReport() {
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      if (releaseReadinessReport?.report) {
        await navigator.clipboard.writeText(
          formatReleaseReadinessReportSnapshot(
            releaseReadinessReport.report,
            releaseReadinessReport.reportPath,
          ),
        );
        setReleaseReadinessCopyNotice("Release report snapshot copied.");
      } else {
        await navigator.clipboard.writeText(formatReleaseReadinessCommandCopy());
        setReleaseReadinessCopyNotice("Release report command copied.");
      }
      window.setTimeout(() => setReleaseReadinessCopyNotice(null), 2000);
    } catch {
      setReleaseReadinessCopyNotice("Copy failed. Release text remains visible below.");
      window.setTimeout(() => setReleaseReadinessCopyNotice(null), 3000);
    }
  }

  async function copySettingsExport() {
    if (!navigator.clipboard) {
      setSettingsTransferNotice("Clipboard unavailable.");
      return;
    }

    const bundle = buildSettingsExportBundle({
      dashboard,
      connectors,
      switchboardMode,
      savingsMode,
    });
    await navigator.clipboard.writeText(formatSettingsExportBundle(bundle));
    setSettingsTransferNotice("Settings export copied.");
    window.setTimeout(() => setSettingsTransferNotice(null), 2500);
  }

  function previewSettingsImport() {
    const preview = parseSettingsImport(settingsImportText);
    setSettingsImportPreview(preview);
    setSettingsTransferNotice(preview.valid ? "Import preview ready." : null);
  }

  async function applySettingsImport() {
    const preview = settingsImportPreview ?? parseSettingsImport(settingsImportText);
    setSettingsImportPreview(preview);
    if (!preview.valid) {
      setSettingsTransferNotice(null);
      return;
    }

    setSettingsImportBusy(true);
    setSettingsTransferNotice("Applying safe preferences...");
    try {
      if (
        preview.safePreferences.switchboardMode &&
        preview.safePreferences.switchboardMode !== switchboardMode
      ) {
        await handleSetSwitchboardMode(preview.safePreferences.switchboardMode);
      }
      if (
        preview.safePreferences.savingsMode &&
        preview.safePreferences.savingsMode !== savingsMode
      ) {
        await handleSetSavingsMode(preview.safePreferences.savingsMode);
      }
      setSettingsTransferNotice("Safe settings applied.");
      window.setTimeout(() => setSettingsTransferNotice(null), 2500);
    } finally {
      setSettingsImportBusy(false);
    }
  }

  async function refreshReleaseReadinessReport() {
    setReleaseReadinessRefreshing(true);
    setReleaseReadinessError(null);
    setReleaseReadinessCopyNotice(null);
    try {
      const payload = await invoke<ReleaseReadinessReportPayload>(
        "refresh_release_readiness_report",
      );
      setReleaseReadinessReport(payload);
      setReleaseReadinessCopyNotice("Release report refreshed.");
      window.setTimeout(() => setReleaseReadinessCopyNotice(null), 2000);
    } catch (error) {
      setReleaseReadinessError(
        describeInvokeError(error, "Could not refresh release report."),
      );
    } finally {
      setReleaseReadinessRefreshing(false);
    }
  }

  function releaseEvidenceControllerOptions() {
    return {
      invoke,
      setBusyId: setReleaseEvidenceBusyId,
      setCopyNotice: setReleaseReadinessCopyNotice,
      setError: setReleaseReadinessError,
      setReport: setReleaseReadinessReport,
      setResult: setReleaseEvidenceResult,
      setTimeout: window.setTimeout.bind(window),
    };
  }

  async function runReleaseEvidenceCommand(commandId: string) {
    await runReleaseEvidenceCommandController(
      commandId,
      releaseEvidenceControllerOptions(),
    );
  }

  async function runLocalReleaseEvidenceSequence() {
    await runLocalReleaseEvidenceSequenceController(
      releaseEvidenceControllerOptions(),
    );
  }

  async function autoConfigureConnectorsForLauncher() {
    setConnectorsBusy(true);
    setConnectorsError(null);

    try {
      let latestConnectors = await invoke<ClientConnectorStatus[]>(
        "get_client_connectors",
      );
      applyConnectorsIfChanged(latestConnectors);

      const step = nextAutoConfigureStep(
        getLauncherAutoConfigureDecision(latestConnectors),
        latestConnectors,
      );

      if (step.kind === "show_client_setup") {
        setLauncherStage("client_setup");
        return;
      }

      if (step.kind === "apply") {
        for (const clientId of step.clientIds) {
          await invoke<ClientSetupResult>("apply_client_setup", { clientId });
        }
        latestConnectors = await invoke<ClientConnectorStatus[]>(
          "get_client_connectors",
        );
        applyConnectorsIfChanged(latestConnectors);

        const postApplyStep = nextAutoConfigureStepAfterApply(
          getLauncherAutoConfigureDecision(latestConnectors),
        );
        if (postApplyStep.kind !== "begin_proxy_verification") {
          setLauncherStage("client_setup");
          return;
        }
      }

      await beginProxyVerificationStep();
    } catch (error) {
      setConnectorsError(
        error instanceof Error
          ? error.message
          : "Could not configure your coding tools automatically.",
      );
      setLauncherStage("client_setup");
    } finally {
      setConnectorsBusy(false);
    }
  }

  async function handleFirstLaunchContinue() {
    await autoConfigureConnectorsForLauncher();
  }

  async function openExternalLink(url: string) {
    await invoke("open_external_link", { url });
  }

  async function runAddonAction(
    command: "install_addon" | "set_addon_enabled" | "uninstall_addon",
    id: string,
    enabled?: boolean,
  ) {
    const copy = addonCopy[id];
    const busyLabel =
      command === "install_addon"
        ? copy?.installing
        : command === "uninstall_addon"
          ? copy?.uninstalling
          : enabled
            ? copy?.enabling
            : copy?.disabling;
    setAddonBusyId(id);
    setAddonBusyLabel(busyLabel ?? null);
    setAddonError(null);
    setAddonResult(null);
    try {
      const next = await invoke<DashboardState>(command, { id, enabled });
      setDashboard(next);
      if (id === "rtk") {
        await refreshRuntimeStatus();
      }
      const message =
        command === "install_addon"
          ? copy?.installed
          : command === "uninstall_addon"
            ? copy?.uninstalled
            : enabled
              ? undefined
              : copy?.disabled;
      if (message) {
        setAddonResult({ id, message });
      }
    } catch (error) {
      setAddonError(
        error instanceof Error
          ? error.message
          : "The addon action could not be completed.",
      );
    } finally {
      setAddonBusyId(null);
      setAddonBusyLabel(null);
    }
  }

  async function prepareRepoMemoryMcp(): Promise<boolean> {
    setAddonBusyId("repo-memory");
    setAddonBusyLabel("Preparing Repo Memory MCP...");
    setAddonError(null);
    setAddonResult(null);
    try {
      await invoke<DashboardState>("install_repo_memory_mcp");
      const next = await invoke<DashboardState>("start_repo_memory_mcp");
      setDashboard(next);
      await refreshRuntimeStatus();
      setAddonResult({
        id: "repo-memory",
        message:
          "Repo Memory MCP prepared. The app installed it, ran the read-only smoke check, and marked it active for supported agents.",
      });
      return true;
    } catch (error) {
      setAddonError(
        error instanceof Error
          ? error.message
          : "Repo Memory MCP could not be prepared.",
      );
      return false;
    } finally {
      setAddonBusyId(null);
      setAddonBusyLabel(null);
    }
  }

  async function setRepoMemoryMcpActive(active: boolean): Promise<boolean> {
    setAddonBusyId("repo-memory");
    setAddonBusyLabel(active ? "Starting Repo Memory MCP..." : "Stopping Repo Memory MCP...");
    setAddonError(null);
    setAddonResult(null);
    try {
      const next = await invoke<DashboardState>(
        active ? "start_repo_memory_mcp" : "stop_repo_memory_mcp",
      );
      setDashboard(next);
      await refreshRuntimeStatus();
      setAddonResult({
        id: "repo-memory",
        message: active
          ? "Repo Memory MCP marked active. Supported agents can request read-only repo context."
          : "Repo Memory MCP stopped for this app session. Agent MCP configuration was left intact.",
      });
      return true;
    } catch (error) {
      setAddonError(
        error instanceof Error
          ? error.message
          : active
            ? "Repo Memory MCP could not be started."
            : "Repo Memory MCP could not be stopped.",
      );
      return false;
    } finally {
      setAddonBusyId(null);
      setAddonBusyLabel(null);
    }
  }

  async function setCavemanLevel(
    level: "scoped" | "aggressive" | "compact_chinese",
  ) {
    setAddonBusyId("caveman");
    setAddonBusyLabel("Updating Caveman level...");
    setAddonError(null);
    setAddonResult(null);
    try {
      const next = await invoke<DashboardState>("set_caveman_level", { level });
      setDashboard(next);
    } catch (error) {
      setAddonError(
        error instanceof Error
          ? error.message
          : "The Caveman level could not be updated.",
      );
    } finally {
      setAddonBusyId(null);
      setAddonBusyLabel(null);
    }
  }

  async function handleContactSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const validationError = getContactRequestValidationError(
      CONTACT_FORM_URL,
      contactEmail,
    );
    if (validationError) {
      setContactSubmitError(validationError);
      setContactSubmitSuccess(null);
      return;
    }

    const trimmed = contactEmail.trim();
    const trimmedMessage = contactMessage.trim().slice(0, 2000);
    setContactSubmitBusy(true);
    setContactSubmitError(null);
    setContactSubmitSuccess(null);

    try {
      await invoke("submit_contact_request", {
        url: CONTACT_FORM_URL,
        email: trimmed,
        message: trimmedMessage || null,
      });
      setContactEmail("");
      setContactMessage("");
      setContactSubmitSuccess(
        "Thanks. Check your inbox for a confirmation email.",
      );
    } catch (error) {
      setContactSubmitError(
        error instanceof Error
          ? error.message
          : "Could not submit the contact request.",
      );
    } finally {
      setContactSubmitBusy(false);
    }
  }

  async function beginProxyVerificationStep() {
    const generation = ++connectorsRefreshGenerationRef.current;
    proxyVerificationSessionRef.current += 1;
    let fresh = connectors;
    try {
      const items = await invoke<ClientConnectorStatus[]>("get_client_connectors");
      if (isCurrentConnectorRefresh(generation, connectorsRefreshGenerationRef.current)) {
        fresh = items;
        applyConnectorsIfChanged(items);
      }
    } catch {
      // fall back to cached state
    }

    setLauncherStage("proxy_verify");
    setProxyVerificationHint(null);
    setProxyVerificationRows(buildInitialProxyVerificationRows(fresh));
    // Reset to null so the polling effect re-anchors on its first reachable
    // /stats reading. Setting it here would risk anchoring on a stale value
    // from a prior visit to this stage.
    proxyVerificationRequestAnchorRef.current = null;
  }

  async function runConnectorSmokeTest(row: ProxyVerificationRow) {
    if (connectorSmokeBusyId !== null || row.state === "verified") {
      return;
    }
    const sessionId = proxyVerificationSessionRef.current;
    setConnectorSmokeBusyId(row.clientId);
    setProxyVerificationHint(null);
    setProxyVerificationRows((current) =>
      current.map((item) =>
        item.clientId === row.clientId
          ? { ...item, state: "testing", message: "Sending test prompt..." }
          : item,
      ),
    );

    try {
      const result = await invoke<ConnectorSmokeTestResult>(
        "run_connector_smoke_test",
        { clientId: row.clientId },
      );
      if (
        !shouldApplyConnectorSmokeResult(
          proxyVerificationSessionRef.current,
          sessionId,
          row.clientId,
          result.clientId,
        )
      ) {
        return;
      }
      setProxyVerificationRows((current) =>
        current.map((item) =>
          item.clientId === row.clientId
            ? {
                ...item,
                // The native smoke command already requires both a zero exit
                // status and the exact expected response. Marking this row
                // verified here prevents an unrelated later request-counter
                // tick from being mistaken for proof of this attempt.
                state: result.success ? "verified" : "waiting",
                message: result.summary,
              }
            : item,
        ),
      );
      if (!result.supported || !result.success) {
        const details = [result.stderrTail, result.stdoutTail]
          .filter(Boolean)
          .join("\n")
          .trim();
        setProxyVerificationHint(
          details.length > 0
            ? `${result.summary} ${details.slice(-300)}`
            : result.summary,
        );
      }
    } catch (error) {
      if (sessionId !== proxyVerificationSessionRef.current) {
        return;
      }
      const message =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Could not send the test prompt.";
      setProxyVerificationRows((current) =>
        current.map((item) =>
          item.clientId === row.clientId
            ? { ...item, state: "waiting", message }
            : item,
        ),
      );
      setProxyVerificationHint(message);
    } finally {
      if (sessionId === proxyVerificationSessionRef.current) {
        setConnectorSmokeBusyId(null);
      }
    }
  }

  async function runAllSupportedConnectorSmokeTests() {
    if (connectorSmokeBusyId !== null) {
      return;
    }
    const pendingRows = proxyVerificationRows.filter(
      (row) => row.oneClickSupported && row.state !== "verified",
    );
    for (const row of pendingRows) {
      await runConnectorSmokeTest(row);
    }
  }

  async function toggleConnector(
    connector: ClientConnectorStatus,
    nextEnabled: boolean,
  ) {
    setConnectorsBusy(true);
    setConnectorsError(null);
    try {
      if (nextEnabled) {
        await invoke<ClientSetupResult>("apply_client_setup", {
          clientId: connector.clientId,
        });
      } else {
        await invoke("disable_client_setup", { clientId: connector.clientId });
      }

      const latestDashboard = await loadDashboard();
      applyDashboardIfChanged(latestDashboard);
      void refreshSavingsAttributionEvents();
      await refreshConnectors();
    } catch (error) {
      setConnectorsError(
        error instanceof Error ? error.message : "Failed to update connector.",
      );
    } finally {
      setConnectorsBusy(false);
    }
  }

  async function verifyConnectors() {
    if (connectorsBusy) return;
    setConnectorsBusy(true);
    try {
      await refreshConnectors();
    } finally {
      setConnectorsBusy(false);
    }
  }

  function dismissCodexNudge() {
    setCodexNudgeDismissed(true);
    try {
      window.localStorage.setItem("headroom:codexNudgeDismissed", "1");
    } catch {
      // localStorage unavailable (private mode); the nudge stays dismissed for
      // this session via state, which is good enough.
    }
  }

  function handleLauncherSurfaceMouseDown(event: MouseEvent<HTMLElement>) {
    if (event.button !== 0) {
      return;
    }

    const target = event.target as HTMLElement;
    if (
      target.closest(
        "button, input, textarea, select, a, [role='button'], [data-no-drag]",
      )
    ) {
      return;
    }

    if (hasTauriRuntime()) {
      void getCurrentWindow().startDragging();
    }
  }

  const hidingRef = useRef(false);

  function triggerHide() {
    if (hidingRef.current) return;
    hidingRef.current = true;
    document.documentElement.classList.add("window-hiding");
    window.setTimeout(() => {
      void invoke("hide_launcher_animated");
    }, launcherHideAnimationMs);
    setTimeout(() => {
      document.documentElement.classList.remove("window-hiding");
      hidingRef.current = false;
    }, 400);
  }

  const headroomTool = dashboard.tools.find((tool) => tool.id === "headroom");
  const headroomVersion = headroomTool ? "0.0.0" : "Unknown";
  const lifetimeTotalTokensSent = savingsDashboard.dailySavings.reduce(
    (sum, point) => sum + point.totalTokensSent,
    0,
  );
  const lifetimeTotalTokensBeforeOptimization =
    lifetimeTotalTokensSent + savingsDashboard.lifetimeEstimatedTokensSaved;
  const headroomLifetimeSavingsPct =
    lifetimeTotalTokensBeforeOptimization > 0
      ? (savingsDashboard.lifetimeEstimatedTokensSaved /
          lifetimeTotalTokensBeforeOptimization) *
        100
      : null;
  const rtkAvgSavingsPct =
    runtimeStatus?.rtk.installed && (runtimeStatus.rtk.totalCommands ?? 0) > 0
      ? (runtimeStatus.rtk.avgSavingsPct ?? 0)
      : null;
  const lifetimeDataDays = new Set(
    savingsDashboard.dailySavings
      .map((point) => point.date)
      .filter((date) => Boolean(date)),
  ).size;
  const lifetimeDataDaysLabel =
    lifetimeDataDays > 0
      ? `Based on ${lifetimeDataDays} day${lifetimeDataDays === 1 ? "" : "s"} of data`
      : "No historical savings data yet";

  useEffect(() => {
    window.dispatchEvent(
      new CustomEvent("headroom:boot-progress", {
        detail: {
          percent: startupPercent,
          status: startupCopy,
        },
      }),
    );
  }, [startupPercent, startupCopy]);

  useEffect(() => {
    if (!startupReady || windowLabel === null) {
      return;
    }
    window.dispatchEvent(new CustomEvent("headroom:boot-complete"));
  }, [startupReady, windowLabel]);

  if (!startupReady || windowLabel === null) {
    return null;
  }

  // Block every window (launcher and main) until the user accepts the current
  // Terms of Use. New installs hit this in the launcher; updating users —
  // who may never see the launcher — hit it in the main window. Bumping the
  // backend's REQUIRED_TERMS_VERSION re-triggers it on the next launch.
  if (
    needsTermsAcceptance(
      dashboard.requiredTermsVersion,
      dashboard.acceptedTermsVersion,
    )
  ) {
    return (
      <TermsGate
        requiredVersion={dashboard.requiredTermsVersion}
        onAccepted={() =>
          setDashboard((prev) => ({
            ...prev,
            acceptedTermsVersion: prev.requiredTermsVersion,
          }))
        }
      />
    );
  }

  const upgradeFailure = runtimeStatus?.runtimeUpgradeFailure ?? null;
  const showUpgradeModal =
    runtimeUpgradeProgress.running &&
    !runtimeUpgradeProgress.complete &&
    !runtimeUpgradeProgress.failed;
  const showUpgradeSuccess =
    !runtimeUpgradeProgress.running &&
    runtimeUpgradeProgress.complete &&
    !runtimeUpgradeProgress.failed;
  const showUpgradeBanner =
    !runtimeUpgradeProgress.running && upgradeFailure !== null;
  const upgradeExhausted =
    upgradeFailure !== null &&
    upgradeFailure.attempts >= MAX_UPGRADE_AUTO_RETRIES;

  const upgradeOverlay = (
    <RuntimeUpgradeOverlay
      runtimeUpgradeProgress={runtimeUpgradeProgress}
      upgradeFailure={upgradeFailure}
      proxyReachable={runtimeStatus?.proxyReachable === true}
      supportIssuesUrl={SUPPORT_ISSUES_URL}
      maxUpgradeAutoRetries={MAX_UPGRADE_AUTO_RETRIES}
    />
  );

  // While a runtime upgrade is in flight, the venv is in the middle of being
  // swapped so `bootstrapComplete` may return false. Don't render the first-
  // run install wizard in that case — render a dedicated update screen in the
  // launcher instead.
  if (
    windowLabel === "launcher" &&
    (showUpgradeModal ||
      showUpgradeSuccess ||
      (showUpgradeBanner && upgradeFailure))
  ) {
    return (
      <LauncherRuntimeUpgradeStep
        appSemver={appSemver}
        onFirstLaunchContinue={handleFirstLaunchContinue}
        onMouseDown={handleLauncherSurfaceMouseDown}
        runtimeUpgradeProgress={runtimeUpgradeProgress}
        showUpgradeModal={showUpgradeModal}
        showUpgradeSuccess={showUpgradeSuccess}
        supportIssuesUrl={SUPPORT_ISSUES_URL}
        upgradeExhausted={upgradeExhausted}
        upgradeFailure={upgradeFailure}
      />
    );
  }
  if (windowLabel === "launcher" && launcherStage === "install") {
    return (
      <LauncherInstallStep
        appSemver={appSemver}
        bootstrapping={bootstrapping}
        bootstrapError={bootstrapError}
        bootstrapProgress={bootstrapProgress}
        bootstrapComplete={dashboard.bootstrapComplete}
        copyFirstRunFootprint={copyFirstRunFootprint}
        handleBootstrap={handleBootstrap}
        handleFirstLaunchContinue={handleFirstLaunchContinue}
        onMouseDown={handleLauncherSurfaceMouseDown}
        onboardingFootprintCopyNotice={onboardingFootprintCopyNotice}
        runtimeStatus={runtimeStatus}
        showInstallProgress={showInstallProgress}
        stepBasePercent={stepBasePercent}
        stepEtaSeedSeconds={stepEtaSeedSeconds}
        stepStartedAtMs={stepStartedAtMs}
      />
    );
  }

  if (windowLabel === "launcher" && launcherStage === "client_setup") {
    const launcherConnectors =
      connectors.length > 0 ? connectors : launcherConnectorFallback;

    return (
      <LauncherClientSetupStep
        appSemver={appSemver}
        connectors={launcherConnectors}
        connectorsBusy={connectorsBusy}
        connectorsError={connectorsError}
        onContinue={beginProxyVerificationStep}
        onMouseDown={handleLauncherSurfaceMouseDown}
        openConnectorHelpId={openConnectorHelpId}
        openConnectorWarningId={openConnectorWarningId}
        setLauncherStage={setLauncherStage}
        setOpenConnectorHelpId={setOpenConnectorHelpId}
        setOpenConnectorWarningId={setOpenConnectorWarningId}
        toggleConnector={toggleConnector}
      />
    );
  }

  if (windowLabel === "launcher" && launcherStage === "proxy_verify") {
    return (
      <LauncherProxyVerifyStep
        appSemver={appSemver}
        connectorSmokeBusyId={connectorSmokeBusyId}
        onBack={() => setLauncherStage("client_setup")}
        onContinue={() => {
          void invoke("complete_setup_wizard");
          setLauncherStage("post_install");
        }}
        onMouseDown={handleLauncherSurfaceMouseDown}
        proxyVerificationHint={proxyVerificationHint}
        proxyVerificationRows={proxyVerificationRows}
        runAllSupportedConnectorSmokeTests={runAllSupportedConnectorSmokeTests}
        runConnectorSmokeTest={runConnectorSmokeTest}
      />
    );
  }

  if (windowLabel === "launcher" && launcherStage === "post_install") {
    return (
      <LauncherPostInstallStep
        appSemver={appSemver}
        dashboard={dashboard}
        lifetimeDataDays={lifetimeDataDays}
        lifetimeDataDaysLabel={lifetimeDataDaysLabel}
        onBack={beginProxyVerificationStep}
        onGetStarted={triggerHide}
        onMouseDown={handleLauncherSurfaceMouseDown}
        savingsDashboard={savingsDashboard}
      />
    );
  }

  // Cold-cache warmup: proxy is up and the ML extras are installed, but the
  // ~260MB Kompress model hasn't loaded yet (it downloads lazily on first use,
  // and the desktop prefetches it in the background after a fresh install).
  // This is normal setup, not a fault, so it must not surface as an issue.
  const kompressWarming = Boolean(
    runtimeStatus &&
    runtimeStatus.running &&
    runtimeStatus.proxyReachable &&
    runtimeStatus.mlInstalled !== false &&
    runtimeStatus.kompressEnabled === false,
  );

  const runtimeIssues: string[] = [];
  if (runtimeStatus?.installed === false) {
    runtimeIssues.push("runtime not installed");
  }
  if (runtimeStatus?.running === false) {
    runtimeIssues.push(
      runtimeStatus.startupErrorHint ??
        runtimeStatus.startupError ??
        "runtime offline",
    );
  }
  if (runtimeStatus?.proxyReachable === false) {
    runtimeIssues.push("proxy unreachable");
  }
  if (runtimeStatus?.mcpConfigured === false) {
    runtimeIssues.push("MCP not configured");
  }
  if (runtimeStatus?.kompressEnabled === false && !kompressWarming) {
    runtimeIssues.push("Kompress disabled");
  }

  const runtimeHealthy = Boolean(
    runtimeStatus &&
    runtimeStatus.running &&
    runtimeStatus.proxyReachable &&
    runtimeStatus.mcpConfigured !== false &&
    (runtimeStatus.kompressEnabled !== false || kompressWarming),
  );
  const platformPreviewNotice =
    runtimeStatus?.supportTier === "experimental"
      ? runtimeStatus.platform === "linux"
        ? "Linux is currently a preview build. Core proxy routing is supported, but Headroom Learn and secure API key storage are disabled while the platform is hardened."
        : "This platform is currently in preview."
      : null;
  const calloutBanner = (() => {
    if (!runtimeStatus) {
      return {
        tone: "disconnected",
        title: "Headroom engine status is unavailable.",
      } as const;
    }

    if (runtimeStatus.paused) {
      if (runtimeStatus.autoPaused) {
        return {
          tone: "auto-paused",
          title:
            "The Headroom engine stopped unexpectedly. Traffic is passing through unoptimized.",
        } as const;
      }
      return {
        tone: "paused",
        title: "The Headroom engine is paused.",
      } as const;
    }

    if (runtimeStatus.starting) {
      return {
        tone: "starting",
        title: "Headroom is starting up.",
      } as const;
    }

    if (!localOnlyMode && pricingStatus?.needsAuthentication) {
      return {
        tone: "degraded",
        title: pricingStatus.gateMessage,
      } as const;
    }

    if (!localOnlyMode && pricingStatus && !pricingStatus.optimizationAllowed) {
      return {
        tone: "disabled",
        title: pricingStatus.gateMessage,
      } as const;
    }

    if (!localOnlyMode && pricingStatus?.shouldNudge) {
      return {
        tone: "starting",
        title: pricingStatus.gateMessage,
      } as const;
    }

    // Codex-only gate: surface in the top banner only when the Claude side isn't
    // itself gating/nudging (handled above), so mixed users never get a double
    // banner. Codex billing/pausing is scoped to Codex traffic.
    const codexUsage = localOnlyMode ? null : pricingStatus?.codex;
    if (codexUsage && codexUsage.optimizationAllowed === false) {
      return {
        tone: "disabled",
        title: codexUsage.gateMessage,
      } as const;
    }
    if (codexUsage?.shouldNudge) {
      return {
        tone: "starting",
        title: codexUsage.gateMessage,
      } as const;
    }

    if (runtimeHealthy) {
      if (connectorPhase === "disabled") {
        return {
          tone: "disabled",
          title:
            "No coding tools connected — Switchboard isn't reducing token use.",
        } as const;
      }
      if (connectorPhase === "verifying") {
        return {
          tone: "starting",
          title:
            "Click Test setup, then send a message in a connected tool to verify routing. Restart the tool first if it was already open.",
        } as const;
      }
      if (kompressWarming) {
        return {
          tone: "healthy",
          title: "AI Switchboard is running while finishing setup.",
        } as const;
      }
      return {
        tone: "healthy",
        title: "AI Switchboard is running and trimming prompt bloat.",
      } as const;
    }

    const disconnected =
      !runtimeStatus.installed ||
      !runtimeStatus.running ||
      !runtimeStatus.proxyReachable;
    return {
      tone: disconnected ? "disconnected" : "degraded",
      title: disconnected
        ? runtimeIssues.length > 0
          ? `AI Switchboard is not hooked up right now: ${runtimeIssues.join(", ")}.`
          : "AI Switchboard is not hooked up right now."
        : runtimeIssues.length > 0
          ? `AI Switchboard needs attention: ${runtimeIssues.join(", ")}.`
          : "AI Switchboard is running, but something needs attention.",
    } as const;
  })();

  const calloutTitle =
    calloutBanner.title.length <= 110
      ? calloutBanner.title
      : (() => {
          const primaryIssue = runtimeIssues[0];
          if (!primaryIssue) {
            return calloutBanner.title;
          }
          if (calloutBanner.tone === "disconnected") {
            return `AI Switchboard is not hooked up right now: ${primaryIssue}.`;
          }
          return `AI Switchboard needs attention: ${primaryIssue}.`;
        })();
  const showRuntimeRestartAction = shouldOfferRuntimeRestartAction(
    calloutBanner.tone,
    {
      runtimeHealthy,
      runtimeStarting: runtimeStatus?.starting === true,
      connectorPhase,
    },
  );
  const tierMismatch = localOnlyMode
    ? null
    : (pricingStatus?.tierMismatch ?? null);
  const switchboardConnectors = sortClientConnectors(
    aggregateClientConnectors(connectors),
  );
  const enabledSwitchboardConnectors = switchboardConnectors.filter(
    (connector) => connector.enabled,
  );
  const derivedSwitchboardMode: SwitchboardMode = deriveSwitchboardMode(
    runtimeStatus,
    enabledSwitchboardConnectors,
  );
  const switchboardMode = switchboardState?.mode ?? derivedSwitchboardMode;
  const switchboardEffectiveMode =
    switchboardState?.effectiveMode ?? derivedSwitchboardMode;
  const switchboardNeedsAttention =
    switchboardState?.needsAttention ??
    switchboardMode !== switchboardEffectiveMode;
  const switchboardModeCopy =
    switchboardState?.summary ?? switchboardModeSummary(switchboardMode);
  const savingsMode = switchboardState?.savingsMode ?? "balanced";
  const switchboardRtkLabel = runtimeStatus?.rtk.installed
    ? runtimeStatus.rtk.enabled
      ? "Enabled"
      : "Installed, off"
    : "Not installed";
  const switchboardProxyStatus =
    runtimeStatus?.running && runtimeStatus.proxyReachable
      ? "Running"
      : runtimeStatus?.paused
        ? "Paused"
        : "Offline";
  const switchboardRtkDetail =
    runtimeStatus?.rtk.enabled
      ? rtkAvgSavingsPct !== null
        ? `${percent1(rtkAvgSavingsPct)}% average savings`
        : "Shell output compression active"
      : runtimeStatus?.rtk.installed
        ? "Installed but disabled"
        : "Shell output compression not installed";
  const switchboardHeadroomLabel =
    (switchboardState?.enabledClients ?? enabledSwitchboardConnectors).length >
    0
      ? (switchboardState?.enabledClients ?? enabledSwitchboardConnectors)
          .map((connector) => connector.name)
          .join(", ")
      : "No clients enabled";
  const repoMemoryLifecycle = repoMemoryMcpLifecycle({
    configured: runtimeStatus?.repoMemoryMcpConfigured,
    error: runtimeStatus?.repoMemoryMcpError,
    active: runtimeStatus?.repoMemoryMcpActive,
    lastStartedAt: runtimeStatus?.repoMemoryMcpLastStartedAt,
    lastCheckedAt: runtimeStatus?.repoMemoryMcpLastCheckedAt,
    supervisionStatus: runtimeStatus?.repoMemoryMcpSupervisionStatus,
    relaunchSurvivalStatus: runtimeStatus?.repoMemoryMcpRelaunchSurvivalStatus,
    supervisionScope: runtimeStatus?.repoMemoryMcpSupervisionScope,
    service: runtimeStatus?.repoMemoryMcpService,
  });
  const exactCacheRecommended = recommendExactCacheDefault({
    mode: resolveSwitchboardModeForCache(runtimeStatus),
    semanticCacheEnabled,
    proxyReachable: runtimeStatus?.proxyReachable ?? false,
  }).recommend;
  const maxCompressionDisclosure = createMaxCompressionActivationPlan({
    mode: resolveSwitchboardModeForCache(runtimeStatus),
    semanticCacheEnabled,
    proxyReachable: runtimeStatus?.proxyReachable ?? false,
  }).excludedCopy;
  const switchboardInspectorRows = buildSwitchboardInspectorRows({
    runtimeStatus,
    switchboardState,
    switchboardConnectors,
    doctorRepairBusy,
    handleDoctorRepair,
    openSettingsFocus,
    repoMemoryLifecycle,
    addonBusyId,
    addonBusyLabel,
    setRepoMemoryMcpActive,
    prepareRepoMemoryMcp,
  });
  const switchboardLocalOnly = switchboardState?.localOnly ?? localOnlyMode;
  const switchboardRemoteServicesEnabled =
    switchboardState?.remoteServicesEnabled ?? !switchboardLocalOnly;
  const trialDaysRemaining = trialDaysRemainingFromPricing(pricingStatus);
  const localGraceHoursRemaining = localGraceHoursRemainingFromPricing(pricingStatus);
  const weeklyLimitPercentLabel = formatPercentValue(
    pricingStatus?.effectiveDisableThresholdPercent ??
      pricingStatus?.disableThresholdPercent,
  );
  const upgradeDefaultPlanId =
    pricingAudience === "individual"
      ? (pricingStatus?.recommendedSubscriptionTier ??
        pricingStatus?.codex?.recommendedSubscriptionTier ??
        cachedPricing.recommendedSubscriptionTier ??
        upgradePlansState.featuredPlanId)
      : "enterprise";
  const upgradeDefaultPlan =
    upgradePlansState.plans.find((plan) => plan.id === upgradeDefaultPlanId) ??
    null;
  const activeHeadroomPlanId =
    pricingAudience === "individual" &&
    pricingStatus?.account?.subscriptionActive
      ? (pricingStatus.account.subscriptionTier ?? null)
      : null;
  const downgradePlanId = getNextLowerUpgradePlanId(activeHeadroomPlanId);
  const visibleUpgradePlans = (() => {
    if (showAllUpgradePlans || upgradePlansState.plans.length <= 2) {
      return upgradePlansState.plans;
    }

    if (
      pricingAudience === "individual" &&
      activeHeadroomPlanId &&
      downgradePlanId
    ) {
      const visiblePlanIds = new Set<UpgradePlanId>([
        activeHeadroomPlanId,
        downgradePlanId,
      ]);
      const activeWindowPlans = upgradePlansState.plans.filter((plan) =>
        visiblePlanIds.has(plan.id),
      );
      if (activeWindowPlans.length === 2) {
        return activeWindowPlans;
      }
    }

    return upgradePlansState.plans.slice(0, 2);
  })();
  const hasHiddenUpgradePlans =
    visibleUpgradePlans.length < upgradePlansState.plans.length;
  const pendingUpgradePlanLabel = upgradePlanIntentLabel(pendingUpgradePlanId);
  const upgradeAuthMessage = pendingUpgradePlanLabel
    ? `Sign in with email to upgrade to the ${pendingUpgradePlanLabel} plan`
    : "Sign in with email to unlock your 7-day Switchboard trial";
  const accountDisplayEmail = accountDisplayEmailFromPricing(
    pricingStatus,
    authEmail,
  );
  const accountPlanName = accountPlanNameFromPricing(
    pricingStatus,
    trialDaysRemaining,
  );
  const upgradeTrialCallout = upgradeTrialCalloutFromPricing(
    pricingBusy,
    pricingStatus,
    localGraceHoursRemaining,
    openUpgradeAuthView,
  );
  const pricingAuthCard = (
    <PricingAuthCard
      authCode={authCode}
      authCodeRequestedFor={authCodeRequestedFor}
      authCodeValid={Boolean(authCode.trim())}
      authEmail={authEmail}
      authEmailValid={authEmailValid}
      authFlowError={authFlowError}
      authFlowSuccess={authFlowSuccess}
      authRequestBusy={authRequestBusy}
      authVerifyBusy={authVerifyBusy}
      onAuthCodeChange={(value) => {
        setAuthCode(value);
        setAuthFlowError(null);
      }}
      onAuthEmailChange={(value) => {
        setAuthEmail(value);
        setAuthFlowError(null);
      }}
      onRequestAuthCode={() => void handleRequestAuthCode()}
      onResetAuthStep={resetUpgradeAuthStep}
      onVerifyAuthCode={() => void handleVerifyAuthCode()}
      pricingError={pricingError}
      upgradeAuthMessage={upgradeAuthMessage}
    />
  );

  return (
    <TrayAppShell
      upgradeOverlay={upgradeOverlay}
      settingsView={
        <SettingsView
          hidden={activeView !== "settings"}
          readinessSignals={localFirstReadinessSourceSignals}
          dashboard={dashboard}
          switchboardMode={switchboardMode}
          savingsMode={savingsMode}
          connectors={connectors}
          appSemver={appSemver}
          settingsTransferNotice={settingsTransferNotice}
          setSettingsImportText={setSettingsImportText}
          setSettingsImportPreview={setSettingsImportPreview}
          setSettingsTransferNotice={setSettingsTransferNotice}
          settingsImportText={settingsImportText}
          settingsImportPreview={settingsImportPreview}
          settingsImportBusy={settingsImportBusy}
          copySettingsExport={copySettingsExport}
          previewSettingsImport={previewSettingsImport}
          applySettingsImport={applySettingsImport}
          plannedConnectorReadiness={plannedConnectorReadiness}
          plannedConnectorCopyNotice={plannedConnectorCopyNotice}
          connectorsBusy={connectorsBusy}
          connectorsError={connectorsError}
          verifyConnectors={verifyConnectors}
          openConnectorHelpId={openConnectorHelpId}
          setOpenConnectorHelpId={setOpenConnectorHelpId}
          toggleConnector={toggleConnector}
          copyPlannedConnectorCommand={copyPlannedConnectorCommand}
          autostartEnabled={autostartEnabled}
          autostartBusy={autostartBusy}
          handleAutostartToggle={handleAutostartToggle}
          showHeadroomDetails={showHeadroomDetails}
          setShowHeadroomDetails={setShowHeadroomDetails}
          setHeadroomLogLines={setHeadroomLogLines}
          headroomLogLines={headroomLogLines}
          headroomLogRef={headroomLogRef}
          headroomVersion={headroomVersion}
          headroomLifetimeSavingsPct={headroomLifetimeSavingsPct}
          runtimeStatus={runtimeStatus}
          kompressWarming={kompressWarming}
          appUpdateConfig={appUpdateConfig}
          appUpdateBusy={appUpdateBusy}
          appUpdateInstallBusy={appUpdateInstallBusy}
          appUpdateStatusCopy={appUpdateStatusCopy}
          checkForAppUpdate={checkForAppUpdate}
          releaseReadinessRefreshing={releaseReadinessRefreshing}
          releaseEvidenceBusyId={releaseEvidenceBusyId}
          releaseEvidenceResult={releaseEvidenceResult}
          releaseReadinessCommand={releaseReadinessCommand}
          releaseReadinessReport={releaseReadinessReport}
          releaseReadinessEvidence={releaseReadinessEvidence}
          releaseReadinessAction={releaseReadinessAction}
          releaseReadinessError={releaseReadinessError}
          releaseReadinessCounts={releaseReadinessCounts}
          releaseReadinessRows={releaseReadinessRows}
          releaseLocalEvidenceRows={releaseLocalEvidenceRows}
          releaseReadinessCopyNotice={releaseReadinessCopyNotice}
          copyReleaseReadinessReport={copyReleaseReadinessReport}
          refreshReleaseReadinessReport={refreshReleaseReadinessReport}
          runReleaseEvidenceCommand={runReleaseEvidenceCommand}
          runLocalReleaseEvidenceSequence={runLocalReleaseEvidenceSequence}
          formatLocalReleaseEvidenceSequenceCopy={formatLocalReleaseEvidenceSequenceCopy}
          setUninstallError={setUninstallError}
          setShowUninstallDialog={setShowUninstallDialog}
          SUPPORT_ISSUES_URL={SUPPORT_ISSUES_URL}
        />
      }
      activeView={activeView}
      setActiveView={setActiveView}
      localOnlyMode={localOnlyMode}
      tierMismatch={tierMismatch}
      upgradeActionError={upgradeActionError}
      upgradeActionBusy={upgradeActionBusy}
      handleUpgradeAction={handleUpgradeAction}
      calloutBanner={calloutBanner}
      calloutTitle={calloutTitle}
      platformPreviewNotice={platformPreviewNotice}
      showRuntimeRestartAction={showRuntimeRestartAction}
      handleResumeRuntime={handleResumeRuntime}
      resuming={resuming}
      resumeError={resumeError}
      connectorPhase={connectorPhase}
      beginProxyVerificationStep={beginProxyVerificationStep}
      connectors={connectors}
      pricingStatus={pricingStatus}
      codexNudgeDismissed={codexNudgeDismissed}
      connectorsBusy={connectorsBusy}
      toggleConnector={toggleConnector}
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
      handleSetSwitchboardMode={handleSetSwitchboardMode}
      handleSetSavingsMode={handleSetSavingsMode}
      doctorReport={doctorReport}
      doctorRepairBusy={doctorRepairBusy}
      doctorRepairError={doctorRepairError}
      doctorRepairSuccess={doctorRepairSuccess}
      managedFootprintReport={managedFootprintReport}
      handleDoctorRepair={handleDoctorRepair}
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
      onActivateEverything={() => void activateEverything()}
      onDeactivateEverything={() => void deactivateEverything()}
      onActivateMasterFeature={(featureId) =>
        void activateMasterFeature(featureId)
      }
      onDeactivateMasterFeature={(featureId) =>
        void deactivateMasterFeature(featureId)
      }
      onOpenMasterFeature={(featureId) => {
        if (featureId === "rollback") {
          openSettingsFocus("rollback-center");
          return;
        }
        setActiveView(masterFeatureView(featureId));
      }}
      masterActivationIsActive={masterActivationIsActive}
      masterOperation={masterOperation}
      onActivateMaxCompression={() => void activateMaxCompression()}
      maxCompressionBusy={maxCompressionBusy}
      maxCompressionDisclosure={maxCompressionDisclosure}
      exactCacheRecommended={exactCacheRecommended}
      semanticCacheEnabled={semanticCacheEnabled}
      onOpenCompressionPlaybook={openCompressionPlaybook}
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
      activityFeedError={activityFeedError}
      activityFeedLoaded={activityFeedLoaded}
      setLatestRepoIntelligenceSummary={setLatestRepoIntelligenceSummary}
      addonError={addonError}
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
      onMeasuredAddonSavingsRecorded={refreshSavingsAttributionEvents}
      handleRtkToggle={handleRtkToggle}
      setCavemanLevel={setCavemanLevel}
      copyPlannedConnectorCommand={copyPlannedConnectorCommand}
      pricingAudience={pricingAudience}
      setPricingAudience={setPricingAudience}
      setUpgradeActionError={setUpgradeActionError}
      billingPeriod={billingPeriod}
      setBillingPeriod={setBillingPeriod}
      upgradeTrialCallout={upgradeTrialCallout}
      authRequestBusy={authRequestBusy}
      authVerifyBusy={authVerifyBusy}
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
      handleReactivateSubscription={handleReactivateSubscription}
      reactivateBusy={reactivateBusy}
      hasHiddenUpgradePlans={hasHiddenUpgradePlans}
      showAllUpgradePlans={showAllUpgradePlans}
      setShowAllUpgradePlans={setShowAllUpgradePlans}
      reactivateError={reactivateError}
      pricingAuthCard={pricingAuthCard}
      showSavingsInfo={showSavingsInfo}
      showUninstallDialog={showUninstallDialog}
      setShowUninstallDialog={setShowUninstallDialog}
      uninstallBusy={uninstallBusy}
      uninstallDisclosureTitle={uninstallDisclosureTitle}
      uninstallDisclosureItems={uninstallDisclosureItems}
      uninstallDisclosureFooter={uninstallDisclosureFooter}
      uninstallCopyNotice={uninstallCopyNotice}
      uninstallError={uninstallError}
      copyUninstallDryRunReport={copyUninstallDryRunReport}
      handleUninstall={handleUninstall}
      pendingPlanChange={pendingPlanChange}
      cancelPlanChange={cancelPlanChange}
      confirmPlanChange={confirmPlanChange}
      planChangeError={planChangeError}
      planChangeBusy={planChangeBusy}
      showAppUpdateDialog={showAppUpdateDialog}
      setShowAppUpdateDialog={setShowAppUpdateDialog}
      appUpdateAvailable={appUpdateAvailable}
      appUpdateReadyToRestart={appUpdateReadyToRestart}
      appUpdateInstallBusy={appUpdateInstallBusy}
      restartIntoInstalledUpdate={restartIntoInstalledUpdate}
      installAvailableUpdate={installAvailableUpdate}
    />
  );
}
