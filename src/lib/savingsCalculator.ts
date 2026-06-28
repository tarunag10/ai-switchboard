import type { DashboardState } from "./types";
import type { RuntimeStatus } from "./types";
import type { RepoSavingsEstimate } from "./repoIntelligence";

export type SavingsCalculatorScope =
  | "session"
  | "repo"
  | "today"
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
  | "markitdown";

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

export function savingsCalculatorScopeLabel(scope: SavingsCalculatorScope) {
  switch (scope) {
    case "session":
      return "current app session";
    case "repo":
      return "current repo";
    case "today":
      return "today";
    case "month":
      return "this month";
    case "lifetime":
      return "lifetime";
  }
}

function currentDateKey() {
  return new Date().toISOString().slice(0, 10);
}

function currentMonthKey() {
  return new Date().toISOString().slice(0, 7);
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
      : scope === "month"
        ? summarizeDailySavings(dashboard, (date) =>
            date.startsWith(currentMonthKey()),
          )
        : null;
  const savedTokens =
    scopedDailySavings?.savedTokens ?? dashboard.lifetimeEstimatedTokensSaved;
  const savedUsd =
    scopedDailySavings?.savedUsd ?? dashboard.lifetimeEstimatedSavingsUsd;
  const sentTokens =
    scopedDailySavings?.sentTokens ??
    dashboard.dailySavings.reduce((sum, point) => sum + point.totalTokensSent, 0);
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
        : scope === "month"
          ? "Tracked switchboard usage this month"
          : dashboard.savingsHistoryLoaded
            ? "All tracked switchboard usage"
            : "All recorded usage",
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
  if (scope === "lifetime" && rtkSaved > 0) {
    rows.push({
      id: "rtk",
      label: "RTK",
      source: "rtk",
      kind: "command_output",
      confidence: "measured",
      savedTokens: rtkSaved,
      savedUsd: null,
      detail:
        rtkCommands > 0
          ? `${rtkCommands.toLocaleString()} command outputs compressed locally.`
          : "Command-output tokens compressed locally.",
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
      confidence: "inferred",
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
      confidence: "inferred",
      savedTokens: markitdownSaved,
      savedUsd: null,
      detail:
        "Markdown extract vs re-attaching the full source document each turn.",
    });
  }

  return rows;
}

export function buildSavingsLedgerRows(
  dashboard: DashboardState,
  scope: SavingsCalculatorScope,
  recordedAt: string,
  options: SavingsCalculatorBreakdownOptions = {},
): SavingsLedgerRow[] {
  return buildSavingsCalculatorBreakdown(dashboard, scope, options).map(
    (row) => ({
      ...row,
      source: row.source,
      scope,
      recordedAt,
      caveat: confidenceCaveat[row.confidence],
    }),
  );
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

export function formatSavingsLedgerShareText(
  rows: SavingsLedgerRow[],
  scope: SavingsCalculatorScope,
  recordedAt: string,
) {
  const summary = summarizeSavingsLedgerRows(rows, scope, recordedAt);
  const scopeLabel = savingsCalculatorScopeLabel(scope);
  const rowLines =
    rows.length > 0
      ? rows.map((row) => {
          const usdPart =
            row.savedUsd === null ? "" : ` / ${formatUsd(row.savedUsd)}`;
          return `- ${row.source}: ${row.label} (${row.confidence}) saved ${formatTokens(row.savedTokens)} tokens${usdPart}. ${row.caveat}`;
        })
      : ["- No ledger rows yet."];

  return [
    `Mac AI Switchboard savings ledger (${scopeLabel})`,
    `Recorded: ${recordedAt}`,
    "Scopes: session uses live app counters; repo uses Repo Intelligence context estimates; today/month/lifetime use saved local history.",
    `Rows: ${formatTokens(summary.rowCount)}`,
    `Total tokens: ${formatTokens(summary.totalTokens)}`,
    `Measured tokens: ${formatTokens(summary.measuredTokens)} / ${formatUsd(summary.measuredUsd)}`,
    `Estimated tokens: ${formatTokens(summary.estimatedTokens)} / ${formatUsd(summary.estimatedUsd)}`,
    `Inferred tokens: ${formatTokens(summary.inferredTokens)}`,
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
    `Mac AI Switchboard savings (${scopeLabel})`,
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
