import {
  useEffect,
  useRef,
  useState,
  type ElementType,
  type FormEvent,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent,
  type ReactElement,
  type ReactNode,
} from "react";
import {
  ArrowClockwise,
  Bell,
  Brain,
  Calculator,
  CaretLeft,
  Copy,
  Cpu,
  CurrencyCircleDollar,
  CurrencyDollar,
  Info,
  EnvelopeSimple,
  FirstAidKit,
  GearSix,
  House,
  Key,
  PuzzlePiece,
  SignOut,
  Sliders,
  Sparkle,
  Terminal,
} from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import macAiSwitchboardLogo from "./assets/mac-ai-switchboard-logo.png";
import packageJson from "../package.json";
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
import { plannedAddons, type PlannedAddon } from "./lib/plannedAddons";
import {
  buildAgentSessionPreparation,
  buildRepoAgentHandoffPayload,
  buildRepoAgentManifest,
  buildRepoIntelligenceSummary,
  estimateRepoIntelligenceSavings,
  formatRepoAgentHandoffMarkdown,
  formatAgentSessionPreparationJson,
  formatAgentSessionSelectedPackMarkdown,
  formatRepoAgentManifestJson,
  formatRepoContextPackMarkdown,
  formatSingleRepoContextPackMarkdown,
  getRepoIndexFreshness,
  repoAgentHandoffProfiles,
  type AgentSessionTaskType,
  type RepoContextPack,
  type RepoAgentHandoffTarget,
  type RepoIntelligenceSummary,
  type RepoSavingsEstimate,
} from "./lib/repoIntelligence";
import {
  formatPlannedConnectorConfigCreationPlansMarkdown,
  getPlannedConnector,
  getPlannedConnectorConfigCreationPlan,
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorReadinessContract,
  getPlannedConnectorSetupChecklistScript,
  getPlannedConnectorSetupGuide,
  plannedConnectors,
  type PlannedConnector,
} from "./lib/plannedConnectors";
import {
  releaseReadinessCommand,
  formatReleaseReadinessReportSnapshot,
  releaseReadinessGroups,
  releaseReadinessItemCount,
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
  getFounderStepPricing,
  isTierDowngrade,
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
  aggregateClientConnectors,
  addDays,
  addMonths,
  buildHourlySavingsChartData,
  buildHourlySavingsWindow,
  buildMonthlySavingsChartData,
  buildMonthlySavingsWindow,
  compactNumber,
  connectorControlState,
  connectorCompatibilityReport,
  connectorDashboardStatus,
  currency,
  currencyExact,
  dayOfMonthTickFormatter,
  earliestHourlyDay,
  earliestSavingsMonth,
  formatDateTime,
  formatDayKey,
  formatLearnStatus,
  formatMonthLabel,
  formatSelectedDayLabel,
  getEnabledSupportedConnectors,
  hasEnabledConnector,
  hourOfDayTickFormatter,
  mergeProviderSavingsForDisplay,
  percent1,
  sortClientConnectors,
  startOfDay,
  startOfMonth,
  summarizePlannedConnectorReadiness,
  type SavingsChartDatum,
} from "./lib/dashboardHelpers";
import {
  buildInitialProxyVerificationRows,
  getClaudeConnector,
  getContactRequestValidationError,
  getInitialLauncherStage,
  getLauncherAutoConfigureDecision,
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
  safeNotificationActionView,
  safeTrayViewForMode,
  serializeState,
  shouldShowCodexNudge,
  type TrayView,
} from "./lib/trayHelpers";
import {
  trackAnalyticsEvent,
  trackInstallMilestoneOnce,
} from "./lib/analytics";
import { localOnlyModeEnabled } from "./lib/localMode";
import {
  buildManagedConfigDiffPreview,
  formatManagedConfigDiffPreview,
  managedChangeRecords,
  type ManagedChangeRecord,
} from "./lib/managedChanges";
import {
  doctorTimelineKindLabel,
  buildManagedChangeTimelineEvents,
  formatDoctorTimelineShareText,
  sortDoctorTimelineEvents,
  type DoctorTimelineEvent,
} from "./lib/doctorRepairCopy";
import {
  formatUninstallDryRunReport,
  uninstallDisclosureFooter,
  uninstallDisclosureItems,
  uninstallDisclosureTitle,
} from "./lib/uninstallDisclosure";
import {
  deriveSwitchboardMode,
  switchboardModeLabel,
  switchboardModeSummary,
} from "./lib/switchboardDisplay";
import {
  buildAddonSavingsEstimate,
  buildFilteredSavingsLedger,
  buildSavingsCalculatorBreakdown,
  buildSavingsLedgerRows,
  buildSavingsCalculatorSummary,
  formatSavingsLedgerShareText,
  CAVEMAN_TEMPLATE_BASELINE_TOKENS,
  CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS,
  PONYTAIL_TEMPLATE_BASELINE_TOKENS,
  PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS,
  MARKITDOWN_TEMPLATE_BASELINE_TOKENS,
  MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS,
  type AddonSavingsEstimate,
  type SavingsLedgerConfidenceFilter,
  type SavingsCalculatorScope,
} from "./lib/savingsCalculator";
import { ActivityFeed } from "./components/ActivityFeed";
import { LauncherShell } from "./components/LauncherShell";
import { OptimizePanel } from "./components/OptimizePanel";
import { TermsGate } from "./components/TermsGate";
import { SwitchboardPanel } from "./components/SwitchboardPanel";
import { SwitchboardDoctorPanel } from "./components/SwitchboardDoctorPanel";
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
  ActivityFeedResponse,
  AppliedPatterns,
  HourlySavingsPoint,
  OutputReduction,
  RuntimeStatus,
  RuntimeUpgradeFailure,
  RuntimeUpgradeProgress,
  SavingsMode,
  SwitchboardMode,
  SwitchboardState,
} from "./lib/types";

interface NavItem {
  id: TrayView;
  label: string;
  icon: ElementType;
}

const navItems: NavItem[] = [
  { id: "home", label: "Home", icon: House },
  { id: "usage", label: "Usage", icon: Calculator },
  { id: "doctor", label: "Doctor", icon: FirstAidKit },
  { id: "optimization", label: "Optimize", icon: Sliders },
  { id: "notifications", label: "Activity", icon: Bell },
  { id: "repoIntelligence", label: "Repo Intelligence", icon: Brain },
  { id: "addons", label: "Addons", icon: PuzzlePiece },
];

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
      "Caveman installed. Pick scoped or aggressive terse mode any time.",
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

const connectorSetupDetails: Record<string, string> = {
  claude_code:
    "Headroom injects ANTHROPIC_BASE_URL into shell profiles and ~/.claude/settings.json so Claude Code connects through Headroom. Token-saving add-ons like RTK are optional.",
  codex:
    "Headroom writes a managed provider block to ~/.codex/config.toml and exports OPENAI_BASE_URL in shell profiles so Codex connects through Headroom.",
  gemini_cli:
    "Gemini CLI is tracked as a planned adapter. Until its reversible config path is implemented, use RTK-only mode for command output savings.",
  opencode:
    "OpenCode is tracked as a planned adapter. Until its reversible config path is implemented, use RTK-only mode for command output savings.",
  cursor:
    "Cursor is tracked as a planned editor connector. Guided setup is shown first because Cursor settings and account behavior can vary by release channel.",
  grok_cli:
    "Grok / xAI CLI is tracked as a planned provider connector. Mac AI Switchboard will keep model and account compatibility visible before routing it.",
  aider:
    "Aider is tracked as a planned agent connector. RTK-only mode can already reduce noisy shell output while provider wrapping is built.",
  continue:
    "Continue is tracked as a planned editor connector. Guided setup stays manual until provider config backup and restore coverage is ready.",
  goose:
    "Goose is tracked as a planned agent connector. Local provider and MCP handoff support will be added after reversible setup coverage.",
  qwen_code:
    "Qwen Code is tracked as a planned CLI connector. Use Repo Intelligence packs today while provider routing waits for model and account guardrails.",
  amazon_q:
    "Amazon Q Developer CLI is tracked as a planned CLI connector. Verification packs are safe today; AWS credential and profile state stay outside managed setup.",
  windsurf:
    "Windsurf is tracked as a planned editor connector. Paste Repo Intelligence handoffs manually until settings backup and restore support lands.",
  zed_ai:
    "Zed AI is tracked as a planned editor connector. Keep provider settings manual while Switchboard adds lossless settings detection and restore.",
};

const connectorSupportWarnings: Record<string, string> = {};

const connectorUnavailableReasons: Record<string, string> = {
  claude_code:
    "Claude Code was not detected. Install Claude Code and restart Headroom.",
  codex: "Codex was not detected. Install the Codex CLI and restart Headroom.",
  gemini_cli: "Gemini CLI adapter is planned but not configurable yet.",
  opencode: "OpenCode adapter is planned but not configurable yet.",
  cursor: "Cursor adapter is planned but not configurable yet.",
  grok_cli: "Grok / xAI CLI adapter is planned but not configurable yet.",
  aider: "Aider adapter is planned but not configurable yet.",
  continue: "Continue adapter is planned but not configurable yet.",
  goose: "Goose adapter is planned but not configurable yet.",
  qwen_code: "Qwen Code adapter is planned but not configurable yet.",
  amazon_q:
    "Amazon Q Developer CLI adapter is planned but not configurable yet.",
  windsurf: "Windsurf adapter is planned but not configurable yet.",
  zed_ai: "Zed AI adapter is planned but not configurable yet.",
};

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

const CLAUDE_CODE_INSTALL_DOCS_URL =
  "https://docs.claude.com/en/docs/claude-code/setup";
const CLAUDE_CODE_INSTALL_CURL_CMD =
  "curl -fsSL https://claude.ai/install.sh | bash";
const CODEX_CLI_INSTALL_CMD = "npm install -g @openai/codex";
const CODEX_CLI_LOGIN_CMD = "codex login";
const CODEX_INSTALL_DOCS_URL = "https://developers.openai.com/codex/cli";
const CODEX_INSTALL_NPM_CMD = "npm i -g @openai/codex";

const SALES_CONTACT_URL =
  (import.meta.env.VITE_HEADROOM_SALES_CONTACT_URL ?? "").trim() ||
  "mailto:hello@example.com";
const CONTACT_FORM_URL = (
  import.meta.env.VITE_HEADROOM_CONTACT_FORM_URL ?? ""
).trim();

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

function SavingsChartTooltip({
  active,
  payload,
  chartMode,
}: {
  active?: boolean;
  payload?: ReadonlyArray<{ payload?: SavingsChartDatum }>;
  chartMode: SavingsChartMode;
}) {
  const point = payload?.[0]?.payload;
  if (!active || !point) {
    return null;
  }

  const providerSavings = mergeProviderSavingsForDisplay(
    point.byProvider ?? [],
  );

  return (
    <div className="savings-chart__tooltip">
      <strong>{point.bucketLabel}</strong>
      {providerSavings.length > 0 ? (
        // Hourly buckets carry per-provider attribution: show Saved/Spent per
        // connector instead of the bucket total (which would be redundant).
        providerSavings.map((provider) => (
          <div className="savings-chart__tooltip-group" key={provider.label}>
            <span className="savings-chart__tooltip-label">
              {provider.label}
            </span>
            <span className="savings-chart__tooltip-item">
              <i
                aria-hidden="true"
                className={`savings-chart__tooltip-dot savings-chart__tooltip-dot--${
                  chartMode === "usd" ? "saved-usd" : "saved-tokens"
                }`}
              />
              {chartMode === "usd"
                ? `Saved ${currencyExact(provider.estimatedSavingsUsd)}`
                : `Saved ${compactNumber(provider.estimatedTokensSaved)} tokens`}
            </span>
            <span className="savings-chart__tooltip-item">
              <i
                aria-hidden="true"
                className={`savings-chart__tooltip-dot savings-chart__tooltip-dot--${
                  chartMode === "usd" ? "actual-usd" : "actual-tokens"
                }`}
              />
              {chartMode === "usd"
                ? `Spent ${currencyExact(provider.actualCostUsd)}`
                : `Spent ${compactNumber(provider.totalTokensSent)} tokens`}
            </span>
          </div>
        ))
      ) : // Monthly buckets (and pre-attribution hourly buckets) have no provider
      // dimension: fall back to the aggregate bucket total.
      chartMode === "usd" ? (
        <div className="savings-chart__tooltip-group">
          <span className="savings-chart__tooltip-label">Dollars</span>
          <span className="savings-chart__tooltip-item">
            <i
              aria-hidden="true"
              className="savings-chart__tooltip-dot savings-chart__tooltip-dot--saved-usd"
            />
            Saved {currencyExact(point.estimatedSavingsUsd)}
          </span>
          <span className="savings-chart__tooltip-item">
            <i
              aria-hidden="true"
              className="savings-chart__tooltip-dot savings-chart__tooltip-dot--actual-usd"
            />
            Spent {currencyExact(point.actualCostUsd)}
          </span>
        </div>
      ) : (
        <div className="savings-chart__tooltip-group">
          <span className="savings-chart__tooltip-label">Tokens</span>
          <span className="savings-chart__tooltip-item">
            <i
              aria-hidden="true"
              className="savings-chart__tooltip-dot savings-chart__tooltip-dot--saved-tokens"
            />
            Saved {compactNumber(point.estimatedTokensSaved)} tokens
          </span>
          <span className="savings-chart__tooltip-item">
            <i
              aria-hidden="true"
              className="savings-chart__tooltip-dot savings-chart__tooltip-dot--actual-tokens"
            />
            Spent {compactNumber(point.totalTokensSent)} tokens
          </span>
        </div>
      )}
    </div>
  );
}

