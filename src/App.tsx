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
  Brain,
  CaretLeft,
  Copy,
  Cpu,
  CurrencyDollar,
  Info,
  EnvelopeSimple,
  Key,
  SignOut,
  Sparkle,
  Terminal,
} from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import {
  refreshDoctorReport as refreshDoctorReportController,
  runDoctorRepairAction,
} from "./lib/doctorRepairController";
import {
  copyManagedDiffPreview as copyManagedDiffPreviewController,
  copyManagedRollbackExecutionPreview as copyManagedRollbackExecutionPreviewController,
  copyManagedRollbackInventory as copyManagedRollbackInventoryController,
  copyManagedRollbackPlan as copyManagedRollbackPlanController,
  copyManagedRollbackUndoAllPreview as copyManagedRollbackUndoAllPreviewController,
  type RollbackCopyOptions,
} from "./lib/rollbackCopyController";
import {
  runLocalReleaseEvidenceSequence as runLocalReleaseEvidenceSequenceController,
  runReleaseEvidenceCommand as runReleaseEvidenceCommandController,
  type ReleaseEvidenceCommandResult,
  type ReleaseReadinessReportPayload,
} from "./lib/releaseEvidenceController";
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
} from "./lib/appUpdate";
import { maybeFireTrialNotifications } from "./lib/trialNotifications";
import {
  maybeFireUrgentPricingNotifications,
  maybeFireUrgentRuntimeNotification,
} from "./lib/urgentNotifications";
import {
  estimateRepoIntelligenceSavings,
  type RepoIntelligenceSummary,
  type RepoSavingsEstimate,
} from "./lib/repoIntelligence";
import {
  RepoIntelligencePreview,
  repoIntelligencePreview,
} from "./components/RepoIntelligencePreview";
import {
  repoMemoryMcpInspectorRow,
  repoMemoryMcpLifecycle,
} from "./lib/repoMemoryMcp";
import {
  formatPlannedConnectorConfigCreationPlansMarkdown,
  getPlannedConnector,
  getPlannedConnectorConfigCreationPlan,
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorReadinessContract,
  getPlannedConnectorSetupChecklistScript,
  getPlannedConnectorSetupGuide,
  type PlannedConnector,
} from "./lib/plannedConnectors";
import {
  formatLocalReleaseEvidenceSequenceCopy,
  releaseReadinessCommand,
  formatReleaseReadinessCommandCopy,
  formatReleaseReadinessNextAction,
  formatReleaseReadinessReportSnapshot,
  formatReleaseReadinessSourceLabel,
  localReleaseEvidenceCommandIds,
  releaseLocalEvidenceRowsFromReport,
  releaseReadinessEvidenceSummary,
  releaseReadinessGroups,
  releaseReadinessItemCount,
  releaseReadinessNextAction,
  releaseReadinessRowsFromReport,
  releaseReadinessStatusCounts,
  releaseShareableGates,
  type ReleaseReadinessReportSnapshot,
} from "./lib/releaseReadiness";
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
} from "./lib/appHelpers";
import {
  bootstrapFailureSignature,
  buildBootstrapFailureReport,
  buildBootstrapInvokeFailureReport,
  reportBootstrapFailure,
} from "./lib/bootstrapSentry";
import {
  CLAUDE_CODE_INSTALL_DOCS_URL,
  CLAUDE_CODE_INSTALL_CURL_CMD,
  CODEX_CLI_INSTALL_CMD,
  CODEX_CLI_LOGIN_CMD,
  CODEX_INSTALL_DOCS_URL,
} from "./lib/cliInstallCommands";
import {
  aggregateClientConnectors,
  compactNumber,
  connectorControlState,
  connectorCompatibilityReport,
  connectorCompatibilityRoutingEvidenceLabel,
  connectorSupportsAutomaticSetup,
  currency,
  currencyExact,
  formatDateTime,
  formatDayKey,
  formatLearnStatus,
  formatPlannedConnectorConfigGateSummary,
  getEnabledSupportedConnectors,
  hasEnabledConnector,
  percent1,
  sortClientConnectors,
  summarizePlannedConnectorReadiness,
} from "./lib/dashboardHelpers";
import {
  buildInitialProxyVerificationRows,
  getClaudeConnector,
  getContactRequestValidationError,
  getInitialLauncherStage,
  getLauncherAutoConfigureDecision,
  hasPendingOneClickProxyVerification,
  isValidEmailAddress,
  needsTermsAcceptance,
  nextAutoConfigureStep,
  nextAutoConfigureStepAfterApply,
  type LauncherStage,
} from "./lib/launcherHelpers";
import { mockDashboard } from "./lib/mockData";
import {
  cachePricingStatus,
  type CachedPricing,
  formatPercentValue,
  formatRemainingDays,
  readCachedPricing,
  subscriptionTierLabel,
  writeCachedPricing,
} from "./lib/pricing";
import {
  activityFeedSignature,
  notificationActionTargetId,
  safeNotificationActionView,
  safeTrayViewForMode,
  serializeState,
  type TrayView,
} from "./lib/trayHelpers";
import {
  trackAnalyticsEvent,
  trackInstallMilestoneOnce,
} from "./lib/analytics";
import { localOnlyModeEnabled } from "./lib/localMode";
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
} from "./lib/managedChanges";
import {
  buildDoctorTimelinePreview,
  buildUpgradeIssueUrl,
  sampleManagedBlock,
} from "./lib/appSupport";
import {
  buildSettingsExportBundle,
  formatSettingsExportBundle,
  parseSettingsImport,
  type SettingsImportPreview,
} from "./lib/settingsTransfer";
import {
  connectorSetupDetails,
  firstManagedConfigTarget,
  formatBackendConnectorConfigPlan,
  getConnectorDetectionWarning,
  getConnectorUnavailableReason,
  getPlannedConnectorNextStep,
  supportsNativeConfigApply,
  supportsNativeManagedRollback,
} from "./lib/settingsConnectorCopy";
import {
  formatBackendUninstallDryRunReport,
  formatUninstallDryRunReport,
  uninstallDisclosureFooter,
  uninstallDisclosureItems,
  uninstallDisclosureTitle,
} from "./lib/uninstallDisclosure";
import {
  deriveSwitchboardMode,
  switchboardModeSummary,
} from "./lib/switchboardDisplay";
import {
  buildAddonSavingsEstimate,
  CAVEMAN_TEMPLATE_BASELINE_TOKENS,
  CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS,
  PONYTAIL_TEMPLATE_BASELINE_TOKENS,
  PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS,
  MARKITDOWN_TEMPLATE_BASELINE_TOKENS,
  MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS,
  type SavingsCalculatorScope,
} from "./lib/savingsCalculator";
import { ActivityFeed } from "./components/ActivityFeed";
import { AddonsView } from "./components/AddonsView";
import { DoctorView } from "./components/DoctorView";
import { HomeView } from "./components/HomeView";
import { LauncherInstallStep } from "./components/LauncherInstallStep";
import { LauncherShell } from "./components/LauncherShell";
import { OptimizePanel } from "./components/OptimizePanel";
import { RepoMapView } from "./components/RepoMapView";
import { TraySidebar } from "./components/TraySidebar";
import type { SavingsChartMode } from "./components/SavingsChartTooltip";
import { SettingsConnectorPanel } from "./components/SettingsConnectorPanel";
import { SettingsLegalPanel } from "./components/SettingsLegalPanel";
import { SettingsOpenLoginCard } from "./components/SettingsOpenLoginCard";
import { RollbackCenter } from "./components/RollbackCenter";
import { SavingsInfoDialog } from "./components/SavingsInfoDialog";
import { SettingsTransferCard } from "./components/SettingsTransferCard";
import { SettingsUninstallCard } from "./components/SettingsUninstallCard";
import { TermsGate } from "./components/TermsGate";
import { UpgradeView } from "./components/UpgradeView";
import { UsageSavingsView } from "./components/UsageSavingsView";
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
  HeadroomLearnPrereqStatus,
  HeadroomLearnStatus,
  HeadroomSubscriptionTier,
  ManagedConfigApplyPreview,
  ManagedConfigApplyResult,
  ManagedFootprintReport,
  ManagedRollbackExecutionResult,
  ManagedRollbackPreview,
  ManagedRollbackUndoAllExecutionResult,
  ManagedRollbackUndoAllPreview,
  ActivityFeedResponse,
  AppliedPatterns,
  HourlySavingsPoint,
  OutputReduction,
  RuntimeStatus,
  RuntimeUpgradeProgress,
  SavingsAttributionEvent,
  SavingsMode,
  SwitchboardMode,
  SwitchboardState,
  UninstallDryRunReport,
} from "./lib/types";
import { hasTauriEventRuntime, hasTauriRuntime } from "./lib/tauriRuntime";

const localFirstReadinessSourceSignals = [
  "detectionEvidence",
  "Settings import/export",
  "Settings migration actions",
  "Copy settings export",
  "Apply safe preferences",
] as const;

interface AddonCopy {
  whatItDoes: string;
  installing?: string;
  uninstalling?: string;
  installed?: string;
  uninstalled?: string;
  enabling?: string;
  disabling?: string;
  disabled?: string;
}

const addonCopy: Record<string, AddonCopy> = {
  rtk: {
    whatItDoes:
      "RTK installs into the managed runtime, adds itself to the shell PATH, and enables the bash auto-rewrite hook. Agent shell commands route through RTK so noisy output is compacted before it spends tokens.",
    installing: "Downloading RTK and registering the bash hook...",
    uninstalling: "Removing RTK, its PATH entry, and the bash hook...",
    uninstalled:
      "RTK removed. Shell commands run normally, without output rewriting.",
    enabling: "Enabling RTK and registering the bash hook...",
    disabling: "Disabling RTK and removing the bash hook...",
    disabled:
      "RTK is off but still installed. Re-enable it later without re-downloading.",
  },
  markitdown: {
    whatItDoes:
      "MarkItDown installs into the managed Python runtime and registers a document Read hook. Documents can be converted to Markdown before an agent reads them, without installing anything system-wide.",
    installing: "Installing MarkItDown and registering the Read hook...",
    uninstalling: "Removing MarkItDown and its Read hook...",
    uninstalled:
      "MarkItDown removed. Your agent reads documents in their original format again.",
    enabling: "Enabling MarkItDown...",
    disabling: "Disabling MarkItDown...",
    disabled:
      "MarkItDown is off. It stays installed but no longer converts documents.",
  },
  caveman: {
    whatItDoes:
      "Caveman writes Switchboard-managed instruction blocks into Claude Code and Codex. It nudges agents toward terse output without hiding legal, safety, or debugging detail.",
    installing: "Writing Caveman guidance blocks...",
    uninstalling: "Removing Caveman guidance blocks...",
    installed:
      "Caveman installed. Pick scoped, aggressive, or Compact Chinese experimental mode any time.",
    uninstalled: "Caveman removed. Managed terse-output blocks were deleted.",
    enabling: "Enabling Caveman guidance...",
    disabling: "Disabling Caveman guidance...",
    disabled: "Caveman is off. Re-enable it later without recreating settings.",
  },
  ponytail: {
    whatItDoes:
      "Ponytail registers its marketplace plugin in Claude Code and/or Codex when those CLIs are on PATH. It nudges agents toward smaller, simpler edits and can run an over-engineering audit.",
    installing: "Registering Ponytail in available coding clients...",
    uninstalling: "Removing Ponytail from registered coding clients...",
    uninstalled:
      "Ponytail removed. Your agent writes code without the Ponytail nudge.",
    installed:
      "Ponytail installed. Run /ponytail-audit in an agent to scan this codebase for over-engineering.",
    enabling: "Enabling Ponytail...",
    disabling: "Disabling Ponytail...",
    disabled:
      "Ponytail is off. It stays installed but no longer nudges agents.",
  },
};

