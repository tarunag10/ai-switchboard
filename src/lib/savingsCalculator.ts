import type { DashboardState } from "./types";
import type { RtkDailyStats } from "./types";
import type { RtkTodayStats } from "./types";
import type { RuntimeStatus } from "./types";
import type { SavingsAttributionEvent } from "./types";
import type { RepoSavingsEstimate } from "./repoIntelligence";

export type SavingsCalculatorScope =
  | "session"
  | "repo"
  | "today"
  | "week"
  | "month"
  | "lifetime";

export type SavingsCalculatorBreakdownKind =
  | "runtime"
  | "command_output"
  | "repo_context"
  | "terse_output"
  | "change_scope"
  | "doc_preprocess";
export type SavingsCalculatorConfidence = "measured" | "estimated" | "inferred";
export type SavingsLedgerSource =
  | "headroom_engine"
  | "rtk"
  | "repo_intelligence"
  | "caveman"
  | "ponytail"
  | "markitdown"
  | "compact_chinese";

export interface AddonSavingsEstimate {
  baselineTokens: number;
  optimizedTokens: number;
  tokensAvoided: number;
  savingsPct: number;
}

export function buildAddonSavingsEstimate(
  baselineTokens: number,
  optimizedTokens: number,
): AddonSavingsEstimate {
  const baseline = Math.max(0, baselineTokens);
  const optimized = Math.max(0, optimizedTokens);
  const tokensAvoided = Math.max(0, baseline - optimized);
  const savingsPct = baseline > 0 ? (tokensAvoided / baseline) * 100 : 0;

  return {
    baselineTokens: baseline,
    optimizedTokens: optimized,
    tokensAvoided,
    savingsPct,
  };
}

// Canonical verbose-vs-optimized handoff templates. Deltas are static and
// auditable here until a runtime measurement source exists for each add-on.
export const CAVEMAN_TEMPLATE_BASELINE_TOKENS = 480;
export const CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS = 180;
export const PONYTAIL_TEMPLATE_BASELINE_TOKENS = 1_400;
export const PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS = 520;
export const MARKITDOWN_TEMPLATE_BASELINE_TOKENS = 3_200;
export const MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS = 900;

export interface SavingsCalculatorSummary {
  scope: SavingsCalculatorScope;
  requests: number;
  savedTokens: number;
  savedUsd: number;
  conservativeSavedUsd: number;
  sentTokens: number;
  beforeTokens: number;
  savingsPct: number | null;
  dataLabel: string;
}

export interface SavingsCalculatorBreakdownRow {
  id: string;
  label: string;
  source: SavingsLedgerSource;
  kind: SavingsCalculatorBreakdownKind;
  confidence: SavingsCalculatorConfidence;
  savedTokens: number;
  savedUsd: number | null;
  detail: string;
}

export interface SavingsCalculatorBreakdownOptions {
  runtimeStatus?: RuntimeStatus | null;
  repoSavings?: RepoSavingsEstimate | null;
  cavemanSavings?: AddonSavingsEstimate | null;
  ponytailSavings?: AddonSavingsEstimate | null;
  markitdownSavings?: AddonSavingsEstimate | null;
  attributionEvents?: SavingsAttributionEvent[];
  rtkToday?: RtkTodayStats | null;
}

export interface SavingsLedgerRow extends SavingsCalculatorBreakdownRow {
  source: SavingsLedgerSource;
  scope: SavingsCalculatorScope;
  recordedAt: string;
  caveat: string;
}

export interface SavingsLedgerSummary {
  scope: SavingsCalculatorScope;
  recordedAt: string;
  rowCount: number;
  measuredTokens: number;
  estimatedTokens: number;
  inferredTokens: number;
  totalTokens: number;
  measuredUsd: number;
  estimatedUsd: number;
}

export interface SavingsLedgerSourceGroup extends SavingsLedgerSummary {
  source: SavingsLedgerSource;
  label: string;
  confidence: SavingsCalculatorConfidence;
}

export type SavingsLedgerConfidenceFilter =
  | "all"
  | SavingsCalculatorConfidence;

export interface FilteredSavingsLedger {
  filter: SavingsLedgerConfidenceFilter;
  rows: SavingsLedgerRow[];
  groups: SavingsLedgerSourceGroup[];
  summary: SavingsLedgerSummary;
}

export interface SavingsLedgerEmptyState {
  title: string;
  detail: string;
}

export interface SavingsAnomalyAlert {
  id: string;
  source: SavingsLedgerSource;
  label: string;
  severity: "warning";
  kind: "output_growth" | "low_savings" | "cost_growth";
  eventCount: number;
  requestDelta: number;
  tokenIncrease: number;
  latestObservedAt: string;
  detail: string;
}