function delay(ms: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

type SavingsChartView = "month" | "day";
type SavingsChartMode = "usd" | "tokens";
const savingsLedgerConfidenceFilters: SavingsLedgerConfidenceFilter[] = [
  "all",
  "measured",
  "estimated",
  "inferred",
];

// Output-token reduction from the proxy's output shaper, shown as a secondary
// line inside the "Total input tokens saved" card so the two numbers (input
// tokens saved vs. output tokens not emitted) read as distinct. The line shows
// just the headline percent; clicking it opens a popover with the method
// ("estimated"/"measured"), confidence band, request count, and a note that
// output savings are counterfactual. Caller renders this only when `reduction`
// is present (the backend returns null until a verbosity baseline is seeded).
// The parent card is itself clickable, so the trigger stops event propagation.
function OutputReductionChip({ reduction }: { reduction: OutputReduction }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const isMeasured = reduction.method === "measured";

  useEffect(() => {
    if (!open) return;
    const onDown = (e: Event) => {
      if (ref.current && !ref.current.contains(e.target as Node))
        setOpen(false);
    };
    const onKey = (e: Event) => {
      if ((e as KeyboardEvent).key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="output-chip" ref={ref}>
      <button
        type="button"
        className={`output-chip__button${open ? " is-open" : ""}`}
        aria-expanded={open}
        aria-label="Output token reduction details"
        onClick={(e) => {
          e.stopPropagation();
          setOpen((v) => !v);
        }}
        onKeyDown={(e) => e.stopPropagation()}
      >
        <span className="output-chip__dot" aria-hidden="true" />
        Output −{percent1(reduction.reductionPercent)}%
      </button>
      {open ? (
        <div
          className="output-chip__popover"
          role="dialog"
          aria-label="Output reduction details"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="output-chip__pop-head">
            <span className="output-chip__pop-title">
              Output token reduction
            </span>
            <span className="output-chip__pop-badge">
              {isMeasured ? "measured" : "estimated"}
            </span>
          </div>
          <div className="output-chip__pop-value">
            {percent1(reduction.reductionPercent)}%
          </div>
          <dl className="output-chip__pop-stats">
            <div>
              <dt>95% CI</dt>
              <dd>
                {percent1(reduction.ciLowPercent)}–
                {percent1(reduction.ciHighPercent)}%
              </dd>
            </div>
            <div>
              <dt>Requests</dt>
              <dd>{compactNumber(reduction.requests)}</dd>
            </div>
          </dl>
          <p className="output-chip__pop-note">
            {isMeasured
              ? "Output tokens the model didn't emit because the shaper steered verbosity / routed effort down — measured against an unshaped A/B holdout."
              : "Output tokens the model didn't emit because the shaper steered verbosity / routed effort down. Output savings are counterfactual, so this is an estimate vs a learned baseline."}
          </p>
        </div>
      ) : null}
    </div>
  );
}

function SavingsCalculatorCard({
  dashboard,
  repoSavings,
  runtimeStatus,
  cavemanSavings,
  ponytailSavings,
  markitdownSavings,
  scope,
  onScopeChange,
}: {
  dashboard: DashboardState;
  repoSavings?: RepoSavingsEstimate | null;
  runtimeStatus?: RuntimeStatus | null;
  cavemanSavings?: AddonSavingsEstimate | null;
  ponytailSavings?: AddonSavingsEstimate | null;
  markitdownSavings?: AddonSavingsEstimate | null;
  scope: SavingsCalculatorScope;
  onScopeChange: (scope: SavingsCalculatorScope) => void;
}) {
  const summary = buildSavingsCalculatorSummary(dashboard, scope);
  const breakdownRows = buildSavingsCalculatorBreakdown(dashboard, scope, {
    repoSavings,
    runtimeStatus,
    cavemanSavings,
    ponytailSavings,
    markitdownSavings,
  });
  const ledgerRows = buildSavingsLedgerRows(
    dashboard,
    scope,
    new Date().toISOString(),
    {
      repoSavings,
      runtimeStatus,
      cavemanSavings,
      ponytailSavings,
      markitdownSavings,
    },
  );
  const savedLabel = compactNumber(summary.savedTokens);
  const sentLabel = compactNumber(summary.sentTokens);
  const beforeLabel = compactNumber(summary.beforeTokens);
  const conservativeUsdLabel = currencyExact(summary.conservativeSavedUsd);
  const hasUsage = summary.requests > 0 || summary.savedTokens > 0;
  const percentLabel =
    summary.savingsPct === null
      ? "Waiting for usage"
      : `${percent1(summary.savingsPct)}%`;

  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const [ledgerFilter, setLedgerFilter] =
    useState<SavingsLedgerConfidenceFilter>("all");
  const ledgerRecordedAt = ledgerRows[0]?.recordedAt ?? new Date().toISOString();
  const filteredLedger = buildFilteredSavingsLedger(
    ledgerRows,
    scope,
    ledgerRecordedAt,
    ledgerFilter,
  );

  async function copySavingsSummary() {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }

    await navigator.clipboard.writeText(
      formatSavingsLedgerShareText(
        filteredLedger.rows,
        scope,
        ledgerRecordedAt,
      ),
    );
    setCopyNotice(
      ledgerFilter === "all" ? "Copied ledger." : `Copied ${ledgerFilter}.`,
    );
  }

  return (
    <article className="soft-card savings-calculator">
      <header className="savings-calculator__header">
        <div className="savings-calculator__title">
          <span className="savings-calculator__icon" aria-hidden="true">
            <Calculator weight="duotone" />
          </span>
          <div>
            <h2>Savings calculator</h2>
            <p>{summary.dataLabel}</p>
          </div>
        </div>
        <div
          className="savings-calculator__scope"
          role="group"
          aria-label="Savings scope"
        >
          {(["session", "overall"] as const).map((item) => (
            <button
              key={item}
              type="button"
              className={`savings-calculator__scope-button${
                scope === item ? " is-active" : ""
              }`}
              onClick={() => onScopeChange(item)}
            >
              {item === "session" ? "Session" : "Overall"}
            </button>
          ))}
        </div>
        <button
          type="button"
          className="savings-calculator__copy"
          onClick={copySavingsSummary}
          disabled={!hasUsage}
          title="Copy savings summary"
        >
          <Copy aria-hidden="true" weight="bold" />
          <span>{copyNotice ?? "Copy"}</span>
        </button>
      </header>
      <div className="savings-calculator__body">
        <div className="savings-calculator__hero">
          <span>
            {scope === "session" ? "Saved this session" : "Saved overall"}
          </span>
          <strong>{currencyExact(summary.savedUsd)}</strong>
        </div>
        <dl className="savings-calculator__metrics">
          <div>
            <dt>Tokens saved</dt>
            <dd>{savedLabel}</dd>
          </div>
          <div>
            <dt>Requests</dt>
            <dd>{compactNumber(summary.requests)}</dd>
          </div>
          <div>
            <dt>Reduction</dt>
            <dd>{percentLabel}</dd>
          </div>
          <div>
            <dt>Likely at least</dt>
            <dd>{conservativeUsdLabel}</dd>
          </div>
        </dl>
      </div>
      <div
        className="savings-calculator__equation"
        aria-label="Savings equation"
      >
        <span>
          Before <strong>{beforeLabel}</strong>
        </span>
        <span aria-hidden="true">-</span>
        <span>
          Sent <strong>{sentLabel}</strong>
        </span>
        <span aria-hidden="true">=</span>
        <span>
          Saved <strong>{savedLabel}</strong>
        </span>
      </div>
      {!hasUsage ? (
        <p className="savings-calculator__empty">
          Run a connected coding agent through Mac AI Switchboard and this card
          will update automatically.
        </p>
      ) : null}
      <div
        className="savings-calculator__breakdown"
        aria-label="Savings source breakdown"
      >
        {breakdownRows.map((row) => (
          <div className="savings-calculator__breakdown-row" key={row.id}>
            <div>
              <strong>{row.label}</strong>
              <span>
                {row.detail} Source: {row.confidence}.
              </span>
            </div>
            <div className="savings-calculator__breakdown-value">
              <strong>{compactNumber(row.savedTokens)}</strong>
              <span>
                {row.savedUsd === null
                  ? "tokens"
                  : `${currencyExact(row.savedUsd)} estimate`}
              </span>
            </div>
          </div>
        ))}
      </div>
      <div className="savings-calculator__ledger" aria-label="Savings ledger">
        <div className="savings-calculator__ledger-head">
          <div>
            <span>Ledger</span>
            <strong>{scope === "session" ? "Session" : "Overall"}</strong>
          </div>
          <div
            className="savings-calculator__ledger-filters"
            role="group"
            aria-label="Ledger confidence filter"
          >
            {savingsLedgerConfidenceFilters.map((filter) => (
              <button
                key={filter}
                type="button"
                className={`savings-calculator__ledger-filter${
                  ledgerFilter === filter ? " is-active" : ""
                }`}
                onClick={() => setLedgerFilter(filter)}
              >
                {filter === "all" ? "All" : filter}
              </button>
            ))}
          </div>
        </div>
        <div className="savings-calculator__ledger-summary">
          <span>
            Sources <strong>{filteredLedger.groups.length}</strong>
          </span>
          <span>
            Rows <strong>{compactNumber(filteredLedger.summary.rowCount)}</strong>
          </span>
          <span>
            Tokens{" "}
            <strong>{compactNumber(filteredLedger.summary.totalTokens)}</strong>
          </span>
        </div>
        <div className="savings-calculator__ledger-list">
          {filteredLedger.groups.length > 0 ? (
            filteredLedger.groups.map((group) => (
              <div
                className="savings-calculator__ledger-row"
                key={group.source}
              >
                <div>
                  <strong>{group.label}</strong>
                  <span>
                    {group.confidence} · {group.rowCount} row
                    {group.rowCount === 1 ? "" : "s"}
                  </span>
                </div>
                <div>
                  <strong>{compactNumber(group.totalTokens)}</strong>
                  <span>
                    {group.measuredTokens > 0
                      ? `${compactNumber(group.measuredTokens)} measured`
                      : group.estimatedTokens > 0
                        ? `${compactNumber(group.estimatedTokens)} estimated`
                        : `${compactNumber(group.inferredTokens)} inferred`}
                  </span>
                </div>
              </div>
            ))
          ) : (
            <div className="savings-calculator__ledger-row">
              <div>
                <strong>No matching ledger rows</strong>
                <span>Change the confidence filter to see other sources.</span>
              </div>
              <div>
                <strong>0</strong>
                <span>{formatDateTime(ledgerRecordedAt)}</span>
              </div>
            </div>
          )}
        </div>
      </div>
    </article>
  );
}

function DailySavingsChart({
  data,
  hourlyData,
  resetSignal,
  chartMode,
  setChartMode,
}: {
  data: DailySavingsPoint[];
  hourlyData: HourlySavingsPoint[];
  resetSignal: number;
  chartMode: SavingsChartMode;
  setChartMode: (mode: SavingsChartMode) => void;
}) {
  const currentMonth = startOfMonth(new Date());
  const today = startOfDay(new Date());
  const [visibleMonth, setVisibleMonth] = useState(() => currentMonth);
  const [visibleDay, setVisibleDay] = useState(() => today);
  const [view, setView] = useState<SavingsChartView>("day");
  const [savingsTodayUsd, setSavingsTodayUsd] = useState<number | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<number>("savings-today-updated", (event) => {
      setSavingsTodayUsd(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);
  const firstSavingsMonth = earliestSavingsMonth(data);
  const firstHourlyDay = earliestHourlyDay(hourlyData);
  const monthlyData = buildMonthlySavingsChartData(
    buildMonthlySavingsWindow(data, visibleMonth),
  );
  const hourlyChartData = buildHourlySavingsChartData(
    buildHourlySavingsWindow(hourlyData, visibleDay),
  );
  const chartData = view === "month" ? monthlyData : hourlyChartData;
  const canViewPreviousMonth = firstSavingsMonth
    ? visibleMonth > firstSavingsMonth
    : false;
  const canViewNextMonth = visibleMonth < currentMonth;
  const canViewPreviousDay = firstHourlyDay
    ? visibleDay > firstHourlyDay
    : false;
  const canViewNextDay = visibleDay < today;
  const label =
    view === "month"
      ? formatMonthLabel(visibleMonth)
      : formatSelectedDayLabel(visibleDay);

  useEffect(() => {
    const now = new Date();
    setVisibleMonth(startOfMonth(now));
    setVisibleDay(startOfDay(now));
  }, [resetSignal]);

  return (
    <div className="savings-chart">
      <section
        aria-label={
          view === "month"
            ? `Monthly history for ${label}`
            : `Hourly history for ${label}`
        }
        className="savings-chart__panel"
      >
        <div className="savings-chart__panel-header">
          <div className="savings-chart__title-row">
            <strong>History</strong>
            <div className="savings-chart__toggle" aria-label="Metric">
              <button
                className={`savings-chart__toggle-button${chartMode === "usd" ? " is-active" : ""}`}
                onClick={() => setChartMode("usd")}
                type="button"
              >
                $
              </button>
              <button
                className={`savings-chart__toggle-button${chartMode === "tokens" ? " is-active" : ""}`}
                onClick={() => setChartMode("tokens")}
                type="button"
              >
                tokens
              </button>
            </div>
          </div>
          <div className="savings-chart__nav">
            <div className="savings-chart__toggle" aria-label="History view">
              <button
                className={`savings-chart__toggle-button${view === "month" ? " is-active" : ""}`}
                onClick={() => setView("month")}
                type="button"
              >
                month
              </button>
              <button
                className={`savings-chart__toggle-button${view === "day" ? " is-active" : ""}`}
                onClick={() => setView("day")}
                type="button"
              >
                day
              </button>
            </div>
            <button
              className="savings-chart__nav-button"
              disabled={
                view === "month" ? !canViewPreviousMonth : !canViewPreviousDay
              }
              onClick={() =>
                view === "month"
                  ? setVisibleMonth((current) => addMonths(current, -1))
                  : setVisibleDay((current) => addDays(current, -1))
              }
              type="button"
            >
              Prev
            </button>
            <span className="savings-chart__range-label">{label}</span>
            <button
              className="savings-chart__nav-button"
              disabled={view === "month" ? !canViewNextMonth : !canViewNextDay}
              onClick={() =>
                view === "month"
                  ? setVisibleMonth((current) => addMonths(current, 1))
                  : setVisibleDay((current) => addDays(current, 1))
              }
              type="button"
            >
              Next
            </button>
          </div>
        </div>
        <div className="savings-chart__canvas savings-chart__canvas--combined">
          <div className="savings-chart__overlay" aria-hidden="true">
            <span className="savings-chart__overlay-total">
              {chartMode === "usd"
                ? currency(
                    Math.max(
                      0,
                      view === "day" &&
                        visibleDay >= today &&
                        savingsTodayUsd !== null
                        ? savingsTodayUsd
                        : chartData.reduce(
                            (s, d) => s + d.estimatedSavingsUsd,
                            0,
                          ),
                    ),
                  )
                : compactNumber(
                    Math.max(
                      0,
                      chartData.reduce((s, d) => s + d.estimatedTokensSaved, 0),
                    ),
                  )}
            </span>
            <span className="savings-chart__overlay-label">
              {view === "day" ? "saved today" : "saved this month"}
            </span>
          </div>
          <ResponsiveContainer height="100%" width="100%">
            <BarChart
              barCategoryGap="5%"
              barGap={1}
              data={chartData}
              margin={{ top: 64, right: 2, left: 2, bottom: 0 }}
            >
              <defs>
                <linearGradient
                  id="actualUsdGradient"
                  x1="0"
                  x2="0"
                  y1="0"
                  y2="1"
                >
                  <stop offset="0%" stopColor="#c96a30" />
                  <stop offset="100%" stopColor="#ED834E" />
                </linearGradient>
                <linearGradient
                  id="savingsUsdGradient"
                  x1="0"
                  x2="0"
                  y1="0"
                  y2="1"
                >
                  <stop offset="0%" stopColor="#3a7f74" />
                  <stop offset="100%" stopColor="#4F9E91" />
                </linearGradient>
                <linearGradient
                  id="actualTokensGradient"
                  x1="0"
                  x2="0"
                  y1="0"
                  y2="1"
                >
                  <stop offset="0%" stopColor="#c96a30" />
                  <stop offset="100%" stopColor="#ED834E" />
                </linearGradient>
                <linearGradient
                  id="savingsTokensGradient"
                  x1="0"
                  x2="0"
                  y1="0"
                  y2="1"
                >
                  <stop offset="0%" stopColor="#d4b832" stopOpacity="0.35" />
                  <stop offset="100%" stopColor="#EBCC6E" stopOpacity="0.25" />
                </linearGradient>
              </defs>
              <CartesianGrid
                stroke="rgba(36, 31, 29, 0.06)"
                strokeDasharray="2 8"
                vertical={false}
              />
              <XAxis
                axisLine={false}
                dataKey="bucketKey"
                interval={0}
                minTickGap={view === "month" ? 8 : 8}
                tickFormatter={
                  view === "month"
                    ? dayOfMonthTickFormatter
                    : hourOfDayTickFormatter
                }
                tick={{ fill: "#7a7169", fontSize: 10 }}
                tickLine={false}
              />
              <YAxis hide yAxisId="usd" />
              <YAxis hide yAxisId="tokens" />
              <Tooltip
                content={(props) => (
                  <SavingsChartTooltip {...props} chartMode={chartMode} />
                )}
                cursor={{ fill: "rgba(36, 31, 29, 0.05)" }}
              />
              {chartMode === "usd" && (
                <>
                  <Bar
                    dataKey="actualCostUsd"
                    fill="url(#actualUsdGradient)"
                    maxBarSize={16}
                    stackId="usd"
                    yAxisId="usd"
                  />
                  <Bar
                    dataKey="estimatedSavingsUsd"
                    fill="url(#savingsUsdGradient)"
                    maxBarSize={16}
                    radius={[1, 1, 0, 0]}
                    stackId="usd"
                    yAxisId="usd"
                  />
                </>
              )}
              {chartMode === "tokens" && (
                <>
                  <Bar
                    dataKey="totalTokensSent"
                    fill="url(#actualTokensGradient)"
                    maxBarSize={16}
                    stackId="tokens"
                    yAxisId="tokens"
                  />
                  <Bar
                    dataKey="estimatedTokensSaved"
                    fill="url(#savingsTokensGradient)"
                    maxBarSize={16}
                    stackId="tokens"
                    yAxisId="tokens"
                    shape={(props: any) => {
                      const { x, y, width, height, fill } = props;
                      if (!width || !height) return <g />;
                      const sw = 1.5;
                      return (
                        <rect
                          x={x + sw / 2}
                          y={y + sw / 2}
                          width={Math.max(0, width - sw)}
                          height={Math.max(0, height - sw)}
                          fill={fill}
                          stroke="#EBCC6E"
                          strokeWidth={sw}
                          rx={1}
                        />
                      );
                    }}
                  />
                </>
              )}
            </BarChart>
          </ResponsiveContainer>
        </div>
      </section>
    </div>
  );
}

function renderConnectorLogo(clientId: string) {
  return <Sparkle className="client-logo__glyph" size={20} weight="duotone" />;
}

function AddonClientChips({
  connectors,
}: {
  connectors: ClientConnectorStatus[];
}) {
  const clients = sortClientConnectors(aggregateClientConnectors(connectors));
  if (clients.length === 0) {
    return null;
  }
  return (
    <div className="addon-card__clients">
      {clients.map((connector) => {
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
            <span className="callout-banner__chip-name">{connector.name}</span>
            <span className="visually-hidden">{status.label}</span>
          </span>
        );
      })}
    </div>
  );
}

function formatAddonVersion(version: string): string {
  return /^\d/.test(version) ? `v${version}` : version;
}

function AddonCard({
  name,
  version,
  installed,
  enabled,
  description,
  copy,
  infoOpen,
  onToggleInfo,
  busy,
  busyLabel,
  resultMessage,
  onDismissResult,
  sourceUrl,
  onOpenSource,
  connectors,
  showClients,
  actionsDisabled,
  onInstall,
  onToggleEnabled,
  onUninstall,
  children,
}: {
  name: string;
  version?: string | null;
  installed: boolean;
  enabled: boolean;
  description: ReactNode;
  copy?: AddonCopy;
  infoOpen: boolean;
  onToggleInfo: () => void;
  busy: boolean;
  busyLabel: string | null;
  resultMessage: string | null;
  onDismissResult: () => void;
  sourceUrl: string;
  onOpenSource: () => void;
  connectors: ClientConnectorStatus[];
  showClients: boolean;
  actionsDisabled: boolean;
  onInstall: () => void;
  onToggleEnabled: () => void;
  onUninstall: () => void;
  children?: ReactNode;
}) {
  return (
    <li className="addon-card">
      <div className="addon-card__body">
        <div className="addon-card__heading">
          <span className="addon-card__name">{name}</span>
          {installed && version ? (
            <span className="addon-card__version">
              {formatAddonVersion(version)}
            </span>
          ) : null}
          {copy ? (
            <button
              type="button"
              className="addon-card__info"
              aria-label={`What ${name} does`}
              aria-expanded={infoOpen}
              onClick={onToggleInfo}
            >
              i
            </button>
          ) : null}
          {installed ? (
            <span
              className={`addon-card__badge addon-card__badge--${enabled ? "on" : "off"}`}
            >
              {enabled ? "Enabled" : "Disabled"}
            </span>
          ) : null}
        </div>
        {infoOpen && copy ? (
          <p className="addon-card__info-text">{copy.whatItDoes}</p>
        ) : null}
        <p className="addon-card__description">{description}</p>
        {showClients ? <AddonClientChips connectors={connectors} /> : null}
        <button
          type="button"
          className="addon-card__link"
          onClick={onOpenSource}
        >
          {sourceUrl}
        </button>
        {busy && busyLabel ? (
          <p className="addon-card__progress">{busyLabel}</p>
        ) : resultMessage ? (
          <p className="addon-card__result">
            {resultMessage}
            <button
              type="button"
              className="addon-card__result-dismiss"
              aria-label="Dismiss"
              onClick={onDismissResult}
            >
              ×
            </button>
          </p>
        ) : null}
        {children}
      </div>
      <div className="addon-card__actions">
        {!installed ? (
          <button
            type="button"
            className="addon-card__action addon-card__action--primary"
            disabled={actionsDisabled}
            onClick={onInstall}
          >
            Install
          </button>
        ) : (
          <>
            <button
              type="button"
              className="addon-card__action"
              disabled={actionsDisabled}
              onClick={onToggleEnabled}
            >
              {enabled ? "Disable" : "Enable"}
            </button>
            <button
              type="button"
              className="addon-card__action addon-card__action--danger"
              disabled={actionsDisabled}
              onClick={onUninstall}
            >
              Uninstall
            </button>
          </>
        )}
      </div>
    </li>
  );
}

function PlannedAddonCard({
  addon,
  onOpenRepoIntelligence,
}: {
  addon: PlannedAddon;
  onOpenRepoIntelligence?: () => void;
}) {
  const showConnectorRoadmap = addon.id === "agent_connectors";
  const showRepoIntelligencePreview = addon.id === "repo_intelligence";
  const isRepoIntelligence = addon.id === "repo_intelligence";
  const cardClassName = isRepoIntelligence
    ? "addon-card addon-card--active"
    : "addon-card addon-card--planned";
  const badgeClassName = isRepoIntelligence
    ? "addon-card__badge addon-card__badge--ready"
    : "addon-card__badge addon-card__badge--planned";

  return (
    <li className={cardClassName}>
      <div className="addon-card__body">
        <div className="addon-card__heading">
          <span className="addon-card__name">{addon.name}</span>
          <span className={badgeClassName}>{addon.statusLabel}</span>
        </div>
        <p className="addon-card__description">{addon.description}</p>
        <ul className="addon-card__bullets">
          {addon.bullets.map((bullet) => (
            <li key={bullet}>{bullet}</li>
          ))}
        </ul>
        <div className="addon-card__evidence-grid">
          <section>
            <h4>Health checks</h4>
            <ul>
              {addon.healthChecks.map((check) => (
                <li key={check}>{check}</li>
              ))}
            </ul>
          </section>
          <section>
            <h4>Savings sources</h4>
            <ul>
              {addon.savingsSources.map((source) => (
                <li key={source}>{source}</li>
              ))}
            </ul>
          </section>
        </div>
        {addon.verificationCommand ? (
          <p className="addon-card__verification">
            <Terminal size={13} weight="duotone" />
            <code>{addon.verificationCommand}</code>
          </p>
        ) : null}
        {showConnectorRoadmap ? (
          <PlannedConnectorRoadmap connectors={plannedConnectors} />
        ) : null}
        {showRepoIntelligencePreview ? (
          <div className="repo-intelligence-addon-cta">
            <strong>Dedicated workspace is available.</strong>
            <span>
              Open Repo Intelligence to index a repository and copy real local
              packs.
            </span>
            <button
              className="addon-card__action addon-card__action--primary"
              onClick={onOpenRepoIntelligence}
              type="button"
            >
              Open Repo Intelligence
            </button>
          </div>
        ) : null}
      </div>
      <div className="addon-card__actions">
        <button type="button" className="addon-card__action" disabled>
          {isRepoIntelligence ? "Open from sidebar" : "Coming soon"}
        </button>
      </div>
    </li>
  );
}

const repoIntelligencePreview = buildRepoIntelligenceSummary([
  { path: "src/App.tsx", bytes: 184_000 },
  { path: "src/lib/dashboardHelpers.ts", bytes: 28_000 },
  { path: "src/lib/repoIntelligence.ts", bytes: 7_400 },
  { path: "src-tauri/src/client_adapters.rs", bytes: 190_000 },
  { path: "src-tauri/src/lib.rs", bytes: 310_000 },
  { path: "src/lib/repoIntelligence.test.ts", bytes: 2_300 },
  { path: "src/lib/dashboardHelpers.test.ts", bytes: 18_000 },
  { path: "docs/repo-intelligence-plan.md", bytes: 4_800 },
  { path: "docs/beta-smoke-test.md", bytes: 9_200 },
  { path: "package.json", bytes: 1_900 },
  { path: "dist/assets/index.js", bytes: 767_000 },
]);

const primaryRepoAgentIds = new Set<RepoAgentHandoffTarget>([
  "claude",
  "codex",
]);

function repoAgentGroupLabel(
  profile: (typeof repoAgentHandoffProfiles)[number],
) {
  if (primaryRepoAgentIds.has(profile.id)) {
    return "Primary agents";
  }
  switch (profile.toolKind) {
    case "cli":
      return "CLI agents";
    case "editor":
      return "Editor agents";
    case "chat":
      return "Chat agents";
  }
}

function repoAgentPackLabel(packId: string) {
  switch (packId) {
    case "implementation":
      return "Implementation pack";
    case "verification":
      return "Verification pack";
    case "handoff":
      return "Handoff pack";
    default:
      return `${packId} pack`;
  }
}

const repoAgentHandoffGroups = repoAgentHandoffProfiles.reduce<
  Array<{
    label: string;
    profiles: typeof repoAgentHandoffProfiles;
  }>
>((groups, profile) => {
  const label = repoAgentGroupLabel(profile);
  const group = groups.find((candidate) => candidate.label === label);
  if (group) {
    group.profiles.push(profile);
  } else {
    groups.push({ label, profiles: [profile] });
  }
  return groups;
}, []);

function sampleManagedBlock(record: ManagedChangeRecord) {
  return [
    `# >>> ${record.markerId} >>>`,
    `# Managed by Mac AI Switchboard for ${record.owner}.`,
    "# Actual write paths fill this block from the connector adapter dry-run.",
    `# <<< ${record.markerId} <<<`,
  ].join("\n");
}

function firstManagedConfigTarget(record: ManagedChangeRecord) {
  return record.paths[0] ?? "~/.config/mac-ai-switchboard-managed";
}

function buildDoctorTimelinePreview(
  report: DoctorReport | null,
  successMessage: string | null,
): DoctorTimelineEvent[] {
  const now = new Date().toISOString();
  const events: DoctorTimelineEvent[] = [
    {
      id: "latest-report",
      kind: "repair",
      title: report ? `Doctor status: ${report.status}` : "Doctor report pending",
      body: report?.summary ?? "Run Doctor to capture local setup evidence.",
      occurredAt: now,
      status: report?.status ?? "warning",
      actor: "doctor",
      target: "switchboard setup",
    },
  ];

  if (successMessage) {
    events.push({
      id: "latest-repair-success",
      kind: "repair",
      title: "Latest repair completed",
      body: successMessage,
      occurredAt: now,
      status: "ok",
      actor: "doctor",
      target: "automatic repair",
    });
  }

  return sortDoctorTimelineEvents([
    ...events,
    ...buildManagedChangeTimelineEvents(managedChangeRecords, now),
  ]);
}

function DoctorTimelineCard({
  events,
}: {
  events: DoctorTimelineEvent[];
}) {
  const [copyNotice, setCopyNotice] = useState<string | null>(null);

  async function copyTimeline() {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }
    await navigator.clipboard.writeText(formatDoctorTimelineShareText(events));
    setCopyNotice("Copied timeline.");
    window.setTimeout(() => setCopyNotice(null), 2500);
  }

  return (
    <article className="soft-card doctor-timeline">
      <div className="doctor-timeline__head">
        <div>
          <span>Doctor timeline</span>
          <strong>{events.length} event{events.length === 1 ? "" : "s"}</strong>
        </div>
        <button
          className="secondary-button secondary-button--small"
          onClick={() => void copyTimeline()}
          type="button"
        >
          {copyNotice ?? "Copy timeline"}
        </button>
      </div>
      <div className="doctor-timeline__list">
        {events.map((event) => (
          <div className="doctor-timeline__event" key={event.id}>
            <div>
              <strong>{event.title}</strong>
              <span>{event.body}</span>
            </div>
            <div>
              <span>{doctorTimelineKindLabel(event.kind)}</span>
              <span>{event.status}</span>
            </div>
          </div>
        ))}
      </div>
    </article>
  );
}

function RepoIntelligencePreview({
  headroomHealthy = false,
  onSummaryChange,
  rtkHealthy = false,
}: {
  headroomHealthy?: boolean;
  onSummaryChange?: (summary: RepoIntelligenceSummary) => void;
  rtkHealthy?: boolean;
}) {
  const [repoPath, setRepoPath] = useState("");
  const [selectedAgent, setSelectedAgent] =
    useState<RepoAgentHandoffTarget>("codex");
  const [selectedTaskType, setSelectedTaskType] =
    useState<AgentSessionTaskType>("verification");
  const [summary, setSummary] = useState<RepoIntelligenceSummary>(
    repoIntelligencePreview,
  );
  const [indexing, setIndexing] = useState(false);
  const [indexError, setIndexError] = useState<string | null>(null);
  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const isPreview = summary === repoIntelligencePreview;
  const hasRealIndex = !isPreview;
  const indexFreshness = getRepoIndexFreshness(summary);
  const indexStatusLabel = indexFreshness.label;
  const cacheStateLabel = summary.indexMetadata
    ? `${summary.indexMetadata.cacheState} cache · ${summary.indexMetadata.fileFingerprints.length.toLocaleString()} fingerprints · ${(summary.indexMetadata.skippedFiles?.length ?? summary.indexMetadata.skippedFileCount).toLocaleString()} skipped reasons · ${(summary.indexMetadata.graphInputs?.length ?? 0).toLocaleString()} graph inputs · ${summary.indexMetadata.parserVersion}`
    : null;
  const savingsEstimate = estimateRepoIntelligenceSavings(summary);
  const agentManifest = buildRepoAgentManifest(summary);
  const selectedAgentProfile =
    repoAgentHandoffProfiles.find((profile) => profile.id === selectedAgent) ??
    repoAgentHandoffProfiles[0];
  const providerRoutingSafe = primaryRepoAgentIds.has(selectedAgent);
  const sessionPreparation = buildAgentSessionPreparation(summary, {
    target: selectedAgentProfile.id,
    taskType: selectedTaskType,
    modeInputs: {
      headroomHealthy,
      rtkHealthy,
      providerRoutingSafe,
    },
  });

  useEffect(() => {
    let cancelled = false;
    invoke<RepoIntelligenceSummary | null>(
      "get_latest_repo_intelligence_summary",
    )
      .then((latest) => {
        if (!cancelled && latest) {
          setSummary(latest);
          setRepoPath(latest.repoRoot ?? "");
          onSummaryChange?.(latest);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  async function runRepoIndex() {
    const trimmedPath = repoPath.trim();
    if (!trimmedPath) {
      setIndexError("Enter a local repository folder path first.");
      return;
    }
    setIndexing(true);
    setIndexError(null);
    try {
      const next = await invoke<RepoIntelligenceSummary>(
        "build_repo_intelligence_summary",
        {
          repoPath: trimmedPath,
        },
      );
      setSummary(next);
      onSummaryChange?.(next);
    } catch (error) {
      setIndexError(
        error instanceof Error
          ? error.message
          : "Repo Intelligence could not index that folder.",
      );
    } finally {
      setIndexing(false);
    }
  }

  async function clearRepoIndex() {
    setIndexing(true);
    setIndexError(null);
    try {
      await invoke<boolean>("clear_repo_intelligence_summary");
      setSummary(repoIntelligencePreview);
      setRepoPath("");
      onSummaryChange?.(repoIntelligencePreview);
    } catch (error) {
      setIndexError(
        error instanceof Error
          ? error.message
          : "Repo Intelligence could not clear the saved index.",
      );
    } finally {
      setIndexing(false);
    }
  }

  async function copyContextPack() {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying real context.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        formatRepoContextPackMarkdown(summary),
      );
      setCopyNotice("Context pack copied.");
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select pack details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyAgentManifest() {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying a real manifest.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(formatRepoAgentManifestJson(summary));
      setCopyNotice("Agent manifest copied.");
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select manifest manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copySingleContextPack(pack: RepoContextPack) {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying this pack.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        formatSingleRepoContextPackMarkdown(summary, pack),
      );
      setCopyNotice(`${pack.title} copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select pack details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyAgentRecipePack(packId: string, label: string) {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying recipe packs.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    const pack = summary.packs.find((contextPack) => contextPack.id === packId);
    if (!pack) {
      setCopyNotice("Recipe pack unavailable. Re-index this repo.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        formatSingleRepoContextPackMarkdown(summary, pack),
      );
      setCopyNotice(`${label} copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select pack details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyAgentHandoff(
    target: RepoAgentHandoffTarget,
    label: string,
  ) {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying agent handoffs.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        formatRepoAgentHandoffMarkdown(summary, target),
      );
      setCopyNotice(`${label} handoff copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select handoff details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyAgentHandoffJson(
    target: RepoAgentHandoffTarget,
    label: string,
  ) {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying agent handoffs.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        JSON.stringify(buildRepoAgentHandoffPayload(summary, target), null, 2),
      );
      setCopyNotice(`${label} JSON handoff copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select JSON handoff manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyPreparedAgentSession() {
    if (!hasRealIndex || !sessionPreparation.handoffMarkdown) {
      setCopyNotice(sessionPreparation.copyDetail);
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(sessionPreparation.handoffMarkdown);
      setCopyNotice(`${sessionPreparation.target.label} session copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select session details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyPreparedAgentSessionJson() {
    const json = formatAgentSessionPreparationJson(sessionPreparation);
    if (!hasRealIndex || !json) {
      setCopyNotice(sessionPreparation.copyDetail);
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(json);
      setCopyNotice(`${sessionPreparation.target.label} JSON copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select session JSON manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyPreparedAgentSessionPack() {
    const packMarkdown = formatAgentSessionSelectedPackMarkdown(
      summary,
      sessionPreparation,
    );
    if (!hasRealIndex || !packMarkdown) {
      setCopyNotice(sessionPreparation.copyDetail);
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(packMarkdown);
      setCopyNotice(
        `${repoAgentPackLabel(sessionPreparation.packId)} copied.`,
      );
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select session pack manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  return (
    <div
      className="repo-intelligence-preview"
      aria-label="Repo Intelligence context pack preview"
    >
      <div className="repo-intelligence-preview__topline">
        <span>{indexStatusLabel}</span>
        <strong>
          {summary.indexedFiles} indexed signals
          {summary.skippedFiles ? `, ${summary.skippedFiles} skipped` : ""}
        </strong>
      </div>
      <div className="repo-intelligence-preview__controls">
        <input
          aria-label="Repository folder path"
          className="repo-intelligence-preview__input"
          onChange={(event) => setRepoPath(event.target.value)}
          placeholder="~/Developer/my-repo"
          type="text"
          value={repoPath}
        />
        <button
          className="addon-card__action addon-card__action--primary"
          disabled={indexing}
          onClick={() => void runRepoIndex()}
          type="button"
        >
          {indexing ? "Indexing..." : "Index"}
        </button>
        {!isPreview ? (
          <>
            <button
              className="addon-card__action"
              disabled={indexing}
              onClick={() => void copyContextPack()}
              type="button"
            >
              Copy pack
            </button>
            <button
              className="addon-card__action"
              disabled={indexing}
              onClick={() => void copyAgentManifest()}
              type="button"
            >
              Copy agent manifest
            </button>
          </>
        ) : null}
        {!isPreview ? (
          <button
            className="addon-card__action"
            disabled={indexing}
            onClick={() => void clearRepoIndex()}
            type="button"
          >
            Clear
          </button>
        ) : null}
      </div>
      {summary.repoRoot ? (
        <p className="repo-intelligence-preview__path">{summary.repoRoot}</p>
      ) : null}
      {summary.indexedAt ? (
        <p className="repo-intelligence-preview__path">
          Indexed {new Date(summary.indexedAt).toLocaleString()}
        </p>
      ) : null}
      {hasRealIndex ? (
        <p className="repo-intelligence-preview__path">
          {indexFreshness.detail}
        </p>
      ) : null}
      {cacheStateLabel ? (
        <p className="repo-intelligence-preview__path">
          Index cache: {cacheStateLabel}
        </p>
      ) : null}
      {copyNotice ? (
        <p className="repo-intelligence-preview__path">{copyNotice}</p>
      ) : null}
      {indexError ? (
        <p className="install-progress__error">{indexError}</p>
      ) : null}
      <div
        className="repo-intelligence-session"
        aria-label="Start agent session"
      >
        <div className="repo-intelligence-session__heading">
          <div>
            <span>Start session</span>
            <strong>{sessionPreparation.target.label}</strong>
          </div>
          <span
            className={`repo-intelligence-session__status repo-intelligence-session__status--${sessionPreparation.copyStatus}`}
          >
            {sessionPreparation.copyStatus}
          </span>
        </div>
        <div className="repo-intelligence-session__controls">
          <label>
            <span>Agent</span>
            <select
              value={selectedAgent}
              onChange={(event) =>
                setSelectedAgent(event.target.value as RepoAgentHandoffTarget)
              }
            >
              {repoAgentHandoffProfiles.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Task</span>
            <select
              value={selectedTaskType}
              onChange={(event) =>
                setSelectedTaskType(event.target.value as AgentSessionTaskType)
              }
            >
              <option value="implementation">Implementation</option>
              <option value="verification">Verification</option>
              <option value="handoff">Handoff</option>
            </select>
          </label>
          <button
            className="addon-card__action addon-card__action--primary"
            disabled={!hasRealIndex || !sessionPreparation.handoffMarkdown}
            onClick={() => void copyPreparedAgentSession()}
            type="button"
          >
            Copy full handoff
          </button>
          <button
            className="addon-card__action"
            disabled={!hasRealIndex || !sessionPreparation.handoffPayload}
            onClick={() => void copyPreparedAgentSessionPack()}
            type="button"
          >
            Copy selected pack
          </button>
          <button
            className="addon-card__action"
            disabled={!hasRealIndex || !sessionPreparation.handoffPayload}
            onClick={() => void copyPreparedAgentSessionJson()}
            type="button"
          >
            Copy JSON
          </button>
        </div>
        <div className="repo-intelligence-session__summary">
          <div>
            <span>Pack</span>
            <strong>{repoAgentPackLabel(sessionPreparation.packId)}</strong>
          </div>
          <div>
            <span>Mode</span>
            <strong>
              {switchboardModeLabel(sessionPreparation.recommendedMode)}
            </strong>
          </div>
          <div>
            <span>Freshness</span>
            <strong>{sessionPreparation.freshness.label}</strong>
          </div>
        </div>
        <p className="repo-intelligence-session__detail">
          {sessionPreparation.copyDetail} Doctor still verifies runtime and
          connector health before any managed setup.
        </p>
        <p className="repo-intelligence-session__detail">
          {sessionPreparation.recommendedModeReason}
        </p>
      </div>
      <div
        className="repo-intelligence-savings"
        aria-label="Repo Intelligence savings calculator"
      >
        <div>
          <span>Full scan</span>
          <strong>{savingsEstimate.fullScanTokens.toLocaleString()}</strong>
          <em>tokens estimated</em>
        </div>
        <div>
          <span>Best pack saved</span>
          <strong>
            {savingsEstimate.bestPackTokensAvoided.toLocaleString()}
          </strong>
          <em>
            {savingsEstimate.bestPack?.title ?? "Context pack"} ·{" "}
            {savingsEstimate.bestPackSavingsPct.toFixed(1)}%
          </em>
        </div>
        <div>
          <span>All packs saved</span>
          <strong>
            {savingsEstimate.allPacksTokensAvoided.toLocaleString()}
          </strong>
          <em>{savingsEstimate.allPacksSavingsPct.toFixed(1)}% vs full scan</em>
        </div>
      </div>
      {summary.graph ? (
        <div
          className="repo-intelligence-graph"
          aria-label="Repo Intelligence graph summary"
        >
          <div>
            <span>Top directories</span>
            <strong>
              {summary.graph.topDirectories
                .slice(0, 3)
                .map((node) => `${node.label} (${node.count})`)
                .join(", ") || "None"}
            </strong>
          </div>
          <div>
            <span>Languages</span>
            <strong>
              {summary.graph.topLanguages
                .slice(0, 3)
                .map((node) => node.label)
                .join(", ") || "Unknown"}
            </strong>
          </div>
          <div>
            <span>Entrypoints</span>
            <strong>{summary.graph.entrypoints.length}</strong>
          </div>
          <div>
            <span>Likely tests</span>
            <strong>{summary.graph.likelyTests.length}</strong>
          </div>
          <div>
            <span>Dependency hubs</span>
            <strong>{summary.graph.dependencyHubs?.length ?? 0}</strong>
            <em>
              {summary.graph.dependencyHubs
                ?.slice(0, 2)
                .map((file) => file.path)
                .join(", ") || "No hub files yet"}
            </em>
          </div>
          <div>
            <span>Import edges</span>
            <strong>{summary.graph.importEdges?.length ?? 0}</strong>
            <em>
              {summary.graph.importEdges
                ?.slice(0, 2)
                .map((edge) => `${edge.from} -> ${edge.to}`)
                .join(", ") || "No path links yet"}
            </em>
          </div>
          <div>
            <span>Reverse hubs</span>
            <strong>{summary.graph.reverseDependencyHubs?.length ?? 0}</strong>
            <em>
              {summary.graph.reverseDependencyHubs
                ?.slice(0, 2)
                .map((node) => `${node.label} (${node.count})`)
                .join(", ") || "No reverse hubs yet"}
            </em>
          </div>
          <div>
            <span>Symbols</span>
            <strong>{summary.graph.symbols?.length ?? 0}</strong>
            <em>
              {summary.graph.symbols
                ?.slice(0, 3)
                .map((symbol) => `${symbol.name} (${symbol.kind})`)
                .join(", ") || "No symbols yet"}
            </em>
          </div>
          <div className="repo-intelligence-graph__wide">
            <span>Agent graph signal</span>
            <strong>
              {`${summary.graph.dependencyHubs?.length ?? 0} hubs · ${
                summary.graph.importEdges?.length ?? 0
              } edges · ${summary.graph.reverseDependencyHubs?.length ?? 0} reverse hubs · ${
                summary.graph.symbols?.length ?? 0
              } symbols`}
            </strong>
            <em>Copied into manifests and handoffs without file contents.</em>
          </div>
        </div>
      ) : null}
      <div className="repo-intelligence-preview__grid">
        {summary.packs.map((pack) => (
          <article className="repo-intelligence-pack" key={pack.id}>
            <div className="repo-intelligence-pack__heading">
              <span>{pack.title}</span>
              <strong>{pack.savingsVsFullScanPct.toFixed(1)}%</strong>
            </div>
            <p>{pack.purpose}</p>
            <span className="repo-intelligence-pack__meta">
              {pack.files.length} files &middot; about{" "}
              {pack.estimatedTokens.toLocaleString()} tokens
            </span>
            {!isPreview ? (
              <button
                className="repo-intelligence-pack__copy"
                onClick={() => void copySingleContextPack(pack)}
                type="button"
              >
                Copy this pack
              </button>
            ) : null}
          </article>
        ))}
      </div>

      <div
        className="repo-intelligence-handoffs"
        aria-label="Agent-specific handoffs"
      >
        <div className="repo-intelligence-recipes__heading">
          <span>Agent handoffs</span>
          <strong>Ready to paste</strong>
        </div>
        <div className="repo-intelligence-handoffs__grid">
          {repoAgentHandoffGroups.map((group) => (
            <section
              className="repo-intelligence-handoff-group"
              key={group.label}
            >
              <div className="repo-intelligence-handoff-group__label">
                <span>{group.label}</span>
              </div>
              <div className="repo-intelligence-handoff-group__buttons">
                {group.profiles.map((profile) => (
                  <div className="repo-intelligence-handoff" key={profile.id}>
                    <div>
                      <strong>{profile.label}</strong>
                      <span>{repoAgentPackLabel(profile.defaultPackId)}</span>
                      <em>{profile.guidance}</em>
                    </div>
                    <div className="repo-intelligence-handoff__actions">
                      <button
                        disabled={isPreview}
                        onClick={() =>
                          void copyAgentHandoff(profile.id, profile.label)
                        }
                        type="button"
                      >
                        Markdown
                      </button>
                      <button
                        disabled={isPreview}
                        onClick={() =>
                          void copyAgentHandoffJson(profile.id, profile.label)
                        }
                        type="button"
                      >
                        JSON
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          ))}
        </div>
      </div>

      <div
        className="repo-intelligence-recipes"
        aria-label="Agent handoff recipes"
      >
        <div className="repo-intelligence-recipes__heading">
          <span>Agent recipes</span>
          <strong>Read-only handoff</strong>
        </div>
        <div className="repo-intelligence-recipes__list">
          {agentManifest.agentRecipes.map((recipe) => (
            <article className="repo-intelligence-recipe" key={recipe.id}>
              <div>
                <strong>{recipe.label}</strong>
                <span>{recipe.tools.join(", ")}</span>
              </div>
              <p>{recipe.instruction}</p>
              {!isPreview ? (
                <button
                  className="repo-intelligence-pack__copy"
                  onClick={() =>
                    void copyAgentRecipePack(recipe.packIds[0], recipe.label)
                  }
                  type="button"
                >
                  Copy recipe pack
                </button>
              ) : null}
            </article>
          ))}
        </div>
      </div>
    </div>
  );
}

function connectorCategoryLabel(category: PlannedConnector["category"]) {
  switch (category) {
    case "cli":
      return "CLI";
    case "editor":
      return "Editor";
    case "agent":
      return "Agent";
  }
}

function PlannedConnectorRoadmap({
  connectors,
}: {
  connectors: PlannedConnector[];
}) {
  return (
    <div className="planned-connectors" aria-label="Planned connector roadmap">
      <div className="planned-connectors__intro">
        <span>Expansion path</span>
        <strong>Detect first, adapt only when reversible.</strong>
      </div>
      <div
        className="planned-connectors__steps"
        aria-label="Connector setup phases"
      >
        <span>Read-only detection</span>
        <span>Guided setup</span>
        <span>Doctor-backed cleanup</span>
      </div>
      <ul className="planned-connectors__list">
        {connectors.map((connector) => {
          const readiness = getPlannedConnectorReadinessContract(connector);
          const readinessBadges = getPlannedConnectorReadinessBadges(connector);
          const configPlan = getPlannedConnectorConfigCreationPlan(connector);
          return (
          <li className="planned-connectors__item" key={connector.id}>
            <div className="planned-connectors__item-head">
              <strong>{connector.name}</strong>
              <span>{connectorCategoryLabel(connector.category)}</span>
            </div>
            <p>{connector.integrationTarget}</p>
            <div className="planned-connectors__capabilities">
              {connector.capabilityBadges.map((badge) => (
                <span key={badge}>{badge}</span>
              ))}
            </div>
            <div
              className="planned-connectors__badges"
              aria-label={`${connector.name} safety badges`}
            >
              {readinessBadges.map((badge) => (
                <span
                  className={`planned-connectors__badge planned-connectors__badge--${badge.kind}`}
                  key={badge.kind}
                  title={badge.detail}
                >
                  {badge.label}
                </span>
              ))}
            </div>
            <div
              className="planned-connectors__modes"
              aria-label={`${connector.name} supported modes`}
            >
              {connector.supportedModes.map((mode) => (
                <span key={mode}>{mode}</span>
              ))}
            </div>
            <div className="planned-connectors__readiness">
              <div>
                <span>Config surface</span>
                <strong>{connector.configSurfaces[0]}</strong>
              </div>
              <div>
                <span>Next gate</span>
                <strong>
                  {readiness.stages.find(
                    (stage) => stage.id === readiness.nextBlockedStage,
                  )?.label ?? "Automation ready"}
                </strong>
              </div>
            </div>
            <div
              className="planned-connectors__config-plan"
              aria-label={`${connector.name} config creation plan`}
              title={configPlan.safetyNote}
            >
              <span>Config creation</span>
              <div>
                {configPlan.steps.map((step) => (
                  <strong key={step.id} title={step.detail}>
                    {step.label}
                  </strong>
                ))}
              </div>
            </div>
            <div
              className="planned-connectors__stage-row"
              aria-label={`${connector.name} readiness stages`}
            >
              {readiness.stages.slice(0, 4).map((stage) => (
                <span
                  className={`planned-connectors__stage planned-connectors__stage--${stage.state}`}
                  key={stage.id}
                  title={stage.evidence}
                >
                  {stage.label}
                </span>
              ))}
            </div>
            <p className="planned-connectors__manual">
              Today: {connector.safeToday}
            </p>
            <p className="planned-connectors__manual">
              Next: {connector.firstAutomation}
            </p>
            <div className="planned-connectors__meta">
              <span>{connector.setupPhase}</span>
              <span>{connector.statusLabel}</span>
            </div>
          </li>
          );
        })}
      </ul>
    </div>
  );
}

function buildUpgradeIssueMailto(failure: RuntimeUpgradeFailure): string {
  const subject = `Mac AI Switchboard engine update issue (${failure.targetHeadroomVersion}, ${failure.failurePhase})`;
  const diagnosticLines = [
    `App version: ${failure.appVersion}`,
    `Target Headroom: ${failure.targetHeadroomVersion}`,
    failure.fallbackHeadroomVersion
      ? `Fallback running: ${failure.fallbackHeadroomVersion}`
      : null,
    `Failure phase: ${failure.failurePhase}`,
    `Attempts: ${failure.attempts}`,
    `First attempt: ${failure.firstAttemptAt}`,
    `Last attempt: ${failure.lastAttemptAt}`,
    `Rollback restored: ${failure.rollbackRestored ? "yes" : "no"}`,
    "",
    "Error:",
    failure.errorMessage,
  ].filter((line): line is string => line !== null);
  const body =
    "What were you doing when this happened?\n\n\n" +
    "---\n" +
    "Diagnostic info (please keep):\n" +
    diagnosticLines.join("\n");
  return `mailto:support@extraheadroom.com?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
}

interface ProxyVerificationRow {
  clientId: string;
  name: string;
  state: "processing" | "waiting" | "verified";
  message: string;
}

interface ReleaseReadinessReportPayload {
  reportPath: string;
  report: ReleaseReadinessReportSnapshot | null;
}

export default function App() {
  const [dashboard, setDashboard] = useState<DashboardState>(mockDashboard);
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
  const releaseReadinessRows = releaseReadinessRowsFromReport(
    releaseReadinessReport?.report,
  );
  const releaseReadinessCounts =
    releaseReadinessStatusCounts(releaseReadinessRows);
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
  const [showRtkDetails, setShowRtkDetails] = useState(false);
  const [rtkActivityLines, setRtkActivityLines] = useState<string[]>([]);
  const rtkActivityRef = useRef<HTMLPreElement | null>(null);
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
  const [rollbackCopyNotice, setRollbackCopyNotice] = useState<string | null>(
    null,
  );
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
  const [doctorRepairBusy, setDoctorRepairBusy] = useState<string | null>(null);
  const [doctorRepairError, setDoctorRepairError] = useState<string | null>(
    null,
  );
  const [doctorRepairSuccess, setDoctorRepairSuccess] = useState<string | null>(
    null,
  );
  const localOnlyMode = localOnlyModeEnabled();
  const appSemver = appUpdateConfig?.currentVersion ?? packageJson.version;
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
    if (!showRtkDetails || !rtkActivityRef.current) {
      return;
    }
    rtkActivityRef.current.scrollTop = rtkActivityRef.current.scrollHeight;
  }, [showRtkDetails, rtkActivityLines]);

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
      body: `Your ${sourceLabel} usage needs the Headroom ${recommendedLabel} plan, above your current ${paidLabel} plan. Upgrade to keep unlimited optimization.`,
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
      const label = getCurrentWindow().label;
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
      const [runtimeResult, switchboardResult, doctorResult, pricingResult] =
        await Promise.all([
          invoke<RuntimeStatus>("get_runtime_status").catch(() => null),
          invoke<SwitchboardState>("get_switchboard_state").catch(() => null),
          invoke<DoctorReport>("get_doctor_report").catch(() => null),
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
        setStartupCopy("Mac AI Switchboard is ready.");
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
    if (!isLastScreen) return;
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
    if (windowLabel !== "main") {
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
      await navigator.clipboard.writeText(formatUninstallDryRunReport());
      setUninstallCopyNotice("Uninstall dry-run copied.");
      window.setTimeout(() => setUninstallCopyNotice(null), 2500);
    } catch {
      setUninstallCopyNotice("Copy failed. Review the uninstall list manually.");
      window.setTimeout(() => setUninstallCopyNotice(null), 3000);
    }
  }

  useEffect(() => {
    if (activeView !== "home" || !trayWindowFocused) {
      return;
    }

    let active = true;
    const refreshDashboard = () => {
      void loadDashboard()
        .then((next) => {
          if (!active) return;
          applyDashboardIfChanged(next);
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
    if (localOnlyMode) {
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
    if (localOnlyMode) {
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

  function stepPercentSpan(step: string) {
    switch (step) {
      case "Preparing install":
        return 13;
      case "Downloading Python":
        return 13;
      case "Creating environment":
        return 17;
      case "Installing Headroom":
        return 20;
      case "Finalizing":
        return 4;
      default:
        return 8;
    }
  }

  function getStepProgress(progress: BootstrapProgress) {
    if (progress.complete) {
      return 1;
    }
    if (!progress.running || !stepStartedAtMs) {
      return 0;
    }

    const elapsedSeconds = Math.max(0, (Date.now() - stepStartedAtMs) / 1000);
    const eta = Math.max(
      8,
      stepEtaSeedSeconds || progress.currentStepEtaSeconds || 20,
    );
    const linear = Math.min(0.96, elapsedSeconds / eta);

    if (elapsedSeconds <= eta) {
      return linear;
    }

    const overtime = elapsedSeconds - eta;
    const creep = Math.min(0.995, linear + overtime / (eta * 10));
    return creep;
  }

  function animatedOverallPercent(progress: BootstrapProgress) {
    if (progress.complete || progress.failed || !progress.running) {
      return progress.overallPercent;
    }

    const span = stepPercentSpan(progress.currentStep);
    const animated = stepBasePercent + span * getStepProgress(progress);
    return Math.min(99, Math.max(progress.overallPercent, animated));
  }

  function etaCopy(seconds: number, progress: BootstrapProgress) {
    if (!showInstallProgress) {
      return "ETA: starts after install";
    }
    if (progress.complete) {
      return "ETA: complete";
    }
    if (progress.failed) {
      return "ETA: unavailable";
    }

    const elapsedSeconds = stepStartedAtMs
      ? Math.max(0, Math.round((Date.now() - stepStartedAtMs) / 1000))
      : 0;
    const baselineEta = Math.max(stepEtaSeedSeconds, seconds);
    const remainingSeconds = Math.max(0, baselineEta - elapsedSeconds);

    if (remainingSeconds <= 0 && progress.running) {
      return "ETA: finishing up";
    }
    if (remainingSeconds <= 0) {
      return "ETA: --";
    }
    if (remainingSeconds < 60) {
      return `ETA: ${remainingSeconds}s`;
    }
    const mins = Math.floor(remainingSeconds / 60);
    const secs = remainingSeconds % 60;
    return `ETA: ${mins}m ${secs}s`;
  }

  function getConnectorUnavailableReason(connector: ClientConnectorStatus) {
    return connectorControlState(connector).reason;
  }

  function canConfigureConnectorWithoutDetection(
    connector: ClientConnectorStatus,
  ) {
    return !connectorControlState(connector).disabled;
  }

  function getConnectorSupportWarning(connector: ClientConnectorStatus) {
    return connectorSupportWarnings[connector.clientId] ?? null;
  }

  function getConnectorDetectionWarning(connector: ClientConnectorStatus) {
    if (connector.installed) {
      return null;
    }
    return connectorUnavailableReasons[connector.clientId] ?? null;
  }

  function getPlannedConnectorNextStep(
    connector: ClientConnectorStatus,
    plannedConnector: PlannedConnector,
  ) {
    if (!connector.installed) {
      return "Install the tool first, then Mac AI Switchboard will detect it here.";
    }

    if (plannedConnector.setupPhase === "Detect") {
      return "Detected. Keep using RTK-only mode while a reversible routing adapter is researched.";
    }

    if (plannedConnector.setupPhase === "Guide") {
      return "Detected. Guided setup is next so account-specific provider settings stay under your control.";
    }

    return "Detected. Automatic setup waits for backup, restore, and off-mode cleanup coverage.";
  }

  function formatBackendConnectorConfigPlan(
    connector: ClientConnectorStatus,
    plannedConnector: PlannedConnector,
  ) {
    const stepDetails = connector.configCreationStepDetails ?? [];
    const stepLabels = connector.configCreationSteps ?? [];
    if (stepDetails.length === 0 && stepLabels.length === 0) {
      return formatPlannedConnectorConfigCreationPlansMarkdown([
        plannedConnector,
      ]);
    }

    return [
      "# Mac AI Switchboard Connector Config Creation Plan",
      "",
      `## ${connector.name}`,
      "- Automation enabled: no",
      "- Safety note: Backend metadata keeps config creation gated until every step has tests and Doctor evidence.",
      ...(stepDetails.length > 0
        ? stepDetails.map((step) => {
            const evidence = step.requiredEvidence?.length
              ? ` Required evidence: ${step.requiredEvidence.join(" ")}`
              : "";
            return `- ${step.label}: ${step.detail}${evidence}`;
          })
        : stepLabels.map((step) => `- ${step}`)),
    ].join("\n");
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
        const windowVisible = await getCurrentWindow()
          .isVisible()
          .catch(() => false);
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
    try {
      const report = await invoke<DoctorReport>("get_doctor_report");
      setDoctorReport(report);
    } catch {
      setDoctorReport(null);
    }
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
    if (doctorRepairBusy !== null) {
      return;
    }
    setDoctorRepairBusy(action);
    setDoctorRepairError(null);
    setDoctorRepairSuccess(null);
    try {
      const report = await invoke<DoctorReport>("run_doctor_repair", {
        action,
      });
      setDoctorReport(report);
      setDoctorRepairSuccess(
        action === "verify_off_mode"
          ? "Off mode verification refreshed."
          : report.status === "ok" && report.issues.length === 0
            ? "Repair complete. Switchboard looks ready."
            : "Repair finished. Review the remaining Doctor items.",
      );
      await refreshSwitchboardState();
    } catch (error) {
      setDoctorRepairError(
        error instanceof Error ? error.message : "Could not run repair.",
      );
    } finally {
      setDoctorRepairBusy(null);
    }
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
      void maybeFireUrgentPricingNotifications(status);
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
        "Copy failed. Select the command and copy manually.",
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
        "Copy failed. Select the command manually.",
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
        await navigator.clipboard.writeText(releaseReadinessCommand);
        setReleaseReadinessCopyNotice("Release report command copied.");
      }
      window.setTimeout(() => setReleaseReadinessCopyNotice(null), 2000);
    } catch {
      setReleaseReadinessCopyNotice("Copy failed. Select release text manually.");
      window.setTimeout(() => setReleaseReadinessCopyNotice(null), 3000);
    }
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
  async function setCavemanLevel(level: "scoped" | "aggressive") {
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
      setAuthFlowSuccess("Headroom account connected.");
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

    void getCurrentWindow().startDragging();
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
  const headroomVersion = headroomTool?.version ?? "Unknown";
  const lifetimeTotalTokensSent = dashboard.dailySavings.reduce(
    (sum, point) => sum + point.totalTokensSent,
    0,
  );
  const lifetimeTotalTokensBeforeOptimization =
    lifetimeTotalTokensSent + dashboard.lifetimeEstimatedTokensSaved;
  const headroomLifetimeSavingsPct =
    lifetimeTotalTokensBeforeOptimization > 0
      ? (dashboard.lifetimeEstimatedTokensSaved /
          lifetimeTotalTokensBeforeOptimization) *
        100
      : null;
  const rtkAvgSavingsPct =
    runtimeStatus?.rtk.installed && (runtimeStatus.rtk.totalCommands ?? 0) > 0
      ? (runtimeStatus.rtk.avgSavingsPct ?? 0)
      : null;
  const lifetimeDataDays = new Set(
    dashboard.dailySavings
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
                    url: buildUpgradeIssueMailto(upgradeFailure),
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
                      url: buildUpgradeIssueMailto(upgradeFailure),
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

  async function copyManagedDiffPreview(record: ManagedChangeRecord) {
    if (!record.backupPath) {
      setRollbackCopyNotice("No config diff required for that record.");
      window.setTimeout(() => setRollbackCopyNotice(null), 2500);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      const preview = buildManagedConfigDiffPreview({
        record,
        targetPath: firstManagedConfigTarget(record),
        currentManagedBlock: null,
        proposedManagedBlock: sampleManagedBlock(record),
      });
      await navigator.clipboard.writeText(
        formatManagedConfigDiffPreview(preview),
      );
      setRollbackCopyNotice(`${record.owner} dry-run copied.`);
      window.setTimeout(() => setRollbackCopyNotice(null), 2500);
    } catch {
      setRollbackCopyNotice("Copy failed. Review the rollback row manually.");
      window.setTimeout(() => setRollbackCopyNotice(null), 3000);
    }
  }

  if (windowLabel === "launcher" && launcherStage === "install") {
    const stepProgress = Math.round(getStepProgress(bootstrapProgress) * 100);
    const renderPercent = animatedOverallPercent(bootstrapProgress);
    const installComplete =
      bootstrapProgress.complete || dashboard.bootstrapComplete;
    const statusCopy = showInstallProgress
      ? `${bootstrapProgress.message} ${
          bootstrapProgress.running && !bootstrapProgress.complete
            ? `(${stepProgress}% of this step)`
            : ""
        }`.trim()
      : "";

    return (
      <LauncherShell
        shellClassName="intro-shell"
        spinnerClassName="intro-shell__spinner"
        copyClassName="intro-shell__copy intro-shell__copy--first-run"
        onMouseDown={handleLauncherSurfaceMouseDown}
        version={appSemver}
        showSpinner={bootstrapping}
      >
        <h1>
          Mac AI Switchboard keeps coding-agent work lean, local, and
          reversible.
        </h1>
        <div className="intro-shell__checklist">
          <article>
            <strong>Local-first</strong>
            <p>
              Routing, client setup, Doctor repairs, and add-ons run on your
              Mac. Model calls still go to your normal provider accounts.
            </p>
          </article>
          <article>
            <strong>Self-contained runtime</strong>
            <p>
              Installs Headroom helper tools into app-owned storage without
              changing your system Python.
            </p>
          </article>
          <article>
            <strong>Off means off</strong>
            <p>
              Switchboard can remove routing hooks, and Doctor can repair stale
              local setup if a client or proxy drifts.
            </p>
          </article>
        </div>
        {installComplete ? (
          <>
            {runtimeStatus?.running !== true ? (
              <>
                <p className="launcher-install-notice">
                  Starting the local Headroom engine for the first time (this
                  can take 1-2 minutes)…
                </p>
                <button
                  className="primary-button primary-button--large primary-button--install launcher-step1-continue"
                  disabled
                  type="button"
                >
                  Starting engine…
                </button>
              </>
            ) : (
              <>
                <p className="launcher-install-notice">
                  Local switchboard runtime is ready
                </p>
                <button
                  className="primary-button primary-button--large primary-button--success launcher-step1-continue"
                  onClick={() => void handleFirstLaunchContinue()}
                  type="button"
                >
                  Continue
                </button>
              </>
            )}
          </>
        ) : (
          <>
            {!bootstrapping && (
              <p className="install-pre-notice">
                Takes a minute or two to install the local engine.
              </p>
            )}
            <button
              className="primary-button primary-button--large primary-button--install"
              disabled={bootstrapping}
              onClick={() => void handleBootstrap()}
              type="button"
            >
              {bootstrapping
                ? "Installing local engine…"
                : bootstrapProgress.failed
                  ? "Try again"
                  : "Install Mac AI Switchboard"}
            </button>
            {!bootstrapping && (
              <div className="install-disclosure">
                <p className="install-disclosure__lead">
                  Clicking Install will:
                </p>
                <ul className="install-disclosure__list">
                  <li>
                    Download a self-contained Python runtime (~2 GB) to{" "}
                    <code>~/.headroom</code>. Your system Python is untouched.
                  </li>
                  <li>
                    Ask before routing supported coding clients through the
                    local proxy: Claude Code via <code>ANTHROPIC_BASE_URL</code>{" "}
                    and <code>~/.claude/settings.json</code>; Codex via{" "}
                    <code>OPENAI_BASE_URL</code> and a managed provider block in{" "}
                    <code>~/.codex/config.toml</code>.
                  </li>
                  <li>
                    Write timestamped backups before local config edits. Off
                    mode removes routing hooks; Doctor can re-apply or repair
                    stale setup.
                  </li>
                  <li>
                    Keep RTK, Ponytail, MarkItDown, and future Repo Intelligence
                    as optional add-ons you control separately.
                  </li>
                </ul>
              </div>
            )}
          </>
        )}
        <div className="install-progress-shell">
          {showInstallProgress ? (
            <div className="install-progress" aria-live="polite">
              <div className="install-progress__bar-track">
                <div
                  className="install-progress__bar-fill"
                  style={{ width: `${renderPercent}%` }}
                />
              </div>
              <div className="install-progress__meta">
                <p>{statusCopy}</p>
                <span>
                  {etaCopy(
                    bootstrapProgress.currentStepEtaSeconds,
                    bootstrapProgress,
                  )}
                </span>
              </div>
              {bootstrapError ? (
                <p className="install-progress__error">{bootstrapError}</p>
              ) : null}
            </div>
          ) : null}
        </div>
      </LauncherShell>
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
              const plannedConnector =
                connector.supportStatus === "planned"
                  ? getPlannedConnector(connector.clientId)
                  : null;
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
                          "Mac AI Switchboard applies local connector configuration."}
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
            Send one message in each connected tool to verify local routing. If
            the tool was already open, restart it first so it reloads the
            managed config.
          </p>
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
            Mac AI Switchboard is ready
            <br />
            in the menu bar
          </h1>
          {dashboard.launchExperience === "first_run" ? (
            <p>
              Send your first prompt from a connected tool. The switchboard will
              route through the local Headroom engine and track savings
              automatically.
            </p>
          ) : (
            <>
              <p>
                Mac AI Switchboard will trim prompt bloat whenever you use
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
                    {currency(dashboard.lifetimeEstimatedSavingsUsd)}
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
                    {compactNumber(dashboard.lifetimeEstimatedTokensSaved)}
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
            "No coding tools connected — Mac AI Switchboard isn't reducing token use.",
        } as const;
      }
      if (connectorPhase === "verifying") {
        return {
          tone: "starting",
          title:
            "Send a message in a connected tool to verify the connection is working. You may need to restart it first.",
        } as const;
      }
      if (kompressWarming) {
        return {
          tone: "healthy",
          title: "Mac AI Switchboard is running while finishing setup.",
        } as const;
      }
      return {
        tone: "healthy",
        title: "Mac AI Switchboard is running and trimming prompt bloat.",
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
          ? `Mac AI Switchboard is not hooked up right now: ${runtimeIssues.join(", ")}.`
          : "Mac AI Switchboard is not hooked up right now."
        : runtimeIssues.length > 0
          ? `Mac AI Switchboard needs attention: ${runtimeIssues.join(", ")}.`
          : "Mac AI Switchboard is running, but something needs attention.",
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
            return `Mac AI Switchboard is not hooked up right now: ${primaryIssue}.`;
          }
          return `Mac AI Switchboard needs attention: ${primaryIssue}.`;
        })();
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
    rtkAvgSavingsPct !== null
      ? `${percent1(rtkAvgSavingsPct)}% average savings`
      : "Shell output compression";
  const switchboardHeadroomLabel =
    (switchboardState?.enabledClients ?? enabledSwitchboardConnectors).length >
    0
      ? (switchboardState?.enabledClients ?? enabledSwitchboardConnectors)
          .map((connector) => connector.name)
          .join(", ")
      : "No clients enabled";
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
    : "Sign in with email to unlock your 7-day Headroom trial";
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
        message: "Loading your Headroom access...",
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
            "Your 72-hour Headroom access expired. Create an account to extend to 7 days.",
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
          "Headroom account connected. Syncing your trial and plan details...",
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
        message: `${daysLabel} of trial to go. Upgrade to continue using Headroom without limits.`,
        actionLabel: "Upgrade",
        onAction: () => void handleUpgradeAction(upgradeDefaultPlanId),
      };
    }
    return {
      tone: pricingStatus.optimizationAllowed
        ? ("warning" as const)
        : ("expired" as const),
      message: `Trial expired. In the free plan you can only use Headroom for ${weeklyLimitPercentLabel} of your weekly Claude Code / Codex limits. Upgrade to use Headroom without limits.`,
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
            By signing in, you agree to the Mac AI Switchboard Terms of Use
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
      <aside className="tray-sidebar">
        <div className="tray-sidebar__logo">
          <img src={macAiSwitchboardLogo} alt="Mac AI Switchboard" />
        </div>
        <nav className="tray-nav" aria-label="Tray navigation">
          {navItems.map((item) => (
            <button
              key={item.id}
              className={`tray-nav__item${activeView === item.id ? " is-active" : ""}`}
              onMouseDown={() => setActiveView(item.id)}
              type="button"
            >
              <span className="tray-nav__icon" aria-hidden="true">
                <item.icon
                  className="tray-nav__icon-svg"
                  size={26}
                  weight={activeView === item.id ? "fill" : "regular"}
                />
              </span>
              <span className="tray-nav__text">
                <strong>{item.label}</strong>
              </span>
            </button>
          ))}
        </nav>
        <div className="tray-sidebar__footer">
          {!localOnlyMode ? (
            <button
              className={`upgrade-pill${activeView === "upgrade" || activeView === "upgradeAuth" ? " is-active" : ""}`}
              onMouseDown={() => setActiveView("upgrade")}
              type="button"
            >
              Upgrade
            </button>
          ) : null}
          <button
            className={`tray-nav__item${activeView === "settings" ? " is-active" : ""}`}
            onMouseDown={() => setActiveView("settings")}
            type="button"
          >
            <span className="tray-nav__icon" aria-hidden="true">
              <GearSix
                className="tray-nav__icon-svg"
                size={26}
                weight={activeView === "settings" ? "fill" : "regular"}
              />
            </span>
            <span className="tray-nav__text">
              <strong>Settings</strong>
            </span>
          </button>
        </div>
      </aside>

      <section className="tray-panel">
        <div className="tray-content" hidden={activeView !== "home"}>
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
                dashboard.lifetimeEstimatedTokensSaved < 1_000_000 && (
                  <p className="callout-banner__subtitle">
                    Now use your connected tools as normal, and check back later
                    to see how much you are saving by using Headroom.
                  </p>
                )}
              {(calloutBanner.tone === "auto-paused" ||
                calloutBanner.tone === "paused") && (
                <div className="callout-banner__resume">
                  <button
                    type="button"
                    className="callout-banner__action"
                    onClick={() => void handleResumeRuntime()}
                    disabled={resuming}
                  >
                    {resuming ? "Restarting…" : "Resume"}
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
              )}
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
            remoteServicesEnabled={switchboardRemoteServicesEnabled}
            savingsMode={savingsMode}
            savingsModeBusy={savingsModeBusy}
            paused={runtimeStatus?.paused === true}
            resuming={resuming}
            modeBusy={switchboardModeBusy}
            modeError={switchboardModeError}
            onSetMode={(mode) => void handleSetSwitchboardMode(mode)}
            onSetSavingsMode={(mode) => void handleSetSavingsMode(mode)}
            onResume={() => void handleResumeRuntime()}
            onManageClients={() => setActiveView("settings")}
            onManageRtk={() => setActiveView("addons")}
          />

          <SwitchboardDoctorPanel
            report={doctorReport}
            busyAction={doctorRepairBusy}
            error={doctorRepairError}
            successMessage={doctorRepairSuccess}
            onRepair={(action) => void handleDoctorRepair(action)}
          />

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
                Total costs saved (estimate)
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
                {currency(dashboard.lifetimeEstimatedSavingsUsd)}
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
                Total input tokens saved
              </span>
              <div className="stat-value-row">
                <strong className="stat-value--blue">
                  {compactNumber(dashboard.lifetimeEstimatedTokensSaved)}
                </strong>
                {dashboard.outputReduction ? (
                  <OutputReductionChip reduction={dashboard.outputReduction} />
                ) : null}
              </div>
            </article>
          </section>

          <SavingsCalculatorCard
            dashboard={dashboard}
            repoSavings={savingsCalculatorRepoEstimate}
            runtimeStatus={runtimeStatus}
            cavemanSavings={cavemanSavingsEstimate}
            ponytailSavings={ponytailSavingsEstimate}
            markitdownSavings={markitdownSavingsEstimate}
            scope={savingsCalculatorScope}
            onScopeChange={setSavingsCalculatorScope}
          />

          {dashboard.savingsHistoryLoaded || historyLoadTimedOut ? (
            <DailySavingsChart
              data={dashboard.dailySavings}
              hourlyData={dashboard.hourlySavings}
              resetSignal={chartResetSignal}
              chartMode={chartMode}
              setChartMode={setChartMode}
            />
          ) : (
            <div className="savings-chart__skeleton" role="status">
              <p className="loading-copy">Loading savings history…</p>
            </div>
          )}
        </div>

        <div className="tray-content" hidden={activeView !== "usage"}>
          <section className="repo-intelligence-view">
            <header className="repo-intelligence-view__header">
              <div>
                <h1>Usage and Savings</h1>
                <p className="repo-intelligence-view__subtitle">
                  Review token savings, estimated cost savings, source
                  breakdowns, and copyable savings summaries.
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
                  Total costs saved (estimate)
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
                  {currency(dashboard.lifetimeEstimatedSavingsUsd)}
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
                  Total input tokens saved
                </span>
                <div className="stat-value-row">
                  <strong className="stat-value--blue">
                    {compactNumber(dashboard.lifetimeEstimatedTokensSaved)}
                  </strong>
                  {dashboard.outputReduction ? (
                    <OutputReductionChip
                      reduction={dashboard.outputReduction}
                    />
                  ) : null}
                </div>
              </article>
            </section>

            <SavingsCalculatorCard
              dashboard={dashboard}
              repoSavings={savingsCalculatorRepoEstimate}
              runtimeStatus={runtimeStatus}
              cavemanSavings={cavemanSavingsEstimate}
              ponytailSavings={ponytailSavingsEstimate}
              markitdownSavings={markitdownSavingsEstimate}
              scope={savingsCalculatorScope}
              onScopeChange={setSavingsCalculatorScope}
            />

            {dashboard.savingsHistoryLoaded || historyLoadTimedOut ? (
              <DailySavingsChart
                data={dashboard.dailySavings}
                hourlyData={dashboard.hourlySavings}
                resetSignal={chartResetSignal}
                chartMode={chartMode}
                setChartMode={setChartMode}
              />
            ) : (
              <div className="savings-chart__skeleton" role="status">
                <p className="loading-copy">Loading savings history…</p>
              </div>
            )}
          </section>
        </div>

        <div className="tray-content" hidden={activeView !== "doctor"}>
          <section className="repo-intelligence-view">
            <header className="repo-intelligence-view__header">
              <div>
                <h1>Doctor</h1>
                <p className="repo-intelligence-view__subtitle">
                  Inspect Mac AI Switchboard setup, run fixes, copy reports, and
                  repair local routing drift.
                </p>
              </div>
              <span className="repo-intelligence-view__badge">Fixes</span>
            </header>
            <SwitchboardDoctorPanel
              report={doctorReport}
              busyAction={doctorRepairBusy}
              error={doctorRepairError}
              successMessage={doctorRepairSuccess}
              onRepair={(action) => void handleDoctorRepair(action)}
            />
            <DoctorTimelineCard
              events={buildDoctorTimelinePreview(
                doctorReport,
                doctorRepairSuccess,
              )}
            />
          </section>
        </div>

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
                <p className="loading-copy">
                  Enable the Claude Code or Codex connector to scan sessions for
                  learnings.
                </p>
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
                          const refreshLabel = isRunning
                            ? "Scanning…"
                            : "Scan now";
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
                                      <button
                                        type="button"
                                        className={`optimize-project-row__refresh${isRunning ? " is-spinning" : ""}`}
                                        onClick={() =>
                                          void handleRunHeadroomLearn(
                                            "claude",
                                            project.projectPath,
                                          )
                                        }
                                        disabled={disableLearn}
                                        aria-label={refreshLabel}
                                        title={refreshLabel}
                                      >
                                        <ArrowClockwise
                                          weight="bold"
                                          size={12}
                                          aria-hidden="true"
                                        />
                                      </button>
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
                                      <button
                                        type="button"
                                        className={`optimize-project-row__refresh${codexRunning ? " is-spinning" : ""}`}
                                        onClick={() =>
                                          void handleRunHeadroomLearn("codex")
                                        }
                                        disabled={codexDisable}
                                        aria-label={
                                          codexRunning
                                            ? "Scanning…"
                                            : "Scan now"
                                        }
                                        title={
                                          codexRunning
                                            ? "Scanning…"
                                            : "Scan now"
                                        }
                                      >
                                        <ArrowClockwise
                                          weight="bold"
                                          size={12}
                                          aria-hidden="true"
                                        />
                                      </button>
                                    </span>
                                  </small>
                                </span>
                                <div className="optimize-project-row__actions">
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

        <div className="tray-content" hidden={activeView !== "addons"}>
          <section className="addons">
            <header className="addons__header">
              <h1>Addons</h1>
              <p className="addons__subtitle">
                Installable local add-ons reduce token use and keep
                document/context prep under your control.
              </p>
            </header>
            {addonError ? <p className="addons__error">{addonError}</p> : null}
            <ul className="addons__list">
              <AddonCard
                key="rtk"
                name="RTK"
                version={runtimeStatus?.rtk.version}
                installed={runtimeStatus?.rtk.installed === true}
                enabled={runtimeStatus?.rtk.enabled === true}
                description={
                  <>
                    Token-optimizing proxy that auto-rewrites your agent's bash
                    commands.
                    {rtkAvgSavingsPct !== null
                      ? ` ${percent1(rtkAvgSavingsPct)}% avg savings.`
                      : ""}
                  </>
                }
                copy={addonCopy.rtk}
                infoOpen={addonInfoId === "rtk"}
                onToggleInfo={() =>
                  setAddonInfoId(addonInfoId === "rtk" ? null : "rtk")
                }
                busy={addonBusyId === "rtk"}
                busyLabel={addonBusyLabel}
                resultMessage={
                  addonResult?.id === "rtk" ? addonResult.message : null
                }
                onDismissResult={() => setAddonResult(null)}
                sourceUrl={
                  dashboard.tools.find((tool) => tool.id === "rtk")
                    ?.sourceUrl ?? "https://github.com/rtk-ai/rtk"
                }
                onOpenSource={() =>
                  void openExternalLink(
                    dashboard.tools.find((tool) => tool.id === "rtk")
                      ?.sourceUrl ?? "https://github.com/rtk-ai/rtk",
                  )
                }
                connectors={connectors}
                showClients={
                  runtimeStatus?.rtk.installed === true &&
                  runtimeStatus.rtk.enabled === true
                }
                actionsDisabled={
                  rtkBusy || addonBusyId === "rtk" || !runtimeStatus
                }
                onInstall={() => void runAddonAction("install_addon", "rtk")}
                onToggleEnabled={() =>
                  void handleRtkToggle(!runtimeStatus?.rtk.enabled)
                }
                onUninstall={() =>
                  void runAddonAction("uninstall_addon", "rtk")
                }
              >
                {runtimeStatus?.rtk.installed ? (
                  <>
                    <button
                      type="button"
                      className="addon-card__link"
                      onClick={async () => {
                        const next = !showRtkDetails;
                        setShowRtkDetails(next);
                        if (next) {
                          try {
                            const lines = await invoke<string[]>(
                              "get_rtk_activity",
                              { maxLines: 80 },
                            );
                            setRtkActivityLines(lines);
                          } catch {
                            setRtkActivityLines([
                              "Failed to load RTK activity.",
                            ]);
                          }
                        }
                      }}
                    >
                      {showRtkDetails
                        ? "Hide RTK activity"
                        : "Show RTK activity"}
                    </button>
                    {showRtkDetails ? (
                      <pre className="runtime-log" ref={rtkActivityRef}>
                        {rtkActivityLines.length > 0
                          ? rtkActivityLines.join("\n")
                          : "No RTK activity yet."}
                      </pre>
                    ) : null}
                  </>
                ) : null}
              </AddonCard>
              {dashboard.tools
                .filter((tool) => !tool.required && tool.id !== "rtk")
                .map((tool) => {
                  const installed = tool.status !== "not_installed";
                  return (
                    <AddonCard
                      key={tool.id}
                      name={tool.name}
                      version={tool.version}
                      installed={installed}
                      enabled={tool.enabled}
                      description={tool.description}
                      copy={addonCopy[tool.id]}
                      infoOpen={addonInfoId === tool.id}
                      onToggleInfo={() =>
                        setAddonInfoId(addonInfoId === tool.id ? null : tool.id)
                      }
                      busy={addonBusyId === tool.id}
                      busyLabel={addonBusyLabel}
                      resultMessage={
                        addonResult?.id === tool.id ? addonResult.message : null
                      }
                      onDismissResult={() => setAddonResult(null)}
                      sourceUrl={tool.sourceUrl}
                      onOpenSource={() => void openExternalLink(tool.sourceUrl)}
                      connectors={connectors}
                      showClients={installed && tool.enabled}
                      actionsDisabled={addonBusyId === tool.id}
                      onInstall={() =>
                        void runAddonAction("install_addon", tool.id)
                      }
                      onToggleEnabled={() =>
                        void runAddonAction(
                          "set_addon_enabled",
                          tool.id,
                          !tool.enabled,
                        )
                      }
                      onUninstall={() =>
                        void runAddonAction("uninstall_addon", tool.id)
                      }
                    >
                      {tool.id === "caveman" && installed && tool.enabled
                        ? (() => {
                            const level =
                              tool.metadata?.level === "aggressive"
                                ? "aggressive"
                                : "scoped";
                            return (
                              <div
                                className="addon-card__segmented"
                                role="group"
                                aria-label="Caveman level"
                              >
                                {(["scoped", "aggressive"] as const).map(
                                  (item) => (
                                    <button
                                      key={item}
                                      type="button"
                                      className={`addon-card__segment${
                                        level === item ? " is-active" : ""
                                      }`}
                                      disabled={
                                        addonBusyId === "caveman" ||
                                        level === item
                                      }
                                      onClick={() => void setCavemanLevel(item)}
                                    >
                                      {item === "scoped"
                                        ? "Scoped"
                                        : "Aggressive"}
                                    </button>
                                  ),
                                )}
                              </div>
                            );
                          })()
                        : null}
                    </AddonCard>
                  );
                })}
              {plannedAddons.map((addon) => (
                <PlannedAddonCard
                  key={addon.id}
                  addon={addon}
                  onOpenRepoIntelligence={() =>
                    setActiveView("repoIntelligence")
                  }
                />
              ))}
            </ul>
          </section>
        </div>

        <div
          className="tray-content tray-content--upgrade"
          hidden={activeView !== "upgrade"}
        >
          <section className="upgrade-hero">
            <h1>Plans based on your AI subscription</h1>
            <div
              className="upgrade-toggle"
              aria-label="Upgrade audiences"
              role="tablist"
            >
              {[
                { id: "individual" as const, label: "Individual" },
                { id: "teamEnterprise" as const, label: "Team & Enterprise" },
              ].map((audience) => (
                <button
                  key={audience.id}
                  aria-selected={pricingAudience === audience.id}
                  className={`upgrade-toggle__item${pricingAudience === audience.id ? " is-active" : ""}`}
                  onClick={() => {
                    setPricingAudience(audience.id);
                    setUpgradeActionError(null);
                  }}
                  role="tab"
                  type="button"
                >
                  {audience.label}
                </button>
              ))}
            </div>
            {pricingAudience === "individual" ? (
              <div
                className="upgrade-billing-toggle"
                role="group"
                aria-label="Billing period"
              >
                {(["annual", "monthly"] as const).map((period) => (
                  <button
                    key={period}
                    className={`upgrade-billing-toggle__item${billingPeriod === period ? " is-active" : ""}`}
                    onClick={() => setBillingPeriod(period)}
                    type="button"
                  >
                    {period === "annual" ? (
                      <>
                        Annual{" "}
                        <span className="upgrade-billing-toggle__save">
                          Save 33%
                        </span>
                      </>
                    ) : (
                      "Monthly"
                    )}
                  </button>
                ))}
              </div>
            ) : null}
          </section>

          {!pricingStatus?.account?.subscriptionActive ? (
            <>
              <section
                className={`upgrade-trial-callout upgrade-trial-callout--${upgradeTrialCallout.tone}`}
              >
                <div className="upgrade-trial-callout__content">
                  <p className="upgrade-trial-callout__message">
                    {upgradeTrialCallout.message}
                  </p>
                </div>
                {upgradeTrialCallout.actionLabel &&
                upgradeTrialCallout.onAction ? (
                  <button
                    className="primary-button upgrade-trial-callout__button"
                    disabled={
                      authRequestBusy ||
                      authVerifyBusy ||
                      upgradeActionBusy !== null
                    }
                    onClick={() => upgradeTrialCallout.onAction?.()}
                    type="button"
                  >
                    {upgradeTrialCallout.actionLabel}
                  </button>
                ) : null}
              </section>

              {pricingStatus?.launchDiscountActive
                ? (() => {
                    const cohorts = pricingStatus.pricingCohorts ?? [];
                    const active = cohorts.find((c) => c.status === "active");
                    const activeLabel = active?.label ?? "Founder";
                    const pct =
                      pricingStatus.activePercentOff ?? active?.percentOff ?? 0;
                    const spotsLeft = active?.spotsLeft ?? null;
                    const capacity = active?.capacity ?? null;
                    const totalCapacity = cohorts.reduce(
                      (sum, c) => sum + (c.capacity ?? 0),
                      0,
                    );
                    const totalFilled = cohorts.reduce((sum, c) => {
                      const cap = c.capacity ?? 0;
                      if (c.status === "sold_out") return sum + cap;
                      if (c.status === "active")
                        return sum + Math.max(0, cap - (c.spotsLeft ?? 0));
                      return sum;
                    }, 0);
                    const filledPct =
                      totalCapacity > 0
                        ? Math.min(
                            100,
                            Math.round(50 + 50 * (totalFilled / totalCapacity)),
                          )
                        : null;
                    const next =
                      cohorts.find((c) => c.status === "upcoming") ?? null;
                    const stepPricing = getFounderStepPricing(
                      upgradePlansState.featuredPlanId,
                      billingPeriod,
                      pct,
                      next?.percentOff ?? 0,
                    );
                    return (
                      <section
                        className="founder-promo"
                        aria-label="Founder pricing"
                      >
                        <div className="founder-promo__main">
                          <p className="founder-promo__intro">
                            <span
                              className="founder-promo__live"
                              aria-hidden="true"
                            />
                            Launch promotion active. Prices rise as{" "}
                            {activeLabel.toLowerCase()} spots fill.
                          </p>
                          <div className="founder-promo__urgency">
                            <div className="founder-promo__count-row">
                              {spotsLeft != null ? (
                                <>
                                  <span className="founder-promo__count">
                                    {spotsLeft}
                                  </span>
                                  <span className="founder-promo__count-label">
                                    {activeLabel} spots left
                                  </span>
                                </>
                              ) : (
                                <span className="founder-promo__count-label">
                                  {activeLabel} pricing
                                </span>
                              )}
                            </div>
                            {filledPct != null ? (
                              <div
                                className="founder-promo__bar"
                                role="presentation"
                              >
                                <span
                                  className="founder-promo__bar-fill"
                                  style={{ width: `${filledPct}%` }}
                                />
                              </div>
                            ) : null}
                          </div>
                        </div>
                        <div className="founder-promo__offer">
                          <div className="founder-promo__steps">
                            <div className="founder-promo__step founder-promo__step--now">
                              <span className="founder-promo__step-tag">
                                Now
                              </span>
                              <span className="founder-promo__step-pct">
                                {pct}% OFF
                              </span>
                              {stepPricing ? (
                                <span className="founder-promo__step-price">
                                  {stepPricing.now} / month
                                </span>
                              ) : null}
                            </div>
                            {next ? (
                              <div className="founder-promo__step founder-promo__step--next">
                                <span className="founder-promo__step-tag">
                                  Next
                                </span>
                                <span className="founder-promo__step-pct">
                                  {next.percentOff > 0
                                    ? `${next.percentOff}% OFF`
                                    : "Full price"}
                                </span>
                                {stepPricing ? (
                                  <span className="founder-promo__step-price">
                                    {stepPricing.next} / month
                                  </span>
                                ) : null}
                              </div>
                            ) : null}
                          </div>
                          <p className="founder-promo__lock">
                            Your price is locked in for good.
                          </p>
                        </div>
                      </section>
                    );
                  })()
                : null}
            </>
          ) : null}

          <section
            className={`upgrade-plan-grid${visibleUpgradePlans.length === 1 ? " upgrade-plan-grid--single" : ""}`}
          >
            {visibleUpgradePlans.map((plan) => {
              const isFeatured = plan.id === upgradePlansState.featuredPlanId;
              const downgradeButtonClassName =
                plan.ctaTone === "downgrade"
                  ? " upgrade-plan-card__button--downgrade"
                  : "";
              const buttonClassName =
                plan.id === "free"
                  ? `primary-button upgrade-plan-card__button upgrade-plan-card__button--free${downgradeButtonClassName}`
                  : plan.ctaVariant === "primary"
                    ? `primary-button upgrade-plan-card__button${downgradeButtonClassName}`
                    : `secondary-button upgrade-plan-card__button${downgradeButtonClassName}`;

              const isActivePlan = plan.id === activeHeadroomPlanId;
              return (
                <article
                  className={`upgrade-plan-card${isFeatured ? " upgrade-plan-card--featured" : ""}${isActivePlan ? " upgrade-plan-card--active" : ""}`}
                  key={plan.id}
                >
                  <div className="upgrade-plan-card__top">
                    <div className="upgrade-plan-card__title-block">
                      <span
                        className="upgrade-plan-card__icon"
                        aria-hidden="true"
                      >
                        <Sparkle weight={isFeatured ? "fill" : "duotone"} />
                      </span>
                      <div>
                        <h2>
                          {plan.name}
                          {isActivePlan ? (
                            <span className="upgrade-plan-card__active-badge">
                              Active
                            </span>
                          ) : null}
                        </h2>
                        <p>{plan.tagline}</p>
                      </div>
                    </div>
                    {plan.centeredPriceLabel ? (
                      <div className="upgrade-plan-card__price-note">
                        {plan.centeredPriceLabel}
                      </div>
                    ) : (
                      <div className="upgrade-plan-card__price-block">
                        <div>
                          {plan.originalPrice && !activeHeadroomPlanId ? (
                            <div className="upgrade-plan-card__sale-row">
                              <s className="upgrade-plan-card__original-price">
                                {plan.originalPrice}
                              </s>
                              <span className="upgrade-plan-card__sale-badge">
                                {pricingStatus?.activePercentOff ?? 50}% off
                              </span>
                            </div>
                          ) : null}
                          <strong>{plan.price}</strong>
                        </div>
                        <span>
                          {plan.billingLines[0]}
                          <br />
                          {plan.billingLines[1]}
                        </span>
                      </div>
                    )}
                    {plan.purchaseInfo ? (
                      <p className="upgrade-plan-card__purchase-info">
                        {plan.purchaseInfo.cancelAtPeriodEnd &&
                        plan.purchaseInfo.endsOn
                          ? plan.id === "free"
                            ? `Activates on ${plan.purchaseInfo.endsOn}`
                            : `Downgrades to Free on ${plan.purchaseInfo.endsOn}`
                          : isActivePlan
                            ? plan.purchaseInfo.discountPct > 0
                              ? `Renews ${plan.purchaseInfo.paidPerMonthLabel}/mo on ${plan.purchaseInfo.renewsOn} (${plan.purchaseInfo.discountPct}% off)`
                              : `Renews ${plan.price}/mo on ${plan.purchaseInfo.renewsOn}`
                            : null}
                      </p>
                    ) : null}
                  </div>
                  <div className="upgrade-plan-card__action">
                    {plan.id === "enterprise" ? (
                      <form
                        className="upgrade-plan-card__contact-form"
                        onSubmit={(event) => void handleContactSubmit(event)}
                      >
                        <input
                          className="upgrade-plan-card__contact-input"
                          onChange={(event) => {
                            setContactEmail(event.target.value);
                            if (contactSubmitError) {
                              setContactSubmitError(null);
                            }
                            if (contactSubmitSuccess) {
                              setContactSubmitSuccess(null);
                            }
                          }}
                          placeholder="you@company.com"
                          type="email"
                          value={contactEmail}
                        />
                        <textarea
                          className="upgrade-plan-card__contact-textarea"
                          maxLength={2000}
                          onChange={(event) => {
                            setContactMessage(event.target.value);
                            if (contactSubmitError) {
                              setContactSubmitError(null);
                            }
                            if (contactSubmitSuccess) {
                              setContactSubmitSuccess(null);
                            }
                          }}
                          placeholder="Tell us about your team and what you're looking for (optional)"
                          rows={4}
                          value={contactMessage}
                        />
                        <button
                          className={`secondary-button upgrade-plan-card__button upgrade-plan-card__contact-submit${contactEmailValid ? " is-ready" : ""}`}
                          disabled={!contactEmailValid || contactSubmitBusy}
                          type="submit"
                        >
                          {contactSubmitBusy ? "Sending..." : plan.ctaLabel}
                        </button>
                      </form>
                    ) : isActivePlan && plan.purchaseInfo?.cancelAtPeriodEnd ? (
                      <button
                        className={buttonClassName}
                        disabled={reactivateBusy}
                        onClick={() => void handleReactivateSubscription()}
                        type="button"
                      >
                        {reactivateBusy
                          ? "Resuming..."
                          : `Resume ${plan.name} plan`}
                      </button>
                    ) : plan.id === "free" &&
                      plan.purchaseInfo?.cancelAtPeriodEnd ? (
                      <button
                        className={buttonClassName}
                        disabled
                        type="button"
                      >
                        {plan.ctaLabel}
                      </button>
                    ) : (
                      <button
                        className={buttonClassName}
                        disabled={
                          plan.disabled || upgradeActionBusy === plan.id
                        }
                        onClick={() => void handleUpgradeAction(plan.id)}
                        type="button"
                      >
                        {upgradeActionBusy === plan.id
                          ? "Opening..."
                          : plan.ctaLabel}
                      </button>
                    )}
                  </div>

                  {plan.features.length > 0 ? (
                    <div className="upgrade-plan-card__features">
                      <ul>
                        {plan.features.map((feature) => (
                          <li key={feature}>{feature}</li>
                        ))}
                      </ul>
                    </div>
                  ) : null}
                  {plan.id === "enterprise" && contactSubmitError ? (
                    <p className="upgrade-plan-card__contact-status upgrade-plan-card__contact-status--error">
                      {contactSubmitError}
                    </p>
                  ) : null}
                  {plan.id === "enterprise" && contactSubmitSuccess ? (
                    <p className="upgrade-plan-card__contact-status upgrade-plan-card__contact-status--success">
                      {contactSubmitSuccess}
                    </p>
                  ) : null}
                </article>
              );
            })}
          </section>
          {pricingAudience === "individual" &&
          (hasHiddenUpgradePlans || showAllUpgradePlans) ? (
            <button
              className="upgrade-plan-grid__toggle"
              onClick={() => setShowAllUpgradePlans((current) => !current)}
              type="button"
            >
              {showAllUpgradePlans ? "show fewer plans" : "show more plans"}
            </button>
          ) : null}

          {upgradeActionError ? (
            <p className="install-progress__error">{upgradeActionError}</p>
          ) : null}
          {reactivateError ? (
            <p className="install-progress__error">{reactivateError}</p>
          ) : null}
        </div>

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

        <div className="tray-content" hidden={activeView !== "settings"}>
          <section className="panel-stack">
            <article className="soft-card panel-card settings-account-card">
              <div className="settings-account-row">
                <p className="settings-account-copy">
                  Headroom account:{" "}
                  {pricingStatus?.authenticated ? (
                    <>
                      {accountDisplayEmail} <em>({accountPlanName})</em>
                    </>
                  ) : (
                    <em>not signed in</em>
                  )}
                </p>
                {pricingStatus?.authenticated ? (
                  <button
                    className="secondary-button secondary-button--small"
                    onClick={() => void handleSignOutHeadroomAccount()}
                    type="button"
                  >
                    <SignOut size={16} weight="bold" />
                    Sign out
                  </button>
                ) : (
                  <button
                    className="secondary-button secondary-button--small"
                    onClick={() => openUpgradeAuthView()}
                    type="button"
                  >
                    Sign in
                  </button>
                )}
              </div>
              {pricingStatus?.claude?.profileFetchError ? (
                <p className="settings-account-notice">
                  {pricingStatus.claude.profileFetchError}
                </p>
              ) : null}
            </article>

            <article className="soft-card panel-card">
              <div className="panel-card__header">
                <div />
              </div>
              <div className="connector-readiness">
                <div>
                  <span className="connector-readiness__eyebrow">
                    Planned tool readiness
                  </span>
                  <strong>{plannedConnectorReadiness.headline}</strong>
                  <p>{plannedConnectorReadiness.detail}</p>
                </div>
                <div className="connector-readiness__actions">
                  <div
                    className="connector-readiness__metrics"
                    aria-label="Planned connector readiness summary"
                  >
                    <span>
                      <strong>{plannedConnectorReadiness.detectedCount}</strong>
                      detected
                    </span>
                    <span>
                      <strong>
                        {plannedConnectorReadiness.manualOnlyCount}
                      </strong>
                      manual
                    </span>
                    <span>
                      <strong>
                        {plannedConnectorReadiness.notDetectedCount}
                      </strong>
                      missing
                    </span>
                    <span>
                      <strong>
                        {plannedConnectorReadiness.safeTodayCount}
                      </strong>
                      safe now
                    </span>
                    <span>
                      <strong>
                        {plannedConnectorReadiness.automationGateCount}
                      </strong>
                      gates
                    </span>
                  </div>
                  <button
                    type="button"
                    className="connector-readiness__copy"
                    onClick={() =>
                      void copyPlannedConnectorCommand(
                        getPlannedConnectorSetupChecklistScript(),
                        "Planned tool checklist",
                      )
                    }
                  >
                    <Copy size={13} weight="bold" />
                    Copy checks
                  </button>
                  <button
                    type="button"
                    className="connector-readiness__copy"
                    onClick={() =>
                      void copyPlannedConnectorCommand(
                        formatPlannedConnectorConfigCreationPlansMarkdown(),
                        "Connector config plans",
                      )
                    }
                  >
                    <Copy size={13} weight="bold" />
                    Copy config plans
                  </button>
                </div>
              </div>
              <div className="connector-list">
                {sortClientConnectors(
                  aggregateClientConnectors(connectors),
                ).map((connector) => {
                  const connectorLabel =
                    connector.clientId === "claude_code"
                      ? "Claude Code connection"
                      : connector.clientId === "codex"
                        ? "Codex connection"
                        : connector.name;
                  const controlState = connectorControlState(connector);
                  const unavailableReason =
                    getConnectorUnavailableReason(connector);
                  const detectionWarning =
                    getConnectorDetectionWarning(connector);
                  const toggleDisabled =
                    connectorsBusy || controlState.disabled;
                  const plannedConnector =
                    connector.supportStatus === "planned"
                      ? getPlannedConnector(connector.clientId)
                      : null;
                  const plannedSetupGuide = plannedConnector
                    ? getPlannedConnectorSetupGuide(plannedConnector.id)
                    : null;
                  const plannedReadiness = plannedConnector
                    ? getPlannedConnectorReadinessContract(plannedConnector)
                    : null;
                  const plannedReadinessBadges = plannedConnector
                    ? getPlannedConnectorReadinessBadges(plannedConnector)
                    : [];
                  const connectorSetupPhase =
                    connector.setupPhase ??
                    plannedConnector?.setupPhase ??
                    null;
                  const connectorSetupHint =
                    connector.setupHint ?? plannedConnector?.notes ?? null;
                  const compatibilityReport =
                    connectorCompatibilityReport(connector);
                  return (
                    <article
                      className="connector-item"
                      key={connector.clientId}
                    >
                      <div>
                        <h3>
                          <span className="client-logo" aria-hidden="true">
                            {renderConnectorLogo(connector.clientId)}
                          </span>
                          {connectorLabel}
                          {connector.supportStatus === "planned" ? (
                            <span className="connector-item__badge connector-item__badge--planned">
                              Planned
                            </span>
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
                            {connectorSetupHint ??
                              connectorSetupDetails[connector.clientId] ??
                              "Mac AI Switchboard applies local connector configuration."}
                          </p>
                        ) : null}
                        {plannedConnector ? (
                          <div className="connector-plan">
                            <div className="connector-plan__meta">
                              <span>{connectorSetupPhase}</span>
                              <span>
                                {connector.category ??
                                  plannedConnector.category}
                              </span>
                            </div>
                            <p className="connector-plan__target">
                              {plannedConnector.integrationTarget}
                            </p>
                            {plannedReadiness ? (
                              <div className="connector-plan__readiness">
                                <div>
                                  <strong>Readiness contract</strong>
                                  <span>
                                    Next gate:{" "}
                                    {plannedReadiness.stages.find(
                                      (stage) =>
                                        stage.id ===
                                        plannedReadiness.nextBlockedStage,
                                    )?.label ?? "Automation ready"}
                                  </span>
                                </div>
                                <div
                                  className="connector-plan__stage-row"
                                  aria-label={`${connector.name} readiness contract`}
                                >
                                  {plannedReadiness.stages.map((stage) => (
                                    <span
                                      className={`connector-plan__stage connector-plan__stage--${stage.state}`}
                                      key={stage.id}
                                      title={stage.evidence}
                                    >
                                      {stage.label}
                                    </span>
                                  ))}
                                </div>
                              </div>
                            ) : null}
                            {plannedReadinessBadges.length ? (
                              <div
                                className="connector-plan__badges"
                                aria-label={`${connector.name} safety badges`}
                              >
                                {plannedReadinessBadges.map((badge) => (
                                  <span
                                    className={`connector-plan__badge connector-plan__badge--${badge.kind}`}
                                    key={badge.kind}
                                    title={badge.detail}
                                  >
                                    {badge.label}
                                  </span>
                                ))}
                              </div>
                            ) : null}
                            {compatibilityReport ? (
                              <div className="connector-plan__compatibility">
                                <strong>{compatibilityReport.title}</strong>
                                {compatibilityReport.binaryPath ? (
                                  <span>
                                    {compatibilityReport.primaryPathLabel}{" "}
                                    {compatibilityReport.binaryPath}
                                  </span>
                                ) : null}
                                {compatibilityReport.version ? (
                                  <span>
                                    Version {compatibilityReport.version}
                                  </span>
                                ) : null}
                                {compatibilityReport.configSurface ? (
                                  <span>
                                    Config {compatibilityReport.configSurface}
                                  </span>
                                ) : null}
                                {compatibilityReport.routingBlocker ? (
                                  <span>
                                    Blocked{" "}
                                    {compatibilityReport.routingBlocker}
                                  </span>
                                ) : null}
                              </div>
                            ) : null}
                            {connector.detectionSources?.length ||
                            connector.configLocations?.length ||
                            connector.detectionEvidence?.length ||
                            connector.automationGates?.length ||
                            connector.manualWorkflow?.length ||
                            connector.configCreationStepDetails?.length ||
                            connector.configCreationSteps?.length ? (
                              <div className="connector-plan__backend">
                                <strong>Backend checks</strong>
                                {connector.detectionSources?.length ? (
                                  <span>
                                    Detects{" "}
                                    {connector.detectionSources
                                      .slice(0, 3)
                                      .join(", ")}
                                  </span>
                                ) : null}
                                {connector.configLocations?.length ? (
                                  <span>
                                    Watches{" "}
                                    {connector.configLocations
                                      .slice(0, 2)
                                      .join(", ")}
                                  </span>
                                ) : null}
                                {connector.detectionEvidence?.length ? (
                                  <span>
                                    Evidence{" "}
                                    {connector.detectionEvidence
                                      .slice(0, 2)
                                      .join(" · ")}
                                  </span>
                                ) : null}
                                {connector.automationGates?.length ? (
                                  <span>
                                    Gates{" "}
                                    {connector.automationGates
                                      .slice(0, 2)
                                      .join(" · ")}
                                  </span>
                                ) : null}
                                {connector.manualWorkflow?.length ? (
                                  <span>
                                    Manual{" "}
                                    {connector.manualWorkflow
                                      .slice(0, 2)
                                      .join(" · ")}
                                  </span>
                                ) : null}
                                {connector.configCreationSteps?.length ? (
                                  <span>
                                    Config plan{" "}
                                    {(connector.configCreationStepDetails
                                      ?.slice(0, 4)
                                      .map(
                                        (step) =>
                                          `${step.label}: ${step.detail}${
                                            step.requiredEvidence?.length
                                              ? ` Evidence ${step.requiredEvidence.join(" ")}`
                                              : ""
                                          }`,
                                      ) ??
                                      connector.configCreationSteps.slice(
                                        0,
                                        4,
                                      )
                                    ).join(" -> ")}
                                  </span>
                                ) : null}
                              </div>
                            ) : null}
                            <div className="connector-plan__capabilities">
                              {plannedConnector.capabilityRows.map(
                                (capability) => (
                                  <div
                                    className="connector-plan__capability"
                                    key={`${plannedConnector.id}-${capability.label}`}
                                  >
                                    <div>
                                      <strong>{capability.label}</strong>
                                      <span>{capability.detail}</span>
                                    </div>
                                    <span
                                      className={`connector-plan__state connector-plan__state--${capability.state
                                        .toLowerCase()
                                        .replace(/\s+/g, "-")}`}
                                    >
                                      {capability.state}
                                    </span>
                                  </div>
                                ),
                              )}
                            </div>
                            <p className="connector-plan__next">
                              {getPlannedConnectorNextStep(
                                connector,
                                plannedConnector,
                              )}
                            </p>
                            {plannedSetupGuide ? (
                              <div className="connector-plan__guide">
                                <div>
                                  <strong>{plannedSetupGuide.label}</strong>
                                  <code>{plannedSetupGuide.command}</code>
                                </div>
                                <button
                                  type="button"
                                  className="connector-plan__copy"
                                  onClick={() =>
                                    void copyPlannedConnectorCommand(
                                      plannedSetupGuide.command,
                                      connector.name,
                                    )
                                  }
                                  aria-label={`Copy ${connector.name} setup check command`}
                                >
                                  <Copy size={13} weight="bold" />
                                </button>
                                <button
                                  type="button"
                                  className="connector-plan__copy"
                                  onClick={() =>
                                    void copyPlannedConnectorCommand(
                                      formatBackendConnectorConfigPlan(
                                        connector,
                                        plannedConnector,
                                      ),
                                      `${connector.name} config plan`,
                                    )
                                  }
                                  aria-label={`Copy ${connector.name} config creation plan`}
                                >
                                  <Copy size={13} weight="duotone" />
                                </button>
                              </div>
                            ) : null}
                            {plannedSetupGuide ? (
                              <p className="connector-plan__note">
                                {plannedSetupGuide.notes}
                              </p>
                            ) : null}
                          </div>
                        ) : null}
                        {connector.enabled &&
                        !connector.verified &&
                        connector.installed ? (
                          <p className="connector-item__restart">
                            Restart {connector.name} to start routing through
                            Headroom.
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
                          disabled={toggleDisabled}
                          onClick={() =>
                            void toggleConnector(connector, !connector.enabled)
                          }
                          role="switch"
                          title={
                            controlState.reason ??
                            unavailableReason ??
                            undefined
                          }
                          type="button"
                        >
                          <span className="connector-switch__thumb" />
                        </button>
                      </div>
                    </article>
                  );
                })}
              </div>
              {connectorsError ? (
                <p className="install-progress__error">{connectorsError}</p>
              ) : null}
              {plannedConnectorCopyNotice ? (
                <p className="connector-copy-notice">
                  {plannedConnectorCopyNotice}
                </p>
              ) : null}
            </article>

            <article className="soft-card panel-card">
              <div className="panel-card__header">
                <div>
                  <h3>Tools status</h3>
                </div>
              </div>
              <div className="runtime-status">
                <div className="runtime-status__topline">
                  <span className="runtime-status__section-title">
                    Mac AI Switchboard app ({appSemver})
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
            <article className="soft-card panel-card release-readiness-card">
              <div className="panel-card__header">
                <div>
                  <h3>Release readiness</h3>
                  <p>
                    {releaseReadinessItemCount()} checks before a signed DMG can
                    be handed to testers.
                  </p>
                </div>
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
              <div className="release-readiness-card__command">
                <Terminal size={15} weight="duotone" />
                <code>{releaseReadinessCommand}</code>
              </div>
              <p className="release-readiness-card__source">
                {releaseReadinessReport?.report
                  ? `Loaded report: ${releaseReadinessReport.reportPath}`
                  : "Using checklist defaults until dist/release-readiness-report.json exists."}
              </p>
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
            <article className="soft-card panel-card">
              <div className="panel-card__header">
                <div>
                  <h3>Open on login</h3>
                </div>
                <div>
                  <p>
                    Automatically launch Headroom whenever you login or restart.
                  </p>
                </div>
                <div className="connector-item__controls">
                  <button
                    aria-checked={autostartEnabled === true}
                    aria-label={`${autostartEnabled ? "Disable" : "Enable"} open on login`}
                    className={`connector-switch${autostartEnabled ? " is-on" : ""}`}
                    disabled={autostartBusy || autostartEnabled === null}
                    onClick={() =>
                      void handleAutostartToggle(!autostartEnabled)
                    }
                    role="switch"
                    type="button"
                  >
                    <span className="connector-switch__thumb" />
                  </button>
                </div>
              </div>
            </article>

            <article className="soft-card panel-card rollback-center-card">
              <div className="panel-card__header">
                <div>
                  <h3>Rollback Center</h3>
                  <p>
                    Managed local changes Mac AI Switchboard can disclose or
                    undo.
                  </p>
                </div>
              </div>
              <div className="rollback-center-card__list">
                {managedChangeRecords.map((record) => (
                  <div className="rollback-center-card__item" key={record.id}>
                    <div>
                      <strong>{record.owner}</strong>
                      <span>{record.rollback}</span>
                      <span>Marker: {record.markerId}</span>
                      <span>Backup: {record.backupPath ?? "not required"}</span>
                      <span>{record.lastVerifiedLabel}</span>
                      {record.backupPath ? (
                        <div className="rollback-center-card__diff">
                          <span>
                            Dry-run target: {firstManagedConfigTarget(record)}
                          </span>
                          <button
                            className="secondary-button secondary-button--small"
                            onClick={() => void copyManagedDiffPreview(record)}
                            type="button"
                          >
                            Copy dry-run diff
                          </button>
                        </div>
                      ) : null}
                    </div>
                    <span className="rollback-center-card__kind">
                      {record.kind.replace(/_/g, " ")}
                    </span>
                  </div>
                ))}
              </div>
              {rollbackCopyNotice ? (
                <p className="rollback-center-card__notice">
                  {rollbackCopyNotice}
                </p>
              ) : null}
            </article>

            <article className="soft-card panel-card">
              <div className="panel-card__header">
                <div>
                  <h3>Uninstall</h3>
                </div>
              </div>
              <p>
                Reverses Mac AI Switchboard changes: removes routing hooks,
                managed runtime storage, app state, login item, known Keychain
                entries, and managed config blocks. Mac AI Switchboard will quit
                when done.
              </p>
              <button
                className="secondary-button secondary-button--small"
                onClick={() => {
                  setUninstallError(null);
                  setShowUninstallDialog(true);
                }}
                type="button"
              >
                Uninstall Mac AI Switchboard
              </button>
            </article>

            <button
              className="contact-link"
              onClick={() =>
                void invoke("open_external_link", {
                  url: "mailto:support@extraheadroom.com",
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
              Quit Mac AI Switchboard
            </button>
          </section>
        </div>

        {showSavingsInfo && (
          <div
            className="modal-backdrop"
            role="dialog"
            aria-modal="true"
            onClick={() => setShowSavingsInfo(false)}
          >
            <div className="modal-card" onClick={(e) => e.stopPropagation()}>
              <h3>How savings are calculated</h3>
              <p>
                Headroom intercepts and prunes all inputs before sending them to
                Claude or Codex.
              </p>
              <p>Savings = tokens removed &times; API token prices.</p>
              <p>This is an optimistic estimate.</p>
              <p>
                Without Headroom, when tokens are sent to Claude for the first
                time they would be stored in their cache. Once in the cache,
                whenever these same tokens are sent again Claude applies a 90%
                discount to their cost. In our testing, this can reduce the
                actual savings by at most 50%.
              </p>
              <p>
                Even accounting for caching, you've likely saved at least{" "}
                <strong>
                  {currency(dashboard.lifetimeEstimatedSavingsUsd * 0.5)}
                </strong>
                .
              </p>
              <div className="modal-actions">
                <button
                  className="button button--primary"
                  onClick={() => setShowSavingsInfo(false)}
                  type="button"
                >
                  Got it
                </button>
              </div>
            </div>
          </div>
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
                  : `Mac AI Switchboard ${appUpdateAvailable.version} is available`}
              </h3>
              <p>
                {appUpdateReadyToRestart
                  ? "The new version has been installed. Restart Mac AI Switchboard when you are ready to switch over."
                  : "Mac AI Switchboard found a new release in the background. Nothing will install until you confirm it here."}
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