const connectorSupportWarnings: Record<string, string> = {};

const launcherConnectorFallback: ClientConnectorStatus[] = [
  {
    clientId: "claude_code",
    name: "Claude Code",
    installed: false,
    enabled: false,
    verified: false,
  },
  {
    clientId: "codex",
    name: "Codex",
    installed: false,
    enabled: false,
    verified: false,
  },
];

const idleBootstrapProgress: BootstrapProgress = {
  running: false,
  complete: false,
  failed: false,
  currentStep: "Idle",
  message: "Installer has not started.",
  currentStepEtaSeconds: 0,
  overallPercent: 0,
};

const idleRuntimeUpgradeProgress: RuntimeUpgradeProgress = {
  running: false,
  complete: false,
  failed: false,
  currentStep: "Idle",
  message: "",
  overallPercent: 0,
  fromVersion: null,
  toVersion: null,
};

const MAX_UPGRADE_AUTO_RETRIES = 2;

const idleHeadroomLearnStatus: HeadroomLearnStatus = {
  running: false,
  progressPercent: 0,
  summary: "Select a project to run headroom learn.",
  outputTail: [],
};

const idleHeadroomLearnPrereqStatus: HeadroomLearnPrereqStatus = {
  claudeCliAvailable: false,
  claudeCliPath: null,
  codexCliAvailable: false,
  codexCliPath: null,
  codexLoggedIn: false,
};

const SALES_CONTACT_URL =
  (import.meta.env.VITE_HEADROOM_SALES_CONTACT_URL ?? "").trim() ||
  "mailto:hello@example.com";
const CONTACT_FORM_URL = (
  import.meta.env.VITE_HEADROOM_CONTACT_FORM_URL ?? ""
).trim();
const SUPPORT_ISSUES_URL =
  "https://github.com/tarunag10/mac-ai-switchboard/issues";

type StartupPhase = "window" | "dashboard" | "bootstrap" | "runtime" | "ready";

const authCodeExpiryFallbackSeconds = 900;
const APP_UPDATE_BACKGROUND_INITIAL_DELAY_MS = 12_000;
const APP_UPDATE_BACKGROUND_CHECK_INTERVAL_MS = 60 * 60 * 1000;

async function loadDashboard(): Promise<DashboardState> {
  try {
    return await invoke<DashboardState>("get_dashboard_state");
  } catch {
    return mockDashboard;
  }
}

async function loadSavingsAttributionEvents(): Promise<SavingsAttributionEvent[]> {
  try {
    return await invoke<SavingsAttributionEvent[]>(
      "get_savings_attribution_events",
    );
  } catch {
    return [];
  }
}