function formatUsd(value: number) {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

function formatTokens(value: number) {
  return new Intl.NumberFormat("en-US").format(Math.round(value));
}

function formatDurationMs(value: number) {
  if (value < 1000) {
    return `${Math.round(value)}ms`;
  }
  return `${new Intl.NumberFormat("en-US", {
    maximumFractionDigits: 1,
  }).format(value / 1000)}s`;
}

function formatPercent(value: number | null) {
  if (value === null) {
    return "waiting for usage";
  }

  return `${new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(value)}%`;
}

const confidenceCaveat: Record<SavingsCalculatorConfidence, string> = {
  measured: "Observed from local counters for this source.",
  estimated: "Estimated from saved history or cost model; not a per-request proof.",
  inferred: "Modelled from a template, context-pack, or workflow delta.",
};

function sourceEvidenceCaveat(
  confidence: SavingsCalculatorConfidence,
  source: SavingsLedgerSource,
) {
  if (confidence === "measured") {
    return confidenceCaveat.measured;
  }

  if (confidence === "estimated") {
    switch (source) {
      case "repo_intelligence":
        return "Estimated from a local Repo Intelligence full-scan vs selected-pack token delta; not provider-spend dollars.";
      case "markitdown":
        return "Estimated from a smoke-tested managed MarkItDown hook or instruction-file change; not a per-document provider bill.";
      case "ponytail":
        return "Estimated from verified Ponytail plugin registration in connected agent hosts; not runtime-measured output.";
      case "caveman":
        return "Estimated from changed Caveman-managed instruction files and the audited terse-output template delta.";
      case "compact_chinese":
        return "Estimated from changed Compact Chinese managed instruction files and the audited terse-output template delta.";
      case "headroom_engine":
      case "rtk":
        return confidenceCaveat.estimated;
    }
  }

  return confidenceCaveat[confidence];
}

const savingsSourceLabels: Record<
  SavingsLedgerSource,
  { id: string; label: string; kind: SavingsCalculatorBreakdownKind }
> = {
  headroom_engine: {
    id: "headroom_attribution_events",
    label: "Headroom",
    kind: "runtime",
  },
  rtk: {
    id: "rtk_attribution_events",
    label: "RTK",
    kind: "command_output",
  },
  repo_intelligence: {
    id: "repo_intelligence_attribution_events",
    label: "Repo Intelligence",
    kind: "repo_context",
  },
  caveman: {
    id: "caveman_attribution_events",
    label: "Caveman",
    kind: "terse_output",
  },
  ponytail: {
    id: "ponytail_attribution_events",
    label: "Ponytail",
    kind: "change_scope",
  },
  markitdown: {
    id: "markitdown_attribution_events",
    label: "MarkItDown",
    kind: "doc_preprocess",
  },
  compact_chinese: {
    id: "compact_chinese_attribution_events",
    label: "Compact Chinese",
    kind: "terse_output",
  },
};

export function savingsCalculatorScopeLabel(scope: SavingsCalculatorScope) {
  switch (scope) {
    case "session":
      return "current app session";
    case "repo":
      return "current repo";
    case "today":
      return "today";
    case "week":
      return "this week";
    case "month":
      return "this month";
    case "lifetime":
      return "lifetime";
  }
}

export function savingsCalculatorScopeDefinition(scope: SavingsCalculatorScope) {
  switch (scope) {
    case "session":
      return "Current app session includes live Headroom and backend attribution counters since this AI Switchboard for Mac launch. It is reset on app restart and is not a repo total.";
    case "repo":
      return "Current repo includes Repo Intelligence context-pack estimates for the indexed repository only. Runtime and RTK traffic are excluded until backend repo-scoped history exists.";
    case "today":
      return "Today includes saved local daily history for the current UTC date plus same-day RTK counters when available.";
    case "week":
      return "This week is the trailing seven-day local history window, including today, plus matching RTK daily rows.";
    case "month":
      return "This month includes saved local history for the current UTC month plus matching RTK daily rows.";
    case "lifetime":
      return "Lifetime includes all saved local Switchboard history and all-time RTK totals on this Mac.";
  }
}

function currentDateKey() {
  return new Date().toISOString().slice(0, 10);
}

function currentMonthKey() {
  return new Date().toISOString().slice(0, 7);
}

function trailingWeekStartDateKey() {
  const date = new Date();
  date.setUTCDate(date.getUTCDate() - 6);
  return date.toISOString().slice(0, 10);
}

function summarizeDailySavings(
  dashboard: DashboardState,
  predicate: (date: string) => boolean,
) {
  return dashboard.dailySavings
    .filter((point) => predicate(point.date))
    .reduce(
      (total, point) => {
        total.savedUsd += point.estimatedSavingsUsd;
        total.savedTokens += point.estimatedTokensSaved;
        total.sentTokens += point.totalTokensSent;
        return total;
      },
      { savedUsd: 0, savedTokens: 0, sentTokens: 0 },
    );
}

function summarizeRtkDailyStats(
  daily: RtkDailyStats[],
  predicate: (date: string) => boolean,
) {
  return daily
    .filter((point) => predicate(point.date))
    .reduce(
      (total, point) => {
        total.savedTokens += point.savedTokens;
        total.commands += point.commands;
        total.inputTokens += Math.max(0, point.inputTokens ?? 0);
        total.outputTokens += Math.max(0, point.outputTokens ?? 0);
        total.totalTimeMs += Math.max(0, point.totalTimeMs ?? 0);
        return total;
      },
      {
        savedTokens: 0,
        commands: 0,
        inputTokens: 0,
        outputTokens: 0,
        totalTimeMs: 0,
      },
    );
}

function rtkSavingsPercent(inputTokens: number, savedTokens: number) {
  return inputTokens > 0 ? (savedTokens / inputTokens) * 100 : null;
}

function rtkMeasuredDetail(
  commands: number,
  savedTokens: number,
  inputTokens: number,
  outputTokens: number,
  totalTimeMs: number,
  scopeLabel: string,
) {
  const parts = [
    commands > 0
      ? `${commands.toLocaleString()} command outputs compressed locally ${scopeLabel}`
      : `Command-output tokens compressed locally ${scopeLabel}`,
  ];
  if (inputTokens > 0 || outputTokens > 0) {
    parts.push(
      `RTK measured ${formatTokens(inputTokens)} input tokens, ${formatTokens(
        outputTokens,
      )} output tokens, and ${formatTokens(savedTokens)} saved tokens`,
    );
  }
  const pct = rtkSavingsPercent(inputTokens, savedTokens);
  if (pct !== null) {
    parts.push(`${formatPercent(pct)} saved vs input`);
  }
  if (totalTimeMs > 0) {
    parts.push(`${formatDurationMs(totalTimeMs)} total RTK processing time`);
  }
  return `${parts.join(". ")}.`;
}

export function buildSavingsCalculatorSummary(
  dashboard: DashboardState,
  scope: SavingsCalculatorScope,
): SavingsCalculatorSummary {
  if (scope === "session") {
    const sentTokens = dashboard.recentUsage.reduce(
      (sum, event) =>
        sum + event.estimatedInputTokens + event.estimatedOutputTokens,
      0,
    );
    const beforeTokens = sentTokens + dashboard.sessionEstimatedTokensSaved;

    return {
      scope,
      requests: dashboard.sessionRequests,
      savedTokens: dashboard.sessionEstimatedTokensSaved,
      savedUsd: dashboard.sessionEstimatedSavingsUsd,
      conservativeSavedUsd: dashboard.sessionEstimatedSavingsUsd * 0.5,
      sentTokens,
      beforeTokens,
      savingsPct:
        beforeTokens > 0
          ? (dashboard.sessionEstimatedTokensSaved / beforeTokens) * 100
          : dashboard.sessionSavingsPct > 0
            ? dashboard.sessionSavingsPct
            : null,
      dataLabel: "Current app session",
    };
  }

  if (scope === "repo") {
    return {
      scope,
      requests: 0,
      savedTokens: 0,
      savedUsd: 0,
      conservativeSavedUsd: 0,
      sentTokens: 0,
      beforeTokens: 0,
      savingsPct: null,
      dataLabel: "Current repo context estimate",
    };
  }

  const scopedDailySavings =
    scope === "today"
      ? summarizeDailySavings(dashboard, (date) => date === currentDateKey())
      : scope === "week"
        ? summarizeDailySavings(
            dashboard,
            (date) =>
              date >= trailingWeekStartDateKey() && date <= currentDateKey(),
          )
      : scope === "month"
        ? summarizeDailySavings(dashboard, (date) =>
            date.startsWith(currentMonthKey()),
          )
        : null;
  const historyReady = dashboard.savingsHistoryLoaded;
  const savedTokens =
    scopedDailySavings?.savedTokens ??
    (historyReady ? dashboard.lifetimeEstimatedTokensSaved : 0);
  const savedUsd =
    scopedDailySavings?.savedUsd ??
    (historyReady ? dashboard.lifetimeEstimatedSavingsUsd : 0);
  const sentTokens =
    scopedDailySavings?.sentTokens ??
    (historyReady
      ? dashboard.dailySavings.reduce((sum, point) => sum + point.totalTokensSent, 0)
      : 0);
  const beforeTokens = sentTokens + savedTokens;
  const savingsPct = beforeTokens > 0 ? (savedTokens / beforeTokens) * 100 : null;

  return {
    scope,
    requests: scope === "lifetime" ? dashboard.lifetimeRequests : 0,
    savedTokens,
    savedUsd,
    conservativeSavedUsd: savedUsd * 0.5,
    sentTokens,
    beforeTokens,
    savingsPct,
    dataLabel:
      scope === "today"
        ? "Tracked switchboard usage today"
        : scope === "week"
          ? "Tracked switchboard usage this week"
        : scope === "month"
          ? "Tracked switchboard usage this month"
          : dashboard.savingsHistoryLoaded
            ? "All tracked switchboard usage"
            : "Waiting for saved local history",
  };
}

export function buildSavingsCalculatorBreakdown(
  dashboard: DashboardState,
  scope: SavingsCalculatorScope,
  options: SavingsCalculatorBreakdownOptions = {},
): SavingsCalculatorBreakdownRow[] {
  const summary = buildSavingsCalculatorSummary(dashboard, scope);
  const rows: SavingsCalculatorBreakdownRow[] = [];

  if (scope !== "repo" || summary.savedTokens > 0 || summary.savedUsd > 0) {
    rows.push({
      id: "headroom",
      label: "Headroom",
      source: "headroom_engine",
      kind: "runtime",
      confidence: scope === "session" ? "measured" : "estimated",
      savedTokens: summary.savedTokens,
      savedUsd: summary.savedUsd,
      detail:
        scope === "session"
          ? "Runtime compression measured in this app session."
          : scope === "repo"
            ? "Runtime savings are not attributed to a repo until backend repo-scoped history ships."
          : "Runtime compression recorded across saved history.",
    });
  }

  const rtkSaved = Math.max(0, options.runtimeStatus?.rtk.totalSaved ?? 0);
  const rtkCommands = Math.max(
    0,
    options.runtimeStatus?.rtk.totalCommands ?? 0,
  );
  const rtkInputTokens = Math.max(
    0,
    options.runtimeStatus?.rtk.totalInput ?? 0,
  );
  const rtkOutputTokens = Math.max(
    0,
    options.runtimeStatus?.rtk.totalOutput ?? 0,
  );
  const rtkTotalTimeMs = Math.max(
    0,
    options.runtimeStatus?.rtk.totalTimeMs ?? 0,
  );
  if (scope === "lifetime" && rtkSaved > 0) {
    rows.push({
      id: "rtk",
      label: "RTK",
      source: "rtk",
      kind: "command_output",
      confidence: "measured",
      savedTokens: rtkSaved,
      savedUsd: null,
      detail: rtkMeasuredDetail(
        rtkCommands,
        rtkSaved,
        rtkInputTokens,
        rtkOutputTokens,
        rtkTotalTimeMs,
        "across all recorded RTK history",
      ),
    });
  }
  const rtkDaily = options.runtimeStatus?.rtk.daily ?? [];
  const rtkScopedDaily =
    scope === "today"
      ? summarizeRtkDailyStats(rtkDaily, (date) => date === currentDateKey())
      : scope === "week"
        ? summarizeRtkDailyStats(
            rtkDaily,
            (date) =>
              date >= trailingWeekStartDateKey() && date <= currentDateKey(),
          )
        : scope === "month"
          ? summarizeRtkDailyStats(rtkDaily, (date) =>
              date.startsWith(currentMonthKey()),
            )
          : {
              savedTokens: 0,
              commands: 0,
              inputTokens: 0,
              outputTokens: 0,
              totalTimeMs: 0,
            };
  const fallbackRtkToday =
    scope === "today" && rtkScopedDaily.savedTokens === 0
      ? {
          savedTokens: Math.max(0, options.rtkToday?.savedTokens ?? 0),
          commands: Math.max(0, options.rtkToday?.commands ?? 0),
          inputTokens: Math.max(0, options.rtkToday?.inputTokens ?? 0),
          outputTokens: Math.max(0, options.rtkToday?.outputTokens ?? 0),
          totalTimeMs: Math.max(0, options.rtkToday?.totalTimeMs ?? 0),
        }
      : {
          savedTokens: 0,
          commands: 0,
          inputTokens: 0,
          outputTokens: 0,
          totalTimeMs: 0,
        };
  const rtkWindowSaved = rtkScopedDaily.savedTokens + fallbackRtkToday.savedTokens;
  const rtkWindowCommands = rtkScopedDaily.commands + fallbackRtkToday.commands;
  const rtkWindowInput =
    rtkScopedDaily.inputTokens + fallbackRtkToday.inputTokens;
  const rtkWindowOutput =
    rtkScopedDaily.outputTokens + fallbackRtkToday.outputTokens;
  const rtkWindowTime =
    rtkScopedDaily.totalTimeMs + fallbackRtkToday.totalTimeMs;
  if (
    (scope === "today" || scope === "week" || scope === "month") &&
    rtkWindowSaved > 0
  ) {
    const scopeLabel = savingsCalculatorScopeLabel(scope);
    rows.push({
      id: `rtk_${scope}`,
      label: `RTK ${scopeLabel}`,
      source: "rtk",
      kind: "command_output",
      confidence: "measured",
      savedTokens: rtkWindowSaved,
      savedUsd: null,
      detail: rtkMeasuredDetail(
        rtkWindowCommands,
        rtkWindowSaved,
        rtkWindowInput,
        rtkWindowOutput,
        rtkWindowTime,
        scopeLabel,
      ),
    });
  }

  const repoSaved = Math.max(
    0,
    options.repoSavings?.bestPackTokensAvoided ?? 0,
  );
  if (repoSaved > 0 && scope !== "today" && scope !== "month") {
    rows.push({
      id: "repo_intelligence",
      label: "Repo Intelligence",
      source: "repo_intelligence",
      kind: "repo_context",
      confidence: "estimated",
      savedTokens: repoSaved,
      savedUsd: null,
      detail: `${options.repoSavings?.bestPack?.title ?? "Best context pack"} avoids a broad full-repo scan; graph summary points agents at hubs, entrypoints, and tests.`,
    });
  }

  const cavemanSaved = Math.max(0, options.cavemanSavings?.tokensAvoided ?? 0);
  if (cavemanSaved > 0) {
    rows.push({
      id: "caveman",
      label: "Caveman",
      source: "caveman",
      kind: "terse_output",
      confidence: "inferred",
      savedTokens: cavemanSaved,
      savedUsd: null,
      detail:
        "Terse handoff template vs a verbose baseline (inferred, not yet runtime-measured).",
    });
  }

  const ponytailSaved = Math.max(0, options.ponytailSavings?.tokensAvoided ?? 0);
  if (ponytailSaved > 0) {
    rows.push({
      id: "ponytail",
      label: "Ponytail",
      source: "ponytail",
      kind: "change_scope",
      confidence: "inferred",
      savedTokens: ponytailSaved,
      savedUsd: null,
      detail:
        "Smaller change slices avoid broad re-reads vs an unbounded rewrite baseline.",
    });
  }

  const markitdownSaved = Math.max(
    0,
    options.markitdownSavings?.tokensAvoided ?? 0,
  );
  if (markitdownSaved > 0) {
    rows.push({
      id: "markitdown",
      label: "MarkItDown",
      source: "markitdown",
      kind: "doc_preprocess",
      confidence: "estimated",
      savedTokens: markitdownSaved,
      savedUsd: null,
      detail:
        "Markdown extract vs re-attaching the full source document each turn; the managed converter is smoke-tested before integration is enabled.",
    });
  }

  const backendRows =
    scope === "session"
      ? backendAttributionBreakdownRows(options.attributionEvents ?? [])
      : [];
  if (backendRows.length === 0) {
    return rows;
  }

  const backendSources = new Set(backendRows.map((row) => row.source));
  return [
    ...backendRows,
    ...rows.filter((row) => !backendSources.has(row.source)),
  ];
}

function backendAttributionBreakdownRows(
  events: SavingsAttributionEvent[],
): SavingsCalculatorBreakdownRow[] {
  const durableSessionEvents = positiveSessionAttributionEvents(events);
  return Object.entries(savingsSourceLabels)
    .flatMap(([source, meta]) => {
      const sourceEvents = durableSessionEvents.filter(
        (event) => event.source === source,
      );
      if (sourceEvents.length === 0) return [];
      const strongestConfidence = strongestAttributionConfidence(sourceEvents);
      const savedTokens = sourceEvents.reduce(
        (sum, event) => sum + Math.max(0, event.deltaTokensSaved),
        0,
      );
      const savedUsd = sourceEvents.reduce(
        (sum, event) => sum + Math.max(0, event.deltaUsd),
        0,
      );
      const requests = sourceEvents.reduce(
        (sum, event) => sum + Math.max(0, event.requestDelta),
        0,
      );
      const firstEvidence = sourceEvents
        .flatMap((event) => event.evidence)
        .filter(Boolean)[0];
      const evidenceDetail = firstEvidence ? ` Evidence: ${firstEvidence}` : "";
      return [
        {
          id: meta.id,
          label: meta.label,
          source: source as SavingsLedgerSource,
          kind: meta.kind,
          confidence: strongestConfidence,
          savedTokens,
          savedUsd: savedUsd > 0 ? savedUsd : null,
          detail: `${sourceEvents.length.toLocaleString()} ${strongestConfidence} ${meta.label} session event${sourceEvents.length === 1 ? "" : "s"} across ${requests.toLocaleString()} ${source === "rtk" ? "command" : "request"}${requests === 1 ? "" : "s"}.${evidenceDetail}`,
        },
      ];
    })
    .sort((left, right) => left.label.localeCompare(right.label));
}

export function buildSavingsLedgerRows(
  dashboard: DashboardState,
  scope: SavingsCalculatorScope,
  recordedAt: string,
  options: SavingsCalculatorBreakdownOptions = {},
): SavingsLedgerRow[] {
  const rows = buildSavingsCalculatorBreakdown(dashboard, scope, options).map(
    (row) => ({
      ...row,
      source: row.source,
      scope,
      recordedAt,
      caveat: sourceEvidenceCaveat(row.confidence, row.source),
    }),
  );

  if (scope !== "session") {
    return rows;
  }

  const backendRows = backendAttributionRows(
    options.attributionEvents ?? [],
    scope,
    recordedAt,
  );
  if (backendRows.length === 0) {
    return rows;
  }

  const backendSources = new Set(backendRows.map((row) => row.source));
  return [
    ...backendRows,
    ...rows.filter((row) => !backendSources.has(row.source)),
  ];
}

function backendAttributionRows(
  events: SavingsAttributionEvent[],
  scope: SavingsCalculatorScope,
  recordedAt: string,
): SavingsLedgerRow[] {
  const durableSessionEvents = positiveSessionAttributionEvents(events);

  return Object.entries(savingsSourceLabels)
    .flatMap(([source, meta]) => {
      const sourceEvents = durableSessionEvents.filter(
        (event) => event.source === source,
      );
      if (sourceEvents.length === 0) {
        return [];
      }
      const strongestConfidence = strongestAttributionConfidence(sourceEvents);
      const savedTokens = sourceEvents.reduce(
        (sum, event) => sum + Math.max(0, event.deltaTokensSaved),
        0,
      );
      const savedUsd = sourceEvents.reduce(
        (sum, event) => sum + Math.max(0, event.deltaUsd),
        0,
      );
      const requests = sourceEvents.reduce(
        (sum, event) => sum + Math.max(0, event.requestDelta),
        0,
      );
      const observedAtValues = sourceEvents
        .map((event) => event.observedAt)
        .sort();
      const latestObservedAt =
        observedAtValues[observedAtValues.length - 1] ?? recordedAt;
      const eventCount = sourceEvents.length;
      const eventEvidence = sourceEvents
        .flatMap((event) => event.evidence)
        .filter(Boolean);
      const evidenceDetail =
        eventEvidence.length > 0 ? ` Evidence: ${eventEvidence[0]}` : "";

      return [
        {
          id: meta.id,
          label: meta.label,
          source: source as SavingsLedgerSource,
          kind: meta.kind,
          confidence: strongestConfidence,
          savedTokens,
          savedUsd: savedUsd > 0 ? savedUsd : null,
          detail: `${eventCount.toLocaleString()} ${strongestConfidence} ${meta.label} session event${eventCount === 1 ? "" : "s"} across ${requests.toLocaleString()} ${source === "rtk" ? "command" : "request"}${requests === 1 ? "" : "s"}.${evidenceDetail}`,
          scope,
          recordedAt: latestObservedAt,
          caveat:
            strongestConfidence === "measured"
              ? "Observed from append-only backend attribution events."
              : sourceEvidenceCaveat(
                  strongestConfidence,
                  source as SavingsLedgerSource,
                ),
        },
      ];
    })
    .sort((left, right) => left.label.localeCompare(right.label));
}

function positiveSessionAttributionEvents(events: SavingsAttributionEvent[]) {
  return events.filter(
    (event) =>
      event.scope === "session" &&
      (event.deltaTokensSaved > 0 ||
        event.deltaUsd > 0 ||
        event.requestDelta > 0),
  );
}

function strongestAttributionConfidence(events: SavingsAttributionEvent[]) {
  const confidenceRank: Record<SavingsCalculatorConfidence, number> = {
    measured: 3,
    estimated: 2,
    inferred: 1,
  };
  return events.reduce(
    (strongest, event) =>
      confidenceRank[event.confidence] > confidenceRank[strongest]
        ? event.confidence
        : strongest,
    "inferred" as SavingsCalculatorConfidence,
  );
}

export function buildSavingsAnomalyAlerts(
  events: SavingsAttributionEvent[],
  scope: SavingsCalculatorScope,
  recordedAt: string,
): SavingsAnomalyAlert[] {
  if (scope !== "session") {
    return [];
  }

  const grouped = new Map<SavingsLedgerSource, SavingsAttributionEvent[]>();
  for (const event of events) {
    if (event.scope !== "session") {
      continue;
    }

    const sourceEvents = grouped.get(event.source) ?? [];
    sourceEvents.push(event);
    grouped.set(event.source, sourceEvents);
  }

  return [...grouped.entries()]
    .flatMap(([source, sourceEvents]) => {
      const meta = savingsSourceLabels[source];
      const observedAtValues = sourceEvents
        .map((event) => event.observedAt)
        .sort();
      const latestObservedAt =
        observedAtValues[observedAtValues.length - 1] ?? recordedAt;
      const evidence = sourceEvents
        .flatMap((event) => event.evidence)
        .filter(Boolean);
      const evidenceDetail =
        evidence.length > 0 ? ` Evidence: ${evidence[0]}` : "";
      const requestDelta = sourceEvents.reduce(
        (sum, event) => sum + Math.max(0, event.requestDelta),
        0,
      );
      const alerts: SavingsAnomalyAlert[] = [];

      const outputGrowthEvents = sourceEvents.filter(
        (event) => event.deltaTokensSaved < 0,
      );
      const tokenIncrease = sourceEvents.reduce(
        (sum, event) => sum + Math.max(0, -event.deltaTokensSaved),
        0,
      );
      if (tokenIncrease > 0) {
        alerts.push({
          id: `${source}_output_growth`,
          source,
          label: meta.label,
          severity: "warning",
          kind: "output_growth",
          eventCount: outputGrowthEvents.length,
          requestDelta,
          tokenIncrease,
          latestObservedAt,
          detail: `${meta.label} output grew by ${formatTokens(tokenIncrease)} token${Math.round(tokenIncrease) === 1 ? "" : "s"} across ${requestDelta.toLocaleString()} ${source === "rtk" ? "command" : "request"}${requestDelta === 1 ? "" : "s"}.${evidenceDetail}`,
        });
      }

      const positiveEvents = sourceEvents.filter(
        (event) => event.deltaTokensSaved >= 0,
      );
      const totalTokensSent = positiveEvents.reduce(
        (sum, event) => sum + Math.max(0, event.totalTokensSent),
        0,
      );
      const totalTokensSaved = positiveEvents.reduce(
        (sum, event) => sum + Math.max(0, event.deltaTokensSaved),
        0,
      );
      const savingsRatio =
        totalTokensSent + totalTokensSaved > 0
          ? totalTokensSaved / (totalTokensSent + totalTokensSaved)
          : 0;
      if (totalTokensSent >= 50_000 && savingsRatio > 0 && savingsRatio < 0.02) {
        alerts.push({
          id: `${source}_low_savings`,
          source,
          label: meta.label,
          severity: "warning",
          kind: "low_savings",
          eventCount: positiveEvents.length,
          requestDelta,
          tokenIncrease: totalTokensSent,
          latestObservedAt,
          detail: `${meta.label} saved only ${(savingsRatio * 100).toFixed(1)}% across ${formatTokens(totalTokensSent)} sent tokens. Review thresholds or switch modes before heavier traffic.${evidenceDetail}`,
        });
      }

      const negativeUsd = sourceEvents.reduce(
        (sum, event) => sum + Math.max(0, -event.deltaUsd),
        0,
      );
      if (negativeUsd > 0) {
        alerts.push({
          id: `${source}_cost_growth`,
          source,
          label: meta.label,
          severity: "warning",
          kind: "cost_growth",
          eventCount: sourceEvents.filter((event) => event.deltaUsd < 0).length,
          requestDelta,
          tokenIncrease: Math.round(negativeUsd * 100),
          latestObservedAt,
          detail: `${meta.label} cost estimate increased by ${formatUsd(negativeUsd)} in this session. Check provider routing and attribution evidence.${evidenceDetail}`,
        });
      }

      return alerts;
    })
    .sort((left, right) => right.tokenIncrease - left.tokenIncrease);
}

export function formatSavingsAnomalyAlerts(alerts: SavingsAnomalyAlert[]) {
  if (alerts.length === 0) {
    return "Anomalies: none detected.";
  }

  return [
    "Anomalies:",
    ...alerts.map(
      (alert) =>
        `- ${alert.source}: ${alert.detail} Latest: ${alert.latestObservedAt}`,
    ),
  ].join("\n");
}

export function summarizeSavingsLedgerRows(
  rows: SavingsLedgerRow[],
  scope: SavingsCalculatorScope,
  recordedAt: string,
): SavingsLedgerSummary {
  return rows.reduce(
    (summary, row) => {
      summary.rowCount += 1;
      summary.totalTokens += row.savedTokens;
      if (row.confidence === "measured") {
        summary.measuredTokens += row.savedTokens;
        summary.measuredUsd += row.savedUsd ?? 0;
      } else if (row.confidence === "estimated") {
        summary.estimatedTokens += row.savedTokens;
        summary.estimatedUsd += row.savedUsd ?? 0;
      } else {
        summary.inferredTokens += row.savedTokens;
      }
      return summary;
    },
    {
      scope,
      recordedAt,
      rowCount: 0,
      measuredTokens: 0,
      estimatedTokens: 0,
      inferredTokens: 0,
      totalTokens: 0,
      measuredUsd: 0,
      estimatedUsd: 0,
    } satisfies SavingsLedgerSummary,
  );
}

export function groupSavingsLedgerRowsBySource(
  rows: SavingsLedgerRow[],
  scope: SavingsCalculatorScope,
  recordedAt: string,
): SavingsLedgerSourceGroup[] {
  const groups = new Map<SavingsLedgerSource, SavingsLedgerRow[]>();
  for (const row of rows) {
    const sourceRows = groups.get(row.source) ?? [];
    sourceRows.push(row);
    groups.set(row.source, sourceRows);
  }

  return [...groups.entries()].map(([source, sourceRows]) => {
    const summary = summarizeSavingsLedgerRows(sourceRows, scope, recordedAt);
    const confidences = new Set(sourceRows.map((row) => row.confidence));
    const confidence: SavingsCalculatorConfidence = confidences.has("measured")
      ? "measured"
      : confidences.has("estimated")
        ? "estimated"
        : "inferred";

    return {
      ...summary,
      source,
      label: sourceRows[0]?.label ?? source,
      confidence,
    };
  });
}

export function filterSavingsLedgerRowsByConfidence(
  rows: SavingsLedgerRow[],
  filter: SavingsLedgerConfidenceFilter,
) {
  return filter === "all"
    ? rows
    : rows.filter((row) => row.confidence === filter);
}

export function buildFilteredSavingsLedger(
  rows: SavingsLedgerRow[],
  scope: SavingsCalculatorScope,
  recordedAt: string,
  filter: SavingsLedgerConfidenceFilter,
): FilteredSavingsLedger {
  const filteredRows = filterSavingsLedgerRowsByConfidence(rows, filter);

  return {
    filter,
    rows: filteredRows,
    groups: groupSavingsLedgerRowsBySource(filteredRows, scope, recordedAt),
    summary: summarizeSavingsLedgerRows(filteredRows, scope, recordedAt),
  };
}

export function getSavingsLedgerEmptyState(
  allRowCount: number,
  filter: SavingsLedgerConfidenceFilter,
): SavingsLedgerEmptyState {
  if (allRowCount === 0) {
    return {
      title: "No savings ledger rows yet",
      detail:
        "Run a connected agent, index a repo, or enable an add-on estimate to populate measured, estimated, or inferred rows.",
    };
  }

  return {
    title: "No matching ledger rows",
    detail:
      filter === "all"
        ? "No sources match the current ledger view."
        : `No ${filter} rows match this ledger view. Change the confidence filter to see other sources.`,
  };
}

export function formatSavingsLedgerConfidenceBreakdown(
  summary: Pick<
    SavingsLedgerSummary,
    "measuredTokens" | "estimatedTokens" | "inferredTokens"
  >,
) {
  return [
    `${formatTokens(summary.measuredTokens)} measured`,
    `${formatTokens(summary.estimatedTokens)} estimated`,
    `${formatTokens(summary.inferredTokens)} inferred`,
  ].join(" · ");
}

export function formatSavingsLedgerAttributionSummary(
  summary: Pick<
    SavingsLedgerSummary,
    "totalTokens" | "measuredTokens" | "estimatedTokens" | "inferredTokens"
  >,
) {
  if (summary.totalTokens <= 0) {
    return "No attributed savings yet.";
  }

  const percent = (value: number) =>
    `${new Intl.NumberFormat("en-US", {
      minimumFractionDigits: 0,
      maximumFractionDigits: 1,
    }).format((value / summary.totalTokens) * 100)}%`;

  return [
    `${percent(summary.measuredTokens)} measured`,
    `${percent(summary.estimatedTokens)} estimated`,
    `${percent(summary.inferredTokens)} inferred`,
  ].join(" · ");
}

export function formatSavingsLedgerShareText(
  rows: SavingsLedgerRow[],
  scope: SavingsCalculatorScope,
  recordedAt: string,
  filter: SavingsLedgerConfidenceFilter = "all",
  anomalyAlerts: SavingsAnomalyAlert[] = [],
) {
  const summary = summarizeSavingsLedgerRows(rows, scope, recordedAt);
  const scopeLabel = savingsCalculatorScopeLabel(scope);
  const rowLines =
    rows.length > 0
      ? rows.map((row) => {
          const usdPart =
            row.savedUsd === null ? "" : ` / ${formatUsd(row.savedUsd)}`;
          return `- ${row.source}: ${row.label} (${row.confidence}, ${savingsCalculatorScopeLabel(row.scope)}, ${row.recordedAt}) saved ${formatTokens(row.savedTokens)} tokens${usdPart}. Evidence: ${row.detail} Caveat: ${row.caveat}`;
        })
      : ["- No ledger rows yet."];

  return [
    `AI Switchboard savings ledger (${scopeLabel})`,
    `Recorded: ${recordedAt}`,
    `Confidence filter: ${filter === "all" ? "all rows" : filter}`,
    `Scope definition: ${savingsCalculatorScopeDefinition(scope)}`,
    `Rows: ${formatTokens(summary.rowCount)}`,
    `Total tokens: ${formatTokens(summary.totalTokens)}`,
    `Measured tokens: ${formatTokens(summary.measuredTokens)} / ${formatUsd(summary.measuredUsd)}`,
    `Estimated tokens: ${formatTokens(summary.estimatedTokens)} / ${formatUsd(summary.estimatedUsd)}`,
    `Inferred tokens: ${formatTokens(summary.inferredTokens)}`,
    `Attribution: ${formatSavingsLedgerAttributionSummary(summary)}`,
    formatSavingsAnomalyAlerts(anomalyAlerts),
    "Equation per row: saved tokens come from each source's before/after or counter delta; see Evidence on each row.",
    "Confidence labels are not interchangeable: inferred rows are never reported as measured.",
    "Rows:",
    ...rowLines,
  ].join("\n");
}

export function formatSavingsCalculatorShareText(
  summary: SavingsCalculatorSummary,
  rows: SavingsCalculatorBreakdownRow[],
) {
  const scopeLabel = savingsCalculatorScopeLabel(summary.scope);
  const sourceLines = rows.map((row) => {
    const usdPart =
      row.savedUsd === null ? "" : ` / ${formatUsd(row.savedUsd)}`;
    return `- ${row.source}: ${row.label} (${row.confidence}) saved ${formatTokens(row.savedTokens)} tokens${usdPart}`;
  });

  return [
    `AI Switchboard savings (${scopeLabel})`,
    `Scope definition: ${savingsCalculatorScopeDefinition(summary.scope)}`,
    `Saved: ${formatTokens(summary.savedTokens)} tokens / ${formatUsd(summary.savedUsd)}`,
    `Requests: ${formatTokens(summary.requests)}`,
    `Reduction: ${formatPercent(summary.savingsPct)}`,
    `Likely at least: ${formatUsd(summary.conservativeSavedUsd)}`,
    `Equation: before ${formatTokens(summary.beforeTokens)} - sent ${formatTokens(summary.sentTokens)} = saved ${formatTokens(summary.savedTokens)}`,
    "Confidence: measured = observed local counters; estimated = saved history or cost estimate; inferred = modelled template or context-pack delta.",
    "Sources:",
    ...(sourceLines.length > 0 ? sourceLines : ["- Waiting for usage"]),
  ].join("\n");
}
