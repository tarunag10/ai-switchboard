import type { DashboardState } from "./types";
import type { RuntimeStatus } from "./types";
import type { RepoSavingsEstimate } from "./repoIntelligence";

export type SavingsCalculatorScope = "session" | "overall";

export type SavingsCalculatorBreakdownKind =
  | "runtime"
  | "command_output"
  | "repo_context"
  | "terse_output"
  | "change_scope"
  | "doc_preprocess";
export type SavingsCalculatorConfidence = "measured" | "estimated" | "inferred";

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
  scope: SavingsCalculatorScope;
  recordedAt: string;
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

  const sentTokens = dashboard.dailySavings.reduce(
    (sum, point) => sum + point.totalTokensSent,
    0,
  );
  const beforeTokens = sentTokens + dashboard.lifetimeEstimatedTokensSaved;
  const savingsPct =
    beforeTokens > 0
      ? (dashboard.lifetimeEstimatedTokensSaved / beforeTokens) * 100
      : null;

  return {
    scope,
    requests: dashboard.lifetimeRequests,
    savedTokens: dashboard.lifetimeEstimatedTokensSaved,
    savedUsd: dashboard.lifetimeEstimatedSavingsUsd,
    conservativeSavedUsd: dashboard.lifetimeEstimatedSavingsUsd * 0.5,
    sentTokens,
    beforeTokens,
    savingsPct,
    dataLabel: dashboard.savingsHistoryLoaded
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
  const rows: SavingsCalculatorBreakdownRow[] = [
    {
      id: "headroom",
      label: "Headroom",
      kind: "runtime",
      confidence: scope === "session" ? "measured" : "estimated",
      savedTokens: summary.savedTokens,
      savedUsd: summary.savedUsd,
      detail:
        scope === "session"
          ? "Runtime compression measured in this app session."
          : "Runtime compression recorded across saved history.",
    },
  ];

  const rtkSaved = Math.max(0, options.runtimeStatus?.rtk.totalSaved ?? 0);
  const rtkCommands = Math.max(
    0,
    options.runtimeStatus?.rtk.totalCommands ?? 0,
  );
  if (scope === "overall" && rtkSaved > 0) {
    rows.push({
      id: "rtk",
      label: "RTK",
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
  if (repoSaved > 0) {
    rows.push({
      id: "repo_intelligence",
      label: "Repo Intelligence",
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
      scope,
      recordedAt,
    }),
  );
}

export function formatSavingsCalculatorShareText(
  summary: SavingsCalculatorSummary,
  rows: SavingsCalculatorBreakdownRow[],
) {
  const scopeLabel =
    summary.scope === "session" ? "current app session" : "overall history";
  const sourceLines = rows.map((row) => {
    const usdPart =
      row.savedUsd === null ? "" : ` / ${formatUsd(row.savedUsd)}`;
    return `- ${row.label} (${row.confidence}): ${formatTokens(row.savedTokens)} tokens${usdPart}`;
  });

  return [
    `Mac AI Switchboard savings (${scopeLabel})`,
    `Saved: ${formatTokens(summary.savedTokens)} tokens / ${formatUsd(summary.savedUsd)}`,
    `Requests: ${formatTokens(summary.requests)}`,
    `Reduction: ${formatPercent(summary.savingsPct)}`,
    `Likely at least: ${formatUsd(summary.conservativeSavedUsd)}`,
    `Equation: before ${formatTokens(summary.beforeTokens)} - sent ${formatTokens(summary.sentTokens)} = saved ${formatTokens(summary.savedTokens)}`,
    "Sources:",
    ...(sourceLines.length > 0 ? sourceLines : ["- Waiting for usage"]),
  ].join("\n");
}