function delay(ms: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

function renderConnectorLogo(clientId: string) {
  return <Sparkle className="client-logo__glyph" size={20} weight="duotone" />;
}

interface ProxyVerificationRow {
  clientId: string;
  name: string;
  state: "processing" | "waiting" | "testing" | "verified";
  message: string;
  oneClickSupported: boolean;
}

interface ConnectorSmokeTestResult {
  clientId: string;
  supported: boolean;
  launched: boolean;
  success: boolean;
  summary: string;
  stdoutTail: string;
  stderrTail: string;
}

export default function App() {
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
  const [pricingAudience, setPricingAudience] =
    useState<PricingAudience>("individual");
  const [billingPeriod, setBillingPeriod] = useState<BillingPeriod>("annual");
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
  const [showAllClaudeProjects, setShowAllClaudeProjects] = useState(false);
  const [selectedClaudeProjectPath, setSelectedClaudeProjectPath] = useState<
    string | null
  >(null);
  const [headroomLearnStatus, setHeadroomLearnStatus] =
    useState<HeadroomLearnStatus>(idleHeadroomLearnStatus);
  const [optimizeAppliedByProject, setOptimizeAppliedByProject] =
    useState<Record<string, AppliedPatterns> | null>(null);
  const [optimizeAppliedRefreshTick, setOptimizeAppliedRefreshTick] =
    useState(0);
  const previousHeadroomLearnRunningRef = useRef(false);
  const [headroomLearnBusy, setHeadroomLearnBusy] = useState(false);
  const [headroomLearnPrereq, setHeadroomLearnPrereq] =
    useState<HeadroomLearnPrereqStatus>(idleHeadroomLearnPrereqStatus);
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
  const [pricingStatus, setPricingStatus] =
    useState<HeadroomPricingStatus | null>(null);
  const [cachedPricing] = useState<CachedPricing>(() => readCachedPricing());
  const [pricingBusy, setPricingBusy] = useState(false);
  const [pricingError, setPricingError] = useState<string | null>(null);
  const pricingRefreshInFlightRef = useRef(false);
  const [authEmail, setAuthEmail] = useState("");
  const [authCode, setAuthCode] = useState("");
  const [authCodeRequestedFor, setAuthCodeRequestedFor] = useState<
    string | null
  >(null);
  const [authCodeExpirySeconds, setAuthCodeExpirySeconds] = useState(
    authCodeExpiryFallbackSeconds,
  );
  const [authRequestBusy, setAuthRequestBusy] = useState(false);
  const [authVerifyBusy, setAuthVerifyBusy] = useState(false);
  const [authFlowError, setAuthFlowError] = useState<string | null>(null);
  const [authFlowSuccess, setAuthFlowSuccess] = useState<string | null>(null);
  const [pendingUpgradePlanId, setPendingUpgradePlanId] =
    useState<UpgradePlanId | null>(null);
  const [showAllUpgradePlans, setShowAllUpgradePlans] = useState(false);
  const [checkoutPollingDeadline, setCheckoutPollingDeadline] = useState<
    number | null
  >(null);
  const desktopActivationSentRef = useRef(false);
  const autoDisabledByGateRef = useRef<Set<string>>(new Set());
  const [learnInstallCopyNotice, setLearnInstallCopyNotice] = useState<
    string | null
  >(null);

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
  const [upgradeActionBusy, setUpgradeActionBusy] =
    useState<UpgradePlanId | null>(null);
  const [upgradeActionError, setUpgradeActionError] = useState<string | null>(
    null,
  );
  const [pendingPlanChange, setPendingPlanChange] = useState<{
    fromTier: HeadroomSubscriptionTier;
    toTier: HeadroomSubscriptionTier;
    billingPeriod: BillingPeriod;
  } | null>(null);
  const [planChangeBusy, setPlanChangeBusy] = useState(false);
  const [planChangeError, setPlanChangeError] = useState<string | null>(null);
  const [reactivateBusy, setReactivateBusy] = useState(false);
  const [reactivateError, setReactivateError] = useState<string | null>(null);
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
  const authEmailValid = isValidEmailAddress(authEmail);
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
    setShowAllUpgradePlans(false);
    if (pricingAudience !== "individual") setBillingPeriod("annual");
  }, [pricingAudience]);

  useEffect(() => {
    if (!pricingStatus?.authenticated) {
      desktopActivationSentRef.current = false;
    }
  }, [pricingStatus?.authenticated]);

  useEffect(() => {
    if (!pricingStatus) return;
    writeCachedPricing(cachePricingStatus(pricingStatus));
  }, [pricingStatus]);

  useEffect(() => {
    const STORAGE_KEY = "headroom:lastNotifiedMismatchTier";
    if (localOnlyMode) {
      window.localStorage.removeItem(STORAGE_KEY);
      return;
    }
    const mismatch = pricingStatus?.tierMismatch;
    if (!mismatch) {
      window.localStorage.removeItem(STORAGE_KEY);
      return;
    }
    const rank: Record<string, number> = { pro: 1, max5x: 2, max20x: 3 };
    const previous = window.localStorage.getItem(STORAGE_KEY);
    // Notify on first detection and whenever the recommended tier climbs higher.
    if (
      previous !== null &&
      (rank[mismatch.recommendedTier] ?? 0) <= (rank[previous] ?? 0)
    ) {
      return;
    }
    const paidLabel = upgradePlanIntentLabel(mismatch.paidTier);
    const recommendedLabel = upgradePlanIntentLabel(mismatch.recommendedTier);
    const sourceLabel = tierRecommendationSourceLabel(
      mismatch.recommendedSource,
    );
    void invoke("show_notification", {
      title: "Upgrade your Headroom plan",
      body: `Your ${sourceLabel} usage needs the Switchboard ${recommendedLabel} plan, above your current ${paidLabel} plan. Upgrade to keep unlimited optimization.`,
    }).catch(() => {});
    window.localStorage.setItem(STORAGE_KEY, mismatch.recommendedTier);
  }, [
    localOnlyMode,
    pricingStatus?.tierMismatch?.recommendedTier,
    pricingStatus?.tierMismatch,
  ]);

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
    if (claudeProjects.length === 0) {
      setSelectedClaudeProjectPath(null);
      return;
    }

    setSelectedClaudeProjectPath((current) => {
      if (
        current &&
        claudeProjects.some((project) => project.projectPath === current)
      ) {
        return current;
      }
      return claudeProjects[0].projectPath;
    });
  }, [claudeProjects]);

  useEffect(() => {
    if (activeView !== "optimization") {
      return;
    }
    void Promise.all([refreshClaudeProjects(), refreshHeadroomLearnPrereq()]);
  }, [activeView]);

  useEffect(() => {
    if (activeView !== "optimization" || !trayWindowFocused) {
      return;
    }

    let active = true;
    const refreshLearnStatus = () => {
      void invoke<HeadroomLearnStatus>("get_headroom_learn_status", {
        projectPath: selectedClaudeProjectPath,
      })
        .then((status) => {
          if (active) {
            setHeadroomLearnStatus(status);
          }
        })
        .catch(() => {
          if (active) {
            setHeadroomLearnStatus((current) => ({
              ...current,
              running: false,
              summary: "Could not read headroom learn status.",
            }));
          }
        });
    };

    refreshLearnStatus();
    const interval = window.setInterval(
      refreshLearnStatus,
      headroomLearnStatus.running ? 900 : 3200,
    );
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [
    activeView,
    selectedClaudeProjectPath,
    headroomLearnStatus.running,
    trayWindowFocused,
  ]);

  useEffect(() => {
    if (activeView !== "upgrade") {
      setUpgradeActionError(null);
    }
  }, [activeView]);

  useEffect(() => {
    const wasRunning = previousHeadroomLearnRunningRef.current;
    previousHeadroomLearnRunningRef.current = headroomLearnStatus.running;

    if (!wasRunning || headroomLearnStatus.running) {
      return;
    }

    if (headroomLearnStatus.success && headroomLearnStatus.projectPath) {
      const completedAt =
        headroomLearnStatus.lastRunAt ??
        headroomLearnStatus.finishedAt ??
        new Date().toISOString();
      setClaudeProjects((current) =>
        current.map((project) =>
          project.projectPath === headroomLearnStatus.projectPath
            ? {
                ...project,
                lastLearnRanAt: completedAt,
                hasPersistedLearnings: true,
                activeDaysSinceLastLearn: 0,
              }
            : project,
        ),
      );
    }

    void refreshClaudeProjects();
  }, [
    headroomLearnStatus.finishedAt,
    headroomLearnStatus.lastRunAt,
    headroomLearnStatus.projectPath,
    headroomLearnStatus.running,
    headroomLearnStatus.success,
  ]);

  const claudeProjectPathsKey = claudeProjects
    .map((project) => project.projectPath)
    .sort()
    .join("\t");
  // Batched applied-patterns fetch: one IPC instead of one per OptimizePanel.
  useEffect(() => {
    if (activeView !== "optimization") {
      return;
    }
    const paths =
      claudeProjectPathsKey === "" ? [] : claudeProjectPathsKey.split("\t");
    if (paths.length === 0) {
      setOptimizeAppliedByProject({});
      return;
    }
    let active = true;
    invoke<Record<string, AppliedPatterns>>(
      "list_applied_patterns_for_projects",
      {
        projectPaths: paths,
      },
    )
      .then((result) => {
        if (!active) return;
        setOptimizeAppliedByProject(result);
      })
      .catch(() => {
        if (!active) return;
        setOptimizeAppliedByProject(null);
      });
    return () => {
      active = false;
    };
  }, [
    activeView,
    claudeProjectPathsKey,
    headroomLearnStatus.finishedAt,
    optimizeAppliedRefreshTick,
  ]);

  // Keep connectorPhase in sync with the connector enabled state from the backend.
  // Any supported connector (Claude Code, Codex, ...) being enabled counts as
  // "connected" — the request-count poller below is connector-agnostic.
  const anyConnectorEnabled = hasEnabledConnector(connectors);
  const plannedConnectorReadiness =
    summarizePlannedConnectorReadiness(connectors);

  // Which agents Headroom Learn should offer, driven by the enabled connectors.
  const claudeLearnEnabled = getClaudeConnector(connectors)?.enabled ?? false;
  const codexLearnEnabled = aggregateClientConnectors(connectors).some(
    (connector) => connector.clientId === "codex" && connector.enabled,
  );
  const learnBlurb =
    claudeLearnEnabled && codexLearnEnabled
      ? "Headroom learns from your Claude Code and Codex sessions. When an agent repeats a mistake, Headroom updates that agent's memory so it doesn't happen again."
      : codexLearnEnabled
        ? "Headroom learns from your Codex sessions. When Codex repeats a mistake, Headroom updates your ~/.codex/AGENTS.md and instructions.md so it doesn't happen again."
        : "Headroom helps Claude Code learn from experience. When Claude makes mistakes, Headroom automatically updates the project's MEMORY.md so they don't happen again. You can also ask Headroom to scan past sessions & add token-saving learnings to CLAUDE.md.";
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

  useEffect(() => {
    if (localOnlyMode || !hasTauriEventRuntime()) {
      return;
    }
    // Pricing status hits the remote Headroom API. When the tray is focused,
    // poll at 60s so fresh subscription/trial state is visible on demand.
    // When hidden, slow to 10 min — still fast enough for trial-expiry and
    // urgent notifications to fire, while cutting hourly API traffic by
    // ~90%. The launcher window never sets `trayWindowFocused` to false
    // (its focus listener isn't wired up), so it keeps the 60s cadence.
    const intervalMs = trayWindowFocused ? 60_000 : 600_000;
    void refreshPricingStatus();
    const interval = window.setInterval(() => {
      void refreshPricingStatus();
    }, intervalMs);
    return () => {
      window.clearInterval(interval);
    };
  }, [localOnlyMode, trayWindowFocused]);

  // headroom:// deep links from the backend trigger an immediate pricing
  // refresh — the typical case is Polar's checkout success page redirecting
  // to headroom://upgraded. Backend has already reconciled the runtime; this
  // just pulls the new status into UI state without waiting for the next
  // poll tick.
  useEffect(() => {
    if (localOnlyMode || !hasTauriEventRuntime()) {
      return;
    }
    let unlisten: (() => void) | undefined;
    void listen("pricing-refreshed", () => {
      void refreshPricingStatus();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [localOnlyMode]);

  // After the user opens a Polar checkout URL, poll pricing status every 5s
  // for up to 5 minutes so we can flip the UI back to "active" within seconds
  // of payment confirmation, instead of waiting out the 60s baseline cadence.
  // Auto-stops once subscription_active is observed or the window expires.
  useEffect(() => {
    if (localOnlyMode) {
      return;
    }
    if (checkoutPollingDeadline === null) return;
    if (Date.now() > checkoutPollingDeadline) {
      setCheckoutPollingDeadline(null);
      return;
    }
    const interval = window.setInterval(() => {
      if (Date.now() > checkoutPollingDeadline) {
        setCheckoutPollingDeadline(null);
        return;
      }
      void refreshPricingStatus();
    }, 5_000);
    return () => {
      window.clearInterval(interval);
    };
  }, [checkoutPollingDeadline, localOnlyMode]);

  // Stop the aggressive checkout poll the moment we observe a live
  // subscription. Saves traffic and stops competing with the 60s cadence.
  useEffect(() => {
    if (localOnlyMode) {
      return;
    }
    if (
      checkoutPollingDeadline !== null &&
      pricingStatus?.account?.subscriptionActive
    ) {
      setCheckoutPollingDeadline(null);
    }
  }, [
    checkoutPollingDeadline,
    localOnlyMode,
    pricingStatus?.account?.subscriptionActive,
  ]);

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

  useEffect(() => {
    if (localOnlyMode) {
      return;
    }
    const runtimeHealthyNow =
      runtimeStatus?.running === true &&
      runtimeStatus?.proxyReachable === true &&
      connectorPhase === "healthy";
    if (
      !pricingStatus?.authenticated ||
      !runtimeHealthyNow ||
      desktopActivationSentRef.current
    ) {
      return;
    }
    desktopActivationSentRef.current = true;
    void invoke<HeadroomPricingStatus>("activate_headroom_account")
      .then((status) => setPricingStatus(status))
      .catch(() => {
        desktopActivationSentRef.current = false;
      });
  }, [
    connectorPhase,
    localOnlyMode,
    pricingStatus?.authenticated,
    runtimeStatus?.proxyReachable,
    runtimeStatus?.running,
  ]);

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
    try {
      setConnectorsError(null);
      const items = await invoke<ClientConnectorStatus[]>(
        "get_client_connectors",
      );
      applyConnectorsIfChanged(items);
    } catch (error) {
      setConnectorsError(
        error instanceof Error
          ? error.message
          : "Could not load connector status.",
      );
    }
  }

  async function refreshSwitchboardState() {
    try {
      const state = await invoke<SwitchboardState>("get_switchboard_state");
      applySwitchboardStateIfChanged(state);
      applyRuntimeStatusIfChanged(state.runtime);
      applyConnectorsIfChanged(state.clients);
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

  async function refreshPricingStatus() {
    if (localOnlyMode) {
      setPricingBusy(false);
      setPricingError(null);
      return;
    }
    if (pricingRefreshInFlightRef.current) {
      return;
    }
    pricingRefreshInFlightRef.current = true;
    setPricingBusy(true);
    try {
      const status = await invoke<HeadroomPricingStatus>(
        "get_headroom_pricing_status",
      );
      setPricingStatus(status);
      void maybeFireTrialNotifications(status);
      void maybeFireUrgentPricingNotifications(status, { localOnlyMode });
      setPricingError(null);
    } catch (error) {
      setPricingError(
        error instanceof Error
          ? error.message
          : "Could not load pricing status.",
      );
    } finally {
      pricingRefreshInFlightRef.current = false;
      setPricingBusy(false);
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

  async function refreshHeadroomLearnPrereq(force = false) {
    try {
      const status = await invoke<HeadroomLearnPrereqStatus>(
        "get_headroom_learn_prereq_status",
        {
          force,
        },
      );
      setHeadroomLearnPrereq(status);
    } catch {
      setHeadroomLearnPrereq(idleHeadroomLearnPrereqStatus);
    }
  }

  async function copyLearnInstallCommand(command: string) {
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(command);
      setLearnInstallCopyNotice("Copied install command.");
      window.setTimeout(() => setLearnInstallCopyNotice(null), 2000);
    } catch {
      setLearnInstallCopyNotice(
        "Copy failed. Command remains visible below.",
      );
      window.setTimeout(() => setLearnInstallCopyNotice(null), 3000);
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

  async function runHeadroomLearn(
    agent: "claude" | "codex",
    projectPath?: string,
  ) {
    if (runtimeStatus?.headroomLearnSupported === false) {
      setHeadroomLearnStatus((current) => ({
        ...current,
        running: false,
        summary: "Headroom Learn is unavailable on this platform.",
        error:
          runtimeStatus.headroomLearnDisabledReason ??
          "Headroom Learn is unavailable on this platform.",
      }));
      return;
    }

    // Codex isn't project-organized, so it shares a stable run key.
    const runKey = agent === "codex" ? "codex" : (projectPath ?? "");
    const displayName =
      agent === "codex"
        ? "Codex sessions"
        : (claudeProjects.find((project) => project.projectPath === projectPath)
            ?.displayName ??
          projectPath ??
          "");
    const startupSummary = `Running headroom learn for ${displayName}.`;
    setHeadroomLearnBusy(true);
    setHeadroomLearnStatus((current) => ({
      ...current,
      running: true,
      projectPath: runKey,
      projectDisplayName: displayName,
      startedAt: new Date().toISOString(),
      finishedAt: null,
      progressPercent: Math.max(8, current.progressPercent || 0),
      summary: startupSummary,
      success: null,
      error: null,
    }));
    try {
      await invoke("start_headroom_learn", {
        agent,
        projectPath: projectPath ?? null,
      });
      for (const waitMs of [180, 350, 650, 900, 1200, 1800, 2400]) {
        await delay(waitMs);
        const status = await invoke<HeadroomLearnStatus>(
          "get_headroom_learn_status",
          {
            projectPath: runKey,
          },
        );
        setHeadroomLearnStatus(status);
        if (!status.running) {
          break;
        }
      }
    } catch (error) {
      setHeadroomLearnStatus((current) => ({
        ...current,
        running: false,
        summary: "headroom learn could not be started.",
        error:
          error instanceof Error
            ? error.message
            : "Failed to start headroom learn.",
      }));
    } finally {
      setHeadroomLearnBusy(false);
    }
  }

  async function handleRunHeadroomLearn(
    agent: "claude" | "codex",
    projectPath?: string,
  ) {
    if (agent === "claude" && projectPath) {
      setSelectedClaudeProjectPath(projectPath);
    }
    try {
      const status = await invoke<HeadroomLearnPrereqStatus>(
        "get_headroom_learn_prereq_status",
      );
      setHeadroomLearnPrereq(status);
      const ready =
        agent === "codex"
          ? status.codexCliAvailable && status.codexLoggedIn
          : status.claudeCliAvailable;
      if (!ready) {
        return;
      }
    } catch {
      setHeadroomLearnPrereq(idleHeadroomLearnPrereqStatus);
      return;
    }
    await runHeadroomLearn(agent, projectPath);
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

  async function prepareRepoMemoryMcp() {
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
    } catch (error) {
      setAddonError(
        error instanceof Error
          ? error.message
          : "Repo Memory MCP could not be prepared.",
      );
    } finally {
      setAddonBusyId(null);
      setAddonBusyLabel(null);
    }
  }

  async function setRepoMemoryMcpActive(active: boolean) {
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
    } catch (error) {
      setAddonError(
        error instanceof Error
          ? error.message
          : active
            ? "Repo Memory MCP could not be started."
            : "Repo Memory MCP could not be stopped.",
      );
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

  function openUpgradeAuthView(planId: UpgradePlanId | null = null) {
    setActiveView(safeTrayViewForMode("upgradeAuth", localOnlyMode));
    setPendingUpgradePlanId(planId);
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
  }

  function resetUpgradeAuthStep() {
    setAuthCode("");
    setAuthCodeRequestedFor(null);
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
  }

  async function handleRequestAuthCode() {
    if (!authEmailValid) {
      setAuthFlowError("Enter a valid email address.");
      return;
    }
    setAuthRequestBusy(true);
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
    try {
      const result = await invoke<HeadroomAuthCodeRequest>(
        "request_headroom_auth_code",
        {
          email: authEmail.trim(),
        },
      );
      setAuthCodeRequestedFor(result.email);
      setAuthCodeExpirySeconds(result.expiresInSeconds);
      setAuthFlowSuccess(`We sent a sign-in code to ${result.email}.`);
    } catch (error) {
      setAuthFlowError(
        describeInvokeError(error, "Could not send sign-in code."),
      );
    } finally {
      setAuthRequestBusy(false);
    }
  }

  async function handleVerifyAuthCode() {
    if (!authEmailValid) {
      setAuthFlowError("Enter a valid email address.");
      return;
    }
    if (!authCode.trim()) {
      setAuthFlowError("Enter the authentication code from your email.");
      return;
    }
    setAuthVerifyBusy(true);
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
    try {
      const status = await invoke<HeadroomPricingStatus>(
        "verify_headroom_auth_code",
        {
          email: authEmail.trim(),
          code: authCode.trim(),
          inviteCode: null,
        },
      );
      setPricingStatus(status);
      setAuthCode("");
      setAuthCodeRequestedFor(null);
      setAuthFlowSuccess("Switchboard account connected.");
      setPendingUpgradePlanId(null);
      setActiveView(safeTrayViewForMode("upgrade", localOnlyMode));
      await refreshConnectors();
    } catch (error) {
      setAuthFlowError(
        describeInvokeError(error, "Could not verify sign-in code."),
      );
    } finally {
      setAuthVerifyBusy(false);
    }
  }

  async function handleSignOutHeadroomAccount() {
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
    try {
      await invoke("sign_out_headroom_account");
      setPricingStatus(
        await invoke<HeadroomPricingStatus>("get_headroom_pricing_status"),
      );
      setAuthCode("");
      setAuthCodeRequestedFor(null);
      setAuthFlowSuccess("Signed out of Headroom.");
      setPendingUpgradePlanId(null);
    } catch (error) {
      setAuthFlowError(
        error instanceof Error
          ? error.message
          : "Could not sign out of Headroom.",
      );
    }
  }

  async function openLearnInstallDocsLink() {
    try {
      await openExternalLink(CLAUDE_CODE_INSTALL_DOCS_URL);
    } catch (error) {
      setLearnInstallCopyNotice(
        error instanceof Error
          ? error.message
          : "Could not open the install guide.",
      );
      window.setTimeout(() => setLearnInstallCopyNotice(null), 3000);
    }
  }

  async function handleUpgradeAction(planId: UpgradePlanId) {
    const activeHeadroomPlanId = pricingStatus?.account?.subscriptionActive
      ? (pricingStatus.account.subscriptionTier ?? null)
      : null;
    const action = (() => {
      switch (planId) {
        case "free":
          return {
            kind: activeHeadroomPlanId
              ? ("billing_portal" as const)
              : ("internal" as const),
          };
        case "pro":
        case "max5x":
        case "max20x": {
          if (activeHeadroomPlanId === planId)
            return { kind: "internal" as const };
          // Polar prorates the product swap with the existing discount applied,
          // so every plan change on an active subscription uses the PATCH path.
          if (activeHeadroomPlanId) {
            return { kind: "change_plan" as const };
          }
          return { kind: "checkout" as const };
        }
        case "team":
          return {
            kind: "external" as const,
            url: SALES_CONTACT_URL,
            missing:
              "Set VITE_HEADROOM_SALES_CONTACT_URL to enable Team sales inquiries.",
          };
        case "enterprise":
          return {
            kind: "external" as const,
            url: SALES_CONTACT_URL,
            missing:
              "Set VITE_HEADROOM_SALES_CONTACT_URL to enable Enterprise contact.",
          };
        default:
          return null;
      }
    })();

    if (!action) {
      return;
    }

    trackAnalyticsEvent("upgrade_button_clicked", {
      plan_id: planId,
      action_kind: action.kind,
      email:
        pricingStatus?.account?.email ??
        pricingStatus?.claude?.email ??
        undefined,
    });

    if (action.kind === "internal") {
      setUpgradeActionError(null);
      setActiveView("home");
      return;
    }

    if (!pricingStatus?.authenticated) {
      openUpgradeAuthView(planId);
      return;
    }

    if (action.kind === "change_plan") {
      const fromTier = pricingStatus?.account?.subscriptionTier;
      if (!fromTier) return;
      setPlanChangeError(null);
      setPendingPlanChange({
        fromTier,
        toTier: planId as HeadroomSubscriptionTier,
        billingPeriod,
      });
      return;
    }

    if (action.kind === "checkout") {
      setUpgradeActionBusy(planId);
      setUpgradeActionError(null);

      try {
        const url = await invoke<string>("create_headroom_checkout_session", {
          subscriptionTier: planId,
          billingPeriod,
        });
        await openExternalLink(url);
        // Aggressive poll for the next 5 minutes so the moment Polar marks
        // the subscription active we surface "Headroom is back online" without
        // making the user wait out the normal 60s pricing-refresh cadence.
        setCheckoutPollingDeadline(Date.now() + 5 * 60_000);
      } catch (error) {
        setUpgradeActionError(
          error instanceof Error
            ? error.message
            : typeof error === "string"
              ? error
              : "Could not start checkout.",
        );
      } finally {
        setUpgradeActionBusy(null);
      }
      return;
    }

    if (action.kind === "billing_portal") {
      setUpgradeActionBusy(planId);
      setUpgradeActionError(null);

      try {
        // Deep-link to the user's subscription page so they land one click
        // away from "Change plan" instead of at the portal root.
        const url = await invoke<string>("get_headroom_billing_portal_url", {
          target: "subscription",
        });
        await openExternalLink(url);
      } catch (error) {
        setUpgradeActionError(
          error instanceof Error
            ? error.message
            : typeof error === "string"
              ? error
              : "Could not open billing portal.",
        );
      } finally {
        setUpgradeActionBusy(null);
      }
      return;
    }

    if (!action.url) {
      setUpgradeActionError(
        action.missing ?? "Could not open the selected plan link.",
      );
      return;
    }

    setUpgradeActionBusy(planId);
    setUpgradeActionError(null);

    try {
      await openExternalLink(action.url);
    } catch (error) {
      setUpgradeActionError(
        error instanceof Error
          ? error.message
          : "Could not open the selected plan link.",
      );
    } finally {
      setUpgradeActionBusy(null);
    }
  }

  async function confirmPlanChange() {
    if (!pendingPlanChange) return;
    setPlanChangeBusy(true);
    setPlanChangeError(null);
    try {
      await invoke("change_headroom_subscription_plan", {
        subscriptionTier: pendingPlanChange.toTier,
        billingPeriod: pendingPlanChange.billingPeriod,
      });
      await refreshPricingStatus();
      setPendingPlanChange(null);
      setActiveView("home");
    } catch (error) {
      setPlanChangeError(
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Could not change subscription plan.",
      );
    } finally {
      setPlanChangeBusy(false);
    }
  }

  function cancelPlanChange() {
    if (planChangeBusy) return;
    setPendingPlanChange(null);
    setPlanChangeError(null);
  }

  async function handleReactivateSubscription() {
    if (reactivateBusy) return;
    setReactivateBusy(true);
    setReactivateError(null);
    try {
      await invoke("reactivate_headroom_subscription");
      await refreshPricingStatus();
    } catch (error) {
      setReactivateError(
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Could not reactivate subscription.",
      );
    } finally {
      setReactivateBusy(false);
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
    let fresh = connectors;
    try {
      fresh = await invoke<ClientConnectorStatus[]>("get_client_connectors");
      applyConnectorsIfChanged(fresh);
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
      setProxyVerificationRows((current) =>
        current.map((item) =>
          item.clientId === row.clientId
            ? {
                ...item,
                state: result.success ? "processing" : "waiting",
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
      setConnectorSmokeBusyId(null);
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
  const canDismissUpgradeFailure =
    upgradeFailure !== null &&
    upgradeFailure.rollbackRestored &&
    runtimeStatus?.proxyReachable === true;

  const upgradeOverlay = (
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
                            url: buildUpgradeIssueUrl(
                              SUPPORT_ISSUES_URL,
                              upgradeFailure,
                            ),
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
      <LauncherShell
        shellClassName="intro-shell intro-shell--post-install"
        spinnerClassName="intro-shell__spinner intro-shell__spinner--post-install"
        copyClassName="intro-shell__copy intro-shell__copy--post-install"
        onMouseDown={handleLauncherSurfaceMouseDown}
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
                onClick={() => void handleFirstLaunchContinue()}
              >
                Continue with previous version
              </button>
              {upgradeFailure.failurePhase === "boot_validation" && (
                <button
                  type="button"
                  className="secondary-button"
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
                              url: buildUpgradeIssueUrl(
                                SUPPORT_ISSUES_URL,
                                upgradeFailure,
                              ),
                    }).catch(() => {})
                  }
                >
                  Report issue
                </button>
              )}
            </div>
          </>
        ) : null}
      </LauncherShell>
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
    const sortedLauncherConnectors = sortClientConnectors(launcherConnectors);
    const availableConnectors = sortedLauncherConnectors.filter((connector) =>
      canConfigureConnectorWithoutDetection(connector),
    );
    const unavailableConnectors = sortedLauncherConnectors.filter(
      (connector) => !canConfigureConnectorWithoutDetection(connector),
    );
    const enabledConnectorCount = launcherConnectors.filter(
      (connector) => connector.enabled,
    ).length;
    const requireSelection = availableConnectors.length > 0;

    return (
      <LauncherShell
        shellClassName="intro-shell intro-shell--post-install intro-shell--client-setup"
        spinnerClassName="intro-shell__spinner intro-shell__spinner--post-install"
        copyClassName="intro-shell__copy intro-shell__copy--post-install"
        onMouseDown={handleLauncherSurfaceMouseDown}
        version={appSemver}
      >
        <div className="post-install__lead">
          <h1>Connect your coding tools</h1>
          <p>Toggle each tool to automatically route it through Headroom.</p>
          <div className="connector-list">
            {availableConnectors.map((connector) => {
              const unavailableReason =
                getConnectorUnavailableReason(connector);
              const detectionWarning = getConnectorDetectionWarning(connector);
              const supportWarning = getConnectorSupportWarning(connector);
              const needsRestart = connector.enabled && !connector.verified;
              const plannedConnector = getPlannedConnector(connector.clientId);
              return (
                <article className="connector-item" key={connector.clientId}>
                  <div>
                    <h3>
                      <span className="client-logo" aria-hidden="true">
                        {renderConnectorLogo(connector.clientId)}
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
                        aria-expanded={
                          openConnectorHelpId === connector.clientId
                        }
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
                const unavailableReason =
                  getConnectorUnavailableReason(connector);
                const supportWarning = getConnectorSupportWarning(connector);
                return (
                  <article
                    className="connector-item is-unavailable"
                    key={connector.clientId}
                  >
                    <div>
                      <h3>
                        <span className="client-logo" aria-hidden="true">
                          {renderConnectorLogo(connector.clientId)}
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
              connectorsBusy ||
              (requireSelection && enabledConnectorCount === 0)
            }
            onClick={() => {
              void beginProxyVerificationStep();
            }}
            type="button"
          >
            Continue
          </button>
        </div>
      </LauncherShell>
    );
  }

  if (windowLabel === "launcher" && launcherStage === "proxy_verify") {
    const hasEnabledApps = proxyVerificationRows.length > 0;
    const hasOneClickTests =
      hasPendingOneClickProxyVerification(proxyVerificationRows);
    const allVerified =
      hasEnabledApps &&
      proxyVerificationRows.every((row) => row.state === "verified");

    return (
      <LauncherShell
        shellClassName="intro-shell intro-shell--post-install"
        spinnerClassName="intro-shell__spinner intro-shell__spinner--post-install"
        copyClassName="intro-shell__copy intro-shell__copy--post-install"
        onMouseDown={handleLauncherSurfaceMouseDown}
        version={appSemver}
      >
        <div className="post-install__lead">
          <h1>Test your setup</h1>
          <p>
            Send automatic test prompts for Claude Code and Codex, then watch
            for a verified badge. For tools without automatic tests, open the
            tool and send one tiny prompt. Restart tools that were already open
            so they reload the managed config.
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
                        {renderConnectorLogo(row.clientId)}
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
            onClick={() => {
              setLauncherStage("client_setup");
            }}
            type="button"
          >
            Back
          </button>
          <button
            className="primary-button primary-button--large primary-button--success"
            onClick={() => {
              void invoke("complete_setup_wizard");
              setLauncherStage("post_install");
            }}
            type="button"
          >
            Continue
          </button>
        </div>
      </LauncherShell>
    );
  }

  if (windowLabel === "launcher" && launcherStage === "post_install") {
    return (
      <LauncherShell
        shellClassName="intro-shell intro-shell--post-install"
        spinnerClassName="intro-shell__spinner intro-shell__spinner--post-install"
        copyClassName="intro-shell__copy intro-shell__copy--post-install"
        onMouseDown={handleLauncherSurfaceMouseDown}
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
              Use Test setup to send a first prompt automatically where
              supported, or send your first prompt from a connected tool.
              Switchboard will route through the local Headroom engine and
              track savings automatically.
            </p>
          ) : (
            <>
              <p>
                Switchboard will trim prompt bloat whenever you use
                enabled clients such as Claude Code or Codex.
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
            onClick={() => {
              void beginProxyVerificationStep();
            }}
            type="button"
          >
            Back
          </button>
          <button
            className="primary-button primary-button--large primary-button--success"
            onClick={() => triggerHide()}
            type="button"
          >
            Get started
          </button>
          <p>Headroom stays active in your menu bar while you work.</p>
        </div>
      </LauncherShell>
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
  const headroomLearnSupported =
    runtimeStatus?.headroomLearnSupported !== false;
  const headroomLearnDisabledReason =
    runtimeStatus?.headroomLearnDisabledReason ??
    "Headroom Learn is unavailable on this platform.";

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
  const proxyListenerAddress =
    runtimeStatus?.proxyBindAddress ?? "127.0.0.1:6767";
  const proxyListenerDetail =
    runtimeStatus?.proxyReachable === true
      ? `${proxyListenerAddress} is accepting loopback traffic. ${runtimeStatus?.proxyAuthDetail ?? "The listener is local-only."}`
      : runtimeStatus?.paused
        ? `${proxyListenerAddress} is intentionally stopped while the Headroom engine is paused.`
        : `${proxyListenerAddress} is not accepting traffic.`;
  const backendStatus = runtimeStatus?.backendStatus ?? null;
  const backendPortDetail = backendStatus
    ? backendStatus.port === backendStatus.defaultPort
      ? `${backendStatus.bindAddress} is the default internal Headroom backend port.`
      : `${backendStatus.bindAddress} is the selected fallback internal backend port; ${backendStatus.defaultPort} was unavailable.`
    : "Internal backend port evidence is unavailable.";
  const backendPortStatus =
    backendStatus?.reachable === true
      ? "Reachable"
      : runtimeStatus?.paused
        ? "Paused"
        : "Unreachable";
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
    service: runtimeStatus?.repoMemoryMcpService,
  });
  const launchAgentStatus = runtimeStatus?.launchAgentStatus ?? null;
  const launchAgentInstalled = launchAgentStatus?.installed === true;
  const legacyLaunchAgentInstalled =
    launchAgentStatus?.legacyInstalled === true;
  const launchAgentLoaded = launchAgentStatus?.loaded === true;
  const legacyLaunchAgentLoaded = launchAgentStatus?.legacyLoaded === true;
  const launchAgentDetail = legacyLaunchAgentInstalled
    ? `Legacy Headroom.plist exists at ${launchAgentStatus?.legacyPath ?? "~/Library/LaunchAgents/Headroom.plist"}. ${launchAgentStatus?.legacyLoadDetail ?? "Legacy launchd load state is unknown."} Run Doctor cleanup or uninstall to remove it.`
    : launchAgentInstalled
      ? `Launch at login plist exists at ${launchAgentStatus?.path ?? "~/Library/LaunchAgents/com.tarunagarwal.mac-ai-switchboard.plist"}. ${launchAgentStatus?.loadDetail ?? "launchd load state is unknown."}`
      : `No app-managed launch-at-login plist found. ${launchAgentStatus?.loadDetail ?? "launchd load state is unknown."}`;
  const switchboardRoutingConnectors =
    switchboardState?.clients ?? switchboardConnectors;
  const codexRoutingConnector = switchboardRoutingConnectors.find(
    (connector) => connector.clientId === "codex",
  );
  const claudeRoutingConnector = switchboardRoutingConnectors.find(
    (connector) => connector.clientId === "claude_code",
  );
  const additionalManagedRoutingConnectors = switchboardRoutingConnectors.filter(
    (connector) =>
      connector.installed === true &&
      connectorSupportsAutomaticSetup(connector) &&
      !["codex", "claude_code"].includes(connector.clientId),
  );
  const connectorRoutingRow = (
    label: string,
    connector: ClientConnectorStatus | undefined,
  ) => {
    const configured = connector?.enabled === true;
    const verified = connector?.verified === true;
    const canRepairManaged =
      connector?.installed === true &&
      connectorSupportsAutomaticSetup(connector) &&
      (!configured || !verified);
    const managedRepairAction =
      connector?.clientId === "codex"
        ? "repair_codex_setup"
        : connector?.clientId
          ? `repair_client_setup:${connector.clientId}`
          : "repair_client_setups";
    const actionLabel = canRepairManaged
        ? connector?.clientId === "codex"
          ? "Repair Codex"
          : "Auto-fix app-managed setup"
        : undefined;
    const actionDisabled = canRepairManaged ? doctorRepairBusy !== null : undefined;
    const onAction = canRepairManaged
      ? () => void handleDoctorRepair(managedRepairAction)
      : undefined;
    return {
      label,
      status: configured
        ? verified
          ? "Verified"
          : "Needs test"
        : canRepairManaged
          ? "Repair ready"
          : "Direct",
      detail: connector?.installed
        ? configured
          ? verified
            ? `${connector.name} is routed through Headroom and verified.`
            : `${connector.name} routing is configured; send a test prompt from Connectors.`
          : canRepairManaged
            ? `${connector.name} routing is repair ready. Use ${actionLabel} to re-apply reversible app-managed setup and verify routing evidence.`
            : `${connector.name} is detected but not routed.`
        : `${label.replace(" routing", "")} is not detected on this Mac.`,
      actionLabel,
      actionBusyLabel:
        canRepairManaged && doctorRepairBusy === managedRepairAction
          ? "Repairing"
          : undefined,
      actionDisabled,
      onAction,
    };
  };
  const enabledConnectorVerifications = switchboardRoutingConnectors
    .filter((connector) => connector.enabled)
    .map((connector) => connector.setupVerification)
    .filter((verification): verification is NonNullable<typeof verification> =>
      Boolean(verification),
    );
  const managedShellBlockVerified = enabledConnectorVerifications.some(
    (verification) =>
      verification.checks.some((check) =>
        /managed shell block|shell profiles/i.test(check),
      ),
  );
  const managedShellBlockMissing = enabledConnectorVerifications.some(
    (verification) =>
      verification.failures.some((failure) =>
        /shell profiles|shell blocks/i.test(failure),
      ),
  );
  const codexProviderVerified =
    codexRoutingConnector?.setupVerification?.checks.some((check) =>
      /provider block/i.test(check),
    ) === true;
  const codexProviderMissing =
    codexRoutingConnector?.setupVerification?.failures.some((failure) =>
      /provider block/i.test(failure),
    ) === true;
  const switchboardInspectorRows = [
    {
      label: "Proxy listener",
      status:
        runtimeStatus?.proxyReachable === true
          ? "Reachable"
          : runtimeStatus?.paused
            ? "Paused"
            : "Unreachable",
      detail: proxyListenerDetail,
    },
    {
      label: "Backend port",
      status: backendPortStatus,
      detail: backendPortDetail,
    },
    connectorRoutingRow("Codex routing", codexRoutingConnector),
    connectorRoutingRow("Claude routing", claudeRoutingConnector),
    ...additionalManagedRoutingConnectors.map((connector) =>
      connectorRoutingRow(`${connector.name} routing`, connector),
    ),
    {
      label: "Client routing",
      status:
        (switchboardState?.enabledClients ?? enabledSwitchboardConnectors)
          .length > 0
          ? "Managed"
          : "Direct",
      detail: switchboardHeadroomLabel,
    },
    {
      label: "Managed shell blocks",
      status: managedShellBlockVerified
        ? "Verified"
        : managedShellBlockMissing
          ? "Missing"
          : "No proof",
      detail: managedShellBlockVerified
        ? "Connector verification found managed shell routing blocks."
        : managedShellBlockMissing
          ? "Connector verification reported missing shell routing blocks."
          : "No enabled connector has reported shell-block verification yet.",
    },
    {
      label: "Codex provider block",
      status: codexProviderVerified
        ? "Verified"
        : codexProviderMissing
          ? "Missing"
          : codexRoutingConnector?.enabled
            ? "No proof"
            : "Direct",
      detail: codexProviderVerified
        ? "Connector verification found the Headroom-managed provider block in ~/.codex/config.toml."
        : codexProviderMissing
          ? "Connector verification reported the Codex provider block is missing."
          : codexRoutingConnector?.enabled
            ? "Codex is enabled, but provider-block verification has not reported proof yet."
            : "Codex provider routing is repair ready. Use the Codex routing repair-ready row to re-apply the managed provider block.",
    },
    {
      label: "Shell export",
      status: runtimeStatus?.rtk.pathConfigured ? "Configured" : "Not configured",
      detail: runtimeStatus?.rtk.pathConfigured
        ? "Managed RTK PATH export is present."
        : "Managed RTK PATH export is not active.",
    },
    {
      label: "RTK shell hook",
      status: runtimeStatus?.rtk.hookConfigured ? "Configured" : "Not configured",
      detail: runtimeStatus?.rtk.hookConfigured
        ? "Managed RTK command-rewrite hook is present."
        : runtimeStatus?.rtk.installed
          ? "RTK is installed, but the managed shell hook is not active."
          : "RTK shell hook is not installed.",
    },
    {
      label: "Headroom MCP",
      status:
        runtimeStatus?.mcpConfigured === true
          ? "Configured"
          : runtimeStatus?.mcpConfigured === false
            ? "Not configured"
            : "Unknown",
      detail:
        runtimeStatus?.mcpConfigured === true
          ? "Claude MCP config includes the local Headroom server."
          : runtimeStatus?.mcpConfigured === false
            ? (runtimeStatus.mcpError ??
              "Claude MCP config does not include the local Headroom server.")
            : "Headroom MCP configuration has not been checked yet.",
    },
    {
      ...repoMemoryMcpInspectorRow({
        configured: runtimeStatus?.repoMemoryMcpConfigured,
        error: runtimeStatus?.repoMemoryMcpError,
        active: runtimeStatus?.repoMemoryMcpActive,
        lastStartedAt: runtimeStatus?.repoMemoryMcpLastStartedAt,
        lastCheckedAt: runtimeStatus?.repoMemoryMcpLastCheckedAt,
        supervisionStatus: runtimeStatus?.repoMemoryMcpSupervisionStatus,
        service: runtimeStatus?.repoMemoryMcpService,
      }),
      actionLabel:
        repoMemoryLifecycle.state === "active"
          ? "Stop MCP"
          : runtimeStatus?.repoMemoryMcpConfigured === true
            ? "Start MCP"
            : "Prepare MCP",
      actionBusyLabel:
        addonBusyId === "repo-memory" ? (addonBusyLabel ?? "Working") : undefined,
      actionDisabled: addonBusyId !== null,
      onAction:
        repoMemoryLifecycle.state === "active"
          ? () => void setRepoMemoryMcpActive(false)
          : runtimeStatus?.repoMemoryMcpConfigured === true
            ? () => void setRepoMemoryMcpActive(true)
            : () => void prepareRepoMemoryMcp(),
    },
    {
      label: "Launch at login",
      status: legacyLaunchAgentInstalled || legacyLaunchAgentLoaded
        ? "Legacy found"
        : launchAgentLoaded
          ? "Loaded"
          : launchAgentInstalled
            ? "Installed"
          : "Not installed",
      detail: launchAgentDetail,
    },
  ];
  const switchboardLocalOnly = switchboardState?.localOnly ?? localOnlyMode;
  const switchboardRemoteServicesEnabled =
    switchboardState?.remoteServicesEnabled ?? !switchboardLocalOnly;
  const sortedClaudeProjects = [...claudeProjects].sort((left, right) => {
    const leftTime = Date.parse(left.lastWorkedAt);
    const rightTime = Date.parse(right.lastWorkedAt);
    return (
      (Number.isNaN(rightTime) ? 0 : rightTime) -
      (Number.isNaN(leftTime) ? 0 : leftTime)
    );
  });
  const pinnedClaudeProject =
    !showAllClaudeProjects && headroomLearnStatus.projectPath
      ? (sortedClaudeProjects.find(
          (project) => project.projectPath === headroomLearnStatus.projectPath,
        ) ?? null)
      : null;
  const visibleClaudeProjects = (() => {
    if (showAllClaudeProjects) {
      return sortedClaudeProjects;
    }

    const topProjects = sortedClaudeProjects.slice(0, 3);
    if (
      !pinnedClaudeProject ||
      topProjects.some(
        (project) => project.projectPath === pinnedClaudeProject.projectPath,
      )
    ) {
      return topProjects;
    }
    return [...topProjects, pinnedClaudeProject];
  })();
  const hiddenClaudeProjectsCount =
    sortedClaudeProjects.length - visibleClaudeProjects.length;
  const trialDaysRemaining = formatRemainingDays(
    pricingStatus?.account?.trialEndsAt,
  );
  const localGraceHoursRemaining = (() => {
    const target = pricingStatus?.localGraceEndsAt
      ? new Date(pricingStatus.localGraceEndsAt).getTime()
      : Number.NaN;
    if (Number.isNaN(target)) {
      return null;
    }
    return Math.max(0, Math.ceil((target - Date.now()) / 3_600_000));
  })();
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
  const accountDisplayEmail = (() => {
    const enteredEmail = authEmail.trim();
    return (
      pricingStatus?.account?.email ??
      (enteredEmail || pricingStatus?.claude.email || "unknown email")
    );
  })();
  const accountPlanName = (() => {
    if (!pricingStatus?.authenticated) {
      return null;
    }
    if (!pricingStatus.account) {
      return pricingStatus.accountSyncError
        ? "Plan unavailable"
        : "Syncing plan...";
    }
    if (pricingStatus.account.subscriptionActive) {
      return subscriptionTierLabel(pricingStatus.account.subscriptionTier);
    }
    if (pricingStatus.account.trialActive) {
      if (trialDaysRemaining != null) {
        return `${trialDaysRemaining} day${trialDaysRemaining === 1 ? "" : "s"} left in trial`;
      }
      return "7-day trial";
    }
    return "Trial expired";
  })();
  const upgradeTrialCallout = (() => {
    if (pricingBusy && !pricingStatus) {
      return {
        tone: "neutral" as const,
        message: "Loading your Switchboard access...",
      };
    }
    if (!pricingStatus) {
      return {
        tone: "neutral" as const,
        message: "Headroom pricing status is unavailable right now.",
      };
    }
    if (!pricingStatus.authenticated) {
      if (!pricingStatus.localGraceActive) {
        return {
          tone: "expired" as const,
          message:
            "Your 72-hour Switchboard access expired. Create an account to extend to 7 days.",
          actionLabel: "Sign up",
          onAction: openUpgradeAuthView,
        };
      }
      const hoursLabel =
        localGraceHoursRemaining != null
          ? `${localGraceHoursRemaining} hour${localGraceHoursRemaining === 1 ? "" : "s"}`
          : "72 hours";
      return {
        tone: "warning" as const,
        message: `${hoursLabel} left in your 72-hour trial. Create an account to extend trial to 7 days.`,
        actionLabel: "Sign up",
        onAction: openUpgradeAuthView,
      };
    }
    if (!pricingStatus.account) {
      return {
        tone: "neutral" as const,
        message:
          pricingStatus.accountSyncError ??
          "Switchboard account connected. Syncing your trial and plan details...",
      };
    }
    if (pricingStatus.account?.subscriptionActive) {
      return {
        tone: "healthy" as const,
        message: `${subscriptionTierLabel(pricingStatus.account.subscriptionTier)} is active. Headroom can keep optimizing without limits.`,
      };
    }
    if (pricingStatus.account?.trialActive) {
      const daysLabel =
        trialDaysRemaining != null
          ? `${trialDaysRemaining} day${trialDaysRemaining === 1 ? "" : "s"}`
          : "7 days";
      return {
        tone: "warning" as const,
        message: `${daysLabel} of trial to go. Upgrade to continue using Switchboard without limits.`,
        actionLabel: "Upgrade",
        onAction: () => void handleUpgradeAction(upgradeDefaultPlanId),
      };
    }
    return {
      tone: pricingStatus.optimizationAllowed
        ? ("warning" as const)
        : ("expired" as const),
      message: `Trial expired. In the free plan you can only use Switchboard for ${weeklyLimitPercentLabel} of your weekly Claude Code / Codex limits. Upgrade to use Switchboard without limits.`,
      actionLabel: "Upgrade",
      onAction: () => void handleUpgradeAction(upgradeDefaultPlanId),
    };
  })();
  const pricingAuthCard = (
    <section className="pricing-auth-card pricing-auth-card--standalone">
      <div className="pricing-auth-card__header">
        <div>
          <h2>{upgradeAuthMessage}.</h2>
        </div>
      </div>
      {!authCodeRequestedFor ? (
        <>
          <div className="pricing-auth-card__grid pricing-auth-card__grid--single">
            <label className="pricing-auth-field">
              <span>Email</span>
              <div className="pricing-auth-field__input">
                <EnvelopeSimple size={16} weight="bold" />
                <input
                  onChange={(event) => {
                    setAuthEmail(event.target.value);
                    setAuthFlowError(null);
                  }}
                  placeholder="you@example.com"
                  type="email"
                  value={authEmail}
                />
              </div>
            </label>
          </div>
          <div className="pricing-auth-card__actions">
            <button
              className="primary-button"
              disabled={!authEmailValid || authRequestBusy}
              onClick={() => void handleRequestAuthCode()}
              type="button"
            >
              {authRequestBusy ? "Sending..." : "Sign in"}
            </button>
          </div>
          <p className="pricing-auth-card__legal">
            By signing in, you agree to the AI Switchboard Terms of Use
            shown at launch.
          </p>
        </>
      ) : (
        <>
          <div className="pricing-auth-card__code-step">
            <p className="pricing-auth-card__step-copy">
              Enter the authentication code we sent to{" "}
              <strong>{authCodeRequestedFor}</strong>.
            </p>
            <button
              className="link-button pricing-auth-card__change-email"
              onClick={resetUpgradeAuthStep}
              type="button"
            >
              Use a different email
            </button>
          </div>
          <div className="pricing-auth-card__grid pricing-auth-card__grid--single">
            <label className="pricing-auth-field">
              <span>Authentication code</span>
              <div className="pricing-auth-field__input">
                <Key size={16} weight="bold" />
                <input
                  onChange={(event) => {
                    setAuthCode(event.target.value);
                    setAuthFlowError(null);
                  }}
                  placeholder={`Enter the code sent to ${authCodeRequestedFor}`}
                  type="text"
                  value={authCode}
                />
              </div>
            </label>
          </div>
          <div className="pricing-auth-card__actions">
            <button
              className="primary-button"
              disabled={!authCode.trim() || authVerifyBusy}
              onClick={() => void handleVerifyAuthCode()}
              type="button"
            >
              {authVerifyBusy ? "Verifying..." : "Verify and continue"}
            </button>
            <p className="pricing-auth-card__resend">
              Didn't receive a code?{" "}
              <button
                className="link-button"
                disabled={authRequestBusy}
                onClick={() => void handleRequestAuthCode()}
                type="button"
              >
                {authRequestBusy ? "Sending..." : "Resend code"}
              </button>
            </p>
          </div>
        </>
      )}
      {authFlowError ? (
        <p className="install-progress__error">{authFlowError}</p>
      ) : null}
      {authFlowSuccess ? (
        <p className="upgrade-plan-card__contact-status upgrade-plan-card__contact-status--success">
          {authFlowSuccess}
        </p>
      ) : null}
      {pricingError ? (
        <p className="install-progress__error">{pricingError}</p>
      ) : null}
    </section>
  );

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
        />

        <UsageSavingsView
          hidden={activeView !== "usage"}
          chartMode={chartMode}
          setChartMode={setChartMode}
          setShowSavingsInfo={setShowSavingsInfo}
          savingsDashboard={savingsDashboard}
          dashboard={dashboard}
          savingsCalculatorRepoEstimate={savingsCalculatorRepoEstimate}
          runtimeStatus={runtimeStatus}
          activityFeed={activityFeed}
          savingsAttributionEvents={savingsAttributionEvents}
          cavemanSavingsEstimate={cavemanSavingsEstimate}
          ponytailSavingsEstimate={ponytailSavingsEstimate}
          markitdownSavingsEstimate={markitdownSavingsEstimate}
          savingsCalculatorScope={savingsCalculatorScope}
          setSavingsCalculatorScope={setSavingsCalculatorScope}
          historyLoadTimedOut={historyLoadTimedOut}
          chartResetSignal={chartResetSignal}
        />

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

        <div className="tray-content" hidden={activeView !== "optimization"}>
          <article className="soft-card optimize-card">
            <header className="optimize-card__head">
              <div className="optimize-card__title-row">
                <span className="optimize-card__title-icon" aria-hidden="true">
                  <Brain weight="duotone" />
                </span>
                <h1>Project learnings</h1>
              </div>
              <p className="optimize-card__blurb">{learnBlurb}</p>
            </header>
            <div className="optimize-card__body">
              <div className="optimize-learn-setup" role="note">
                <strong>Where this lives</strong>
                <span>
                  Enable Claude Code or Codex in Addons, then return here and
                  run a visible scan button for the project or session history.
                </span>
              </div>
              {!headroomLearnSupported ? (
                <div className="optimize-minimal">
                  <p className="optimize-minimal__meta">
                    {headroomLearnDisabledReason}
                  </p>
                  <p className="optimize-minimal__meta">
                    Linux preview currently supports the core Headroom proxy,
                    Claude Code routing, and RTK activity tracking.
                  </p>
                </div>
              ) : !claudeLearnEnabled && !codexLearnEnabled ? (
                <div className="optimize-empty-action">
                  <p className="loading-copy">
                    No learning source is enabled yet. Turn on the Claude Code
                    or Codex connector in Addons, then the scan controls appear
                    here.
                  </p>
                  <button
                    type="button"
                    className="secondary-button secondary-button--small"
                    onClick={() => setActiveView("addons")}
                  >
                    Open Addons
                  </button>
                </div>
              ) : (
                <div className="optimize-minimal">
                  {claudeLearnEnabled &&
                  claudeProjectsBusy &&
                  claudeProjects.length === 0 ? (
                    <p className="loading-copy">Loading projects…</p>
                  ) : claudeLearnEnabled && claudeProjects.length === 0 ? (
                    <p className="loading-copy">
                      No Claude Code projects found in{" "}
                      <code>~/.claude/projects</code>.
                    </p>
                  ) : claudeLearnEnabled ? (
                    <>
                      {!headroomLearnPrereq.claudeCliAvailable ? (
                        <div className="install-prompt" role="status">
                          <header className="install-prompt__head">
                            <span
                              className="install-prompt__icon"
                              aria-hidden="true"
                            >
                              <Terminal weight="duotone" />
                            </span>
                            <div className="install-prompt__head-text">
                              <h2 className="install-prompt__title">
                                Install the Claude Code CLI
                              </h2>
                              <p className="install-prompt__body">
                                Headroom Learn uses the <code>claude</code> CLI
                                to analyze your sessions.
                              </p>
                            </div>
                          </header>
                          <div className="install-prompt__cmd">
                            <code className="install-prompt__cmd-text">
                              {CLAUDE_CODE_INSTALL_CURL_CMD}
                            </code>
                            <button
                              className="install-prompt__cmd-copy"
                              type="button"
                              onClick={() =>
                                void copyLearnInstallCommand(
                                  CLAUDE_CODE_INSTALL_CURL_CMD,
                                )
                              }
                            >
                              Copy
                            </button>
                          </div>
                          <div className="install-prompt__foot">
                            <button
                              className="install-prompt__link"
                              type="button"
                              onClick={() => void openLearnInstallDocsLink()}
                            >
                              Open install docs
                            </button>
                            <span
                              className="install-prompt__foot-sep"
                              aria-hidden="true"
                            >
                              ·
                            </span>
                            <button
                              className="install-prompt__link install-prompt__link--recheck"
                              type="button"
                              onClick={() =>
                                void refreshHeadroomLearnPrereq(true)
                              }
                            >
                              <ArrowClockwise
                                weight="bold"
                                size={12}
                                aria-hidden="true"
                              />
                              Re-check
                            </button>
                            {learnInstallCopyNotice ? (
                              <span className="install-prompt__notice">
                                {learnInstallCopyNotice}
                              </span>
                            ) : null}
                          </div>
                        </div>
                      ) : null}
                      <div className="optimize-projects">
                        {visibleClaudeProjects.map((project) => {
                          const isRunning =
                            headroomLearnStatus.running &&
                            headroomLearnStatus.projectPath ===
                              project.projectPath;
                          const isLatestLearnProject =
                            headroomLearnStatus.projectPath ===
                            project.projectPath;
                          const disableLearn =
                            !headroomLearnPrereq.claudeCliAvailable ||
                            headroomLearnBusy ||
                            claudeProjectsBusy ||
                            (headroomLearnStatus.running && !isRunning);
                          const learnMeta = formatLearnStatus(project);
                          const projectResultTone =
                            headroomLearnStatus.success === true
                              ? "success"
                              : headroomLearnStatus.success === false ||
                                  headroomLearnStatus.error
                                ? "failure"
                                : "idle";
                          const projectResultLabel =
                            headroomLearnStatus.success === true
                              ? "Run succeeded"
                              : headroomLearnStatus.success === false ||
                                  headroomLearnStatus.error
                                ? "Last run failed"
                                : "No completed run yet";
                          const showInlineResult =
                            isLatestLearnProject &&
                            !headroomLearnStatus.running &&
                            (headroomLearnStatus.success !== null ||
                              Boolean(headroomLearnStatus.error) ||
                              headroomLearnStatus.outputTail.length > 0);
                          return (
                            <div
                              className={`optimize-project-row${isRunning || showInlineResult ? " optimize-project-row--active" : ""}`}
                              key={project.id}
                            >
                              <div className="optimize-project-row__main">
                                <span className="optimize-project-row__name">
                                  <strong>{project.displayName}</strong>
                                  <small>
                                    <span
                                      className="optimize-project-row__training"
                                      aria-live="polite"
                                    >
                                      {isRunning
                                        ? `Scanning sessions${
                                            typeof headroomLearnStatus.elapsedSeconds ===
                                            "number"
                                              ? ` · ${headroomLearnStatus.elapsedSeconds}s`
                                              : ""
                                          }`
                                        : learnMeta}
                                    </span>
                                    <OptimizePanel
                                      projectPath={project.projectPath}
                                      refreshSignal={
                                        isLatestLearnProject &&
                                        !headroomLearnStatus.running
                                          ? Date.parse(
                                              headroomLearnStatus.finishedAt ??
                                                "",
                                            ) || 0
                                          : 0
                                      }
                                      preloadedApplied={
                                        optimizeAppliedByProject
                                          ? (optimizeAppliedByProject[
                                              project.projectPath
                                            ] ?? {
                                              claudeMd: [],
                                              memoryMd: [],
                                            })
                                          : undefined
                                      }
                                      onAppliedMutated={() =>
                                        setOptimizeAppliedRefreshTick(
                                          (tick) => tick + 1,
                                        )
                                      }
                                    />
                                  </small>
                                </span>
                                <div className="optimize-project-row__actions">
                                  <button
                                    type="button"
                                    className={`secondary-button secondary-button--small optimize-project-row__scan${isRunning ? " is-spinning" : ""}`}
                                    onClick={() =>
                                      void handleRunHeadroomLearn(
                                        "claude",
                                        project.projectPath,
                                      )
                                    }
                                    disabled={disableLearn}
                                  >
                                    <ArrowClockwise
                                      weight="bold"
                                      size={12}
                                      aria-hidden="true"
                                    />
                                    {isRunning
                                      ? "Scanning"
                                      : "Scan Claude project"}
                                  </button>
                                  {showInlineResult ? (
                                    <span
                                      className={`optimize-project-row__status optimize-minimal__result--${projectResultTone}`}
                                    >
                                      {projectResultLabel}
                                    </span>
                                  ) : null}
                                </div>
                              </div>
                              {showInlineResult && headroomLearnStatus.error ? (
                                <div className="optimize-project-row__result">
                                  <p className="install-progress__error">
                                    {headroomLearnStatus.error}
                                  </p>
                                </div>
                              ) : null}
                            </div>
                          );
                        })}
                      </div>
                      {sortedClaudeProjects.length > 3 ? (
                        <button
                          className="optimize-minimal__inline-action optimize-projects__toggle"
                          onClick={() =>
                            setShowAllClaudeProjects((current) => !current)
                          }
                          type="button"
                        >
                          {showAllClaudeProjects
                            ? "fewer projects"
                            : "more projects"}
                        </button>
                      ) : null}
                    </>
                  ) : null}
                  {codexLearnEnabled
                    ? (() => {
                        const codexReady =
                          headroomLearnPrereq.codexCliAvailable &&
                          headroomLearnPrereq.codexLoggedIn;
                        const codexRunning =
                          headroomLearnStatus.running &&
                          headroomLearnStatus.projectPath === "codex";
                        const codexIsLatest =
                          headroomLearnStatus.projectPath === "codex";
                        const codexDisable =
                          !codexReady ||
                          headroomLearnBusy ||
                          (headroomLearnStatus.running && !codexRunning);
                        const codexShowResult =
                          codexIsLatest &&
                          !headroomLearnStatus.running &&
                          (headroomLearnStatus.success !== null ||
                            Boolean(headroomLearnStatus.error) ||
                            headroomLearnStatus.outputTail.length > 0);
                        const codexResultTone =
                          headroomLearnStatus.success === true
                            ? "success"
                            : headroomLearnStatus.success === false ||
                                headroomLearnStatus.error
                              ? "failure"
                              : "idle";
                        const codexResultLabel =
                          headroomLearnStatus.success === true
                            ? "Run succeeded"
                            : headroomLearnStatus.success === false ||
                                headroomLearnStatus.error
                              ? "Last run failed"
                              : "No completed run yet";
                        if (!codexReady) {
                          const codexCmd = headroomLearnPrereq.codexCliAvailable
                            ? CODEX_CLI_LOGIN_CMD
                            : CODEX_CLI_INSTALL_CMD;
                          return (
                            <div className="install-prompt" role="status">
                              <header className="install-prompt__head">
                                <span
                                  className="install-prompt__icon"
                                  aria-hidden="true"
                                >
                                  <Terminal weight="duotone" />
                                </span>
                                <div className="install-prompt__head-text">
                                  <h2 className="install-prompt__title">
                                    {headroomLearnPrereq.codexCliAvailable
                                      ? "Sign in to the Codex CLI"
                                      : "Install the Codex CLI"}
                                  </h2>
                                  <p className="install-prompt__body">
                                    Headroom Learn analyzes your Codex sessions
                                    with the <code>codex</code> CLI on your
                                    ChatGPT subscription.
                                    {headroomLearnPrereq.codexCliAvailable
                                      ? " Sign in to continue."
                                      : ""}
                                  </p>
                                </div>
                              </header>
                              <div className="install-prompt__cmd">
                                <code className="install-prompt__cmd-text">
                                  {codexCmd}
                                </code>
                                <button
                                  className="install-prompt__cmd-copy"
                                  type="button"
                                  onClick={() =>
                                    void copyLearnInstallCommand(codexCmd)
                                  }
                                >
                                  Copy
                                </button>
                              </div>
                              <div className="install-prompt__foot">
                                <button
                                  className="install-prompt__link"
                                  type="button"
                                  onClick={() =>
                                    void openExternalLink(
                                      CODEX_INSTALL_DOCS_URL,
                                    )
                                  }
                                >
                                  Open install docs
                                </button>
                                <span
                                  className="install-prompt__foot-sep"
                                  aria-hidden="true"
                                >
                                  ·
                                </span>
                                <button
                                  className="install-prompt__link install-prompt__link--recheck"
                                  type="button"
                                  onClick={() =>
                                    void refreshHeadroomLearnPrereq(true)
                                  }
                                >
                                  <ArrowClockwise
                                    weight="bold"
                                    size={12}
                                    aria-hidden="true"
                                  />
                                  Re-check
                                </button>
                                {learnInstallCopyNotice ? (
                                  <span className="install-prompt__notice">
                                    {learnInstallCopyNotice}
                                  </span>
                                ) : null}
                              </div>
                            </div>
                          );
                        }
                        return (
                          <div className="optimize-projects">
                            <div
                              className={`optimize-project-row${codexRunning || codexShowResult ? " optimize-project-row--active" : ""}`}
                            >
                              <div className="optimize-project-row__main">
                                <span className="optimize-project-row__name">
                                  <strong>Codex sessions</strong>
                                  <small>
                                    <span
                                      className="optimize-project-row__training"
                                      aria-live="polite"
                                    >
                                      {codexRunning
                                        ? `Scanning sessions${
                                            typeof headroomLearnStatus.elapsedSeconds ===
                                            "number"
                                              ? ` · ${headroomLearnStatus.elapsedSeconds}s`
                                              : ""
                                          }`
                                        : "Scans ~/.codex/sessions into AGENTS.md"}
                                    </span>
                                  </small>
                                </span>
                                <div className="optimize-project-row__actions">
                                  <button
                                    type="button"
                                    className={`secondary-button secondary-button--small optimize-project-row__scan${codexRunning ? " is-spinning" : ""}`}
                                    onClick={() =>
                                      void handleRunHeadroomLearn("codex")
                                    }
                                    disabled={codexDisable}
                                  >
                                    <ArrowClockwise
                                      weight="bold"
                                      size={12}
                                      aria-hidden="true"
                                    />
                                    {codexRunning
                                      ? "Scanning"
                                      : "Scan Codex sessions"}
                                  </button>
                                  {codexShowResult ? (
                                    <span
                                      className={`optimize-project-row__status optimize-minimal__result--${codexResultTone}`}
                                    >
                                      {codexResultLabel}
                                    </span>
                                  ) : null}
                                </div>
                              </div>
                              {codexShowResult && headroomLearnStatus.error ? (
                                <div className="optimize-project-row__result">
                                  <p className="install-progress__error">
                                    {headroomLearnStatus.error}
                                  </p>
                                </div>
                              ) : null}
                            </div>
                          </div>
                        );
                      })()
                    : null}
                </div>
              )}
              {claudeProjectsError ? (
                <p className="install-progress__error">{claudeProjectsError}</p>
              ) : null}
              {headroomLearnStatus.error &&
              headroomLearnStatus.projectPath !== "codex" &&
              !claudeProjects.some(
                (project) =>
                  project.projectPath === headroomLearnStatus.projectPath,
              ) ? (
                <p className="install-progress__error">
                  {headroomLearnStatus.error}
                </p>
              ) : null}
            </div>
          </article>
        </div>

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

        <AddonsView
          activeView={activeView}
          setActiveView={setActiveView}
          addonError={addonError}
          runtimeStatus={runtimeStatus}
          dashboard={dashboard}
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
          setCavemanLevel={setCavemanLevel}
          copyPlannedConnectorCommand={copyPlannedConnectorCommand}
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

        <div
          className="tray-content"
          data-readiness-signals={localFirstReadinessSourceSignals.join(" | ")}
          hidden={activeView !== "settings"}
        >
          <section className="panel-stack">
            <article className="soft-card panel-card settings-account-card">
              <div className="settings-account-row">
                <p className="settings-account-copy">
                  Account and paid APIs: <em>not included</em>
                </p>
                <span className="settings-account-badge">Local-free</span>
              </div>
              <p className="settings-account-notice">
                AI Switchboard does not include remote account, billing,
                checkout, or paid pricing APIs. Provider model calls still use
                the accounts you configure in Claude, Codex, or other tools.
              </p>
            </article>

            <SettingsLegalPanel
              requiredTermsVersion={dashboard.requiredTermsVersion}
            />

            <SettingsTransferCard
              switchboardMode={switchboardMode}
              savingsMode={savingsMode}
              connectorCount={connectors.length}
              addonCount={dashboard.tools.filter((tool) => !tool.required).length}
              importText={settingsImportText}
              importPreview={settingsImportPreview}
              importBusy={settingsImportBusy}
              notice={settingsTransferNotice}
              onCopyExport={() => void copySettingsExport()}
              onImportTextChange={(value) => {
                setSettingsImportText(value);
                setSettingsImportPreview(null);
                setSettingsTransferNotice(null);
              }}
              onPreviewImport={previewSettingsImport}
              onApplyImport={() => void applySettingsImport()}
            />

            <article className="soft-card panel-card">
              <div className="panel-card__header">
                <div />
              </div>
              </article>

              <SettingsConnectorPanel
                connectors={connectors}
                connectorsBusy={connectorsBusy}
                connectorsError={connectorsError}
                copyPlannedConnectorCommand={copyPlannedConnectorCommand}
                openConnectorHelpId={openConnectorHelpId}
                plannedConnectorCopyNotice={plannedConnectorCopyNotice}
                plannedConnectorReadiness={plannedConnectorReadiness}
                setOpenConnectorHelpId={setOpenConnectorHelpId}
                toggleConnector={toggleConnector}
              />

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
                      <span className="runtime-status__channel-pill">
                        beta channel
                      </span>
                    ) : null}
                  </span>
                </div>
                <div className="runtime-status__section-action-row">
                  <button
                    className="secondary-button secondary-button--small"
                    disabled={appUpdateBusy || appUpdateInstallBusy}
                    onClick={() => void checkForAppUpdate()}
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
                    Headroom CLI ({headroomVersion})
                    {headroomLifetimeSavingsPct !== null ? (
                      <span className="runtime-status__section-context">
                        {" "}
                        ({percent1(headroomLifetimeSavingsPct)}% all-time
                        savings)
                      </span>
                    ) : null}
                  </span>
                </div>
                <div className="runtime-status__grid runtime-status__grid--4">
                  {(
                    [
                      {
                        name: "Runtime",
                        ok: runtimeStatus?.running === true,
                      },
                      {
                        name: "Proxy",
                        ok: runtimeStatus?.proxyReachable === true,
                        suffix: "6767",
                        onClick: () => void invoke("open_headroom_dashboard"),
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
                    }[]
                  ).map((s) => {
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
                        title={
                          s.ok === null ? `${s.name} status unknown` : undefined
                        }
                      >
                        <span className="runtime-status__label">{s.name}:</span>
                        <span
                          className={`runtime-status__indicator ${indicatorClass}`}
                        >
                          {indicatorSymbol}
                        </span>
                        {s.suffix && (
                          <span className="runtime-status__suffix">
                            ({s.suffix})
                          </span>
                        )}
                      </span>
                    );
                  })}
                </div>
                <button
                  className="link-button runtime-status__section-action"
                  onClick={async () => {
                    const next = !showHeadroomDetails;
                    setShowHeadroomDetails(next);
                    if (next) {
                      try {
                        const lines = await invoke<string[]>(
                          "get_headroom_logs",
                          { maxLines: 80 },
                        );
                        setHeadroomLogLines(lines);
                      } catch {
                        setHeadroomLogLines(["Failed to load headroom logs."]);
                      }
                    }
                  }}
                  type="button"
                >
                  {showHeadroomDetails
                    ? "Hide headroom logs"
                    : "Show headroom logs"}
                </button>
                {showHeadroomDetails ? (
                  <pre className="runtime-log" ref={headroomLogRef}>
                    {headroomLogLines.length > 0
                      ? headroomLogLines.join("\n")
                      : "No log output yet."}
                  </pre>
                ) : null}
              </div>
            </article>
            <article
              className="soft-card panel-card release-readiness-card"
              id="release-readiness"
            >
              <div className="panel-card__header">
                <div>
                  <h3>Release readiness</h3>
                  <p>
                    {releaseReadinessItemCount()} checks before a signed DMG can
                    be handed to testers.
                  </p>
                </div>
                <div className="release-readiness-card__actions">
                  <button
                    className="secondary-button secondary-button--small"
                    disabled={releaseReadinessRefreshing}
                    onClick={() => void refreshReleaseReadinessReport()}
                    type="button"
                  >
                    <ArrowClockwise size={14} weight="bold" />
                    {releaseReadinessRefreshing
                      ? "Refreshing"
                      : "Refresh report"}
                  </button>
                  <button
                    className="secondary-button secondary-button--small"
                    disabled={releaseEvidenceBusyId !== null}
                    onClick={() => void runLocalReleaseEvidenceSequence()}
                    title={formatLocalReleaseEvidenceSequenceCopy()}
                    type="button"
                  >
                    <ArrowClockwise size={14} weight="bold" />
                    {releaseEvidenceBusyId === "local-evidence"
                      ? "Running local evidence"
                      : "Run local evidence"}
                  </button>
                  <button
                    className="secondary-button secondary-button--small"
                    onClick={() => void copyReleaseReadinessReport()}
                    type="button"
                  >
                    <Copy size={14} weight="bold" />
                    {releaseReadinessReport?.report
                      ? "Copy report snapshot"
                      : "Copy report command"}
                  </button>
                </div>
              </div>
              <div className="release-readiness-card__command">
                <Terminal size={15} weight="duotone" />
                <code>{releaseReadinessCommand}</code>
              </div>
              <p className="release-readiness-card__source">
                {formatReleaseReadinessSourceLabel(
                  releaseReadinessReport?.report
                    ? releaseReadinessReport.reportPath
                    : null,
                )}
              </p>
              <p className="release-readiness-card__source">
                {releaseReadinessEvidence.copy}
              </p>
              <p className="release-readiness-card__source">
                {formatReleaseReadinessNextAction(releaseReadinessAction)}
              </p>
              {releaseReadinessError ? (
                <p className="release-readiness-card__error">
                  {releaseReadinessError}
                </p>
              ) : null}
              <div
                className="release-readiness-card__summary"
                aria-label="Release readiness status summary"
              >
                <span>
                  <strong>{releaseReadinessCounts.ready}</strong> scripted
                </span>
                <span>
                  <strong>{releaseReadinessCounts.blocked}</strong> blocked
                </span>
                <span>
                  <strong>{releaseReadinessCounts["local-only"]}</strong> local-only
                </span>
              </div>
              <div
                className="release-readiness-card__status-grid"
                aria-label="Release readiness source status"
              >
                {releaseReadinessRows.map((row) => (
                  <div
                    className="release-readiness-card__status-row"
                    key={row.id}
                  >
                    <div>
                      <strong>{row.label}</strong>
                      <span>{row.detail}</span>
                    </div>
                    <span
                      className={`release-readiness-card__status-badge release-readiness-card__status-badge--${row.tone}`}
                    >
                      {row.statusLabel}
                    </span>
                    <code>{row.source}</code>
                  </div>
                ))}
              </div>
              {releaseLocalEvidenceRows.length > 0 ? (
                <div
                  className="release-readiness-card__local-evidence"
                  aria-label="Local validation evidence"
                >
                  <h4>Local evidence</h4>
                  <div className="release-readiness-card__status-grid">
                    {releaseLocalEvidenceRows.map((row) => (
                      <div
                        className="release-readiness-card__status-row"
                        key={row.id}
                      >
                        <div>
                          <strong>{row.label}</strong>
                          <span>{row.detail}</span>
                        </div>
                        <span
                          className={`release-readiness-card__status-badge release-readiness-card__status-badge--${
                            row.passed ? "ready" : "blocked"
                          }`}
                        >
                          {row.statusLabel}
                        </span>
                        <code>{row.command}</code>
                        <code>{row.summaryPath}</code>
                      </div>
                    ))}
                  </div>
                </div>
              ) : null}
              <div
                className="release-readiness-card__gates"
                aria-label="Shareable DMG gates"
              >
                {releaseShareableGates.map((gate) => (
                  <div className="release-readiness-card__gate" key={gate.id}>
                    <strong>{gate.label}</strong>
                    <span>{gate.detail}</span>
                  </div>
                ))}
              </div>
              <div className="release-readiness-card__grid">
                {releaseReadinessGroups.map((group) => (
                  <section
                    className="release-readiness-card__group"
                    key={group.id}
                  >
                    <h4>{group.title}</h4>
                    <ul>
                      {group.items.map((item) => (
                        <li key={item.id}>
                          <strong>{item.label}</strong>
                          <span>{item.detail}</span>
                          {item.command ? <code>{item.command}</code> : null}
                          {item.executable ? (
                            <button
                              className="secondary-button secondary-button--small"
                              disabled={releaseEvidenceBusyId !== null}
                              onClick={() =>
                                void runReleaseEvidenceCommand(item.id)
                              }
                              type="button"
                            >
                              <ArrowClockwise size={14} weight="bold" />
                              {releaseEvidenceBusyId === item.id
                                ? "Running"
                                : "Run evidence"}
                            </button>
                          ) : null}
                          {releaseEvidenceResult?.commandId === item.id ? (
                            <span className="release-readiness-card__evidence-result">
                              Generated{" "}
                              {releaseEvidenceResult.summaryPath ??
                                releaseEvidenceResult.command}
                            </span>
                          ) : null}
                        </li>
                      ))}
                    </ul>
                  </section>
                ))}
              </div>
              {releaseReadinessCopyNotice ? (
                <p className="connector-copy-notice">
                  {releaseReadinessCopyNotice}
                </p>
              ) : null}
            </article>
            <SettingsOpenLoginCard
              autostartBusy={autostartBusy}
              autostartEnabled={autostartEnabled}
              onToggle={handleAutostartToggle}
            />

            <RollbackCenter />

            <SettingsUninstallCard
              onOpenUninstallDialog={() => {
                setUninstallError(null);
                setShowUninstallDialog(true);
              }}
            />

            <button
              className="contact-link"
              onClick={() =>
                void invoke("open_external_link", {
                  url: SUPPORT_ISSUES_URL,
                })
              }
              type="button"
            >
              Contact us
            </button>
            <button
              className="quit-button"
              onClick={() => void invoke("quit_headroom")}
              type="button"
            >
              Quit AI Switchboard for Mac
            </button>
          </section>
        </div>

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
