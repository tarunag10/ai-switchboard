import { useState } from "react";
import { Calculator, Copy } from "@phosphor-icons/react";
import {
  buildAddonSavingsEstimate,
  buildFilteredSavingsLedger,
  buildSavingsAnomalyAlerts,
  buildSavingsCalculatorBreakdown,
  buildSavingsLedgerRows,
  buildSavingsCalculatorSummary,
  formatSavingsAnomalyAlerts,
  formatSavingsLedgerConfidenceBreakdown,
  formatSavingsLedgerAttributionSummary,
  formatSavingsLedgerShareText,
  getSavingsLedgerEmptyState,
  savingsCalculatorScopeDefinition,
  savingsCalculatorScopeLabel,
  type AddonSavingsEstimate,
  type SavingsLedgerConfidenceFilter,
  type SavingsCalculatorScope,
} from "../lib/savingsCalculator";
import {
  compactNumber,
  currencyExact,
  formatDateTime,
  percent1,
} from "../lib/dashboardHelpers";
import type {
  DashboardState,
  RuntimeStatus,
  ActivityFeedResponse,
  SavingsAttributionEvent,
} from "../lib/types";
import type { RepoSavingsEstimate } from "../lib/repoIntelligence";

const savingsLedgerConfidenceFilters: SavingsLedgerConfidenceFilter[] = [
  "all",
  "measured",
  "estimated",
  "inferred",
];
const savingsCalculatorScopes: SavingsCalculatorScope[] = [
  "session",
  "repo",
  "today",
  "week",
  "month",
  "lifetime",
];

export function SavingsCalculatorCard({
  dashboard,
  repoSavings,
  runtimeStatus,
  rtkToday,
  attributionEvents,
  cavemanSavings,
  ponytailSavings,
  markitdownSavings,
  scope,
  onScopeChange,
}: {
  dashboard: DashboardState;
  repoSavings?: RepoSavingsEstimate | null;
  runtimeStatus?: RuntimeStatus | null;
  rtkToday?: ActivityFeedResponse["tiles"]["rtkToday"];
  attributionEvents?: SavingsAttributionEvent[];
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
    rtkToday,
    attributionEvents,
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
      rtkToday,
      attributionEvents,
      cavemanSavings,
      ponytailSavings,
      markitdownSavings,
    },
  );
  const savedLabel = compactNumber(summary.savedTokens);
  const sentLabel = compactNumber(summary.sentTokens);
  const beforeLabel = compactNumber(summary.beforeTokens);
  const conservativeUsdLabel = currencyExact(summary.conservativeSavedUsd);
  const hasUsage =
    summary.requests > 0 || summary.savedTokens > 0 || ledgerRows.length > 0;
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
  const anomalyAlerts = buildSavingsAnomalyAlerts(
    attributionEvents ?? [],
    scope,
    ledgerRecordedAt,
  );
  const ledgerEmptyState = getSavingsLedgerEmptyState(
    ledgerRows.length,
    ledgerFilter,
  );
  const measuredSessionAttributionEvents = (attributionEvents ?? []).filter(
    (event) =>
      scope === "session" &&
      event.scope === "session" &&
      event.confidence === "measured" &&
      (event.deltaTokensSaved > 0 ||
        event.deltaUsd > 0 ||
        event.requestDelta > 0),
  ).length;

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
        ledgerFilter,
        anomalyAlerts,
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
          {savingsCalculatorScopes.map((item) => (
            <button
              key={item}
              type="button"
              className={`savings-calculator__scope-button${
                scope === item ? " is-active" : ""
              }`}
              onClick={() => onScopeChange(item)}
            >
              {savingsCalculatorScopeLabel(item).replace("current app ", "")}
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
          <span>Saved {savingsCalculatorScopeLabel(scope)}</span>
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
                {row.detail} Source: {row.source}. Confidence:{" "}
                {row.confidence}.
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
            <strong>{savingsCalculatorScopeLabel(scope)}</strong>
            <p>{savingsCalculatorScopeDefinition(scope)}</p>
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
          <span>
            Confidence{" "}
            <strong>
              {formatSavingsLedgerConfidenceBreakdown(filteredLedger.summary)}
            </strong>
          </span>
          <span>
            Attribution{" "}
            <strong>
              {formatSavingsLedgerAttributionSummary(filteredLedger.summary)}
            </strong>
          </span>
          {scope === "session" ? (
            <span>
              Backend events{" "}
              <strong>{compactNumber(measuredSessionAttributionEvents)}</strong>
            </span>
          ) : null}
        </div>
        {anomalyAlerts.length > 0 ? (
          <div
            className="savings-calculator__anomalies"
            aria-label="Savings anomaly alerts"
          >
            <strong>Savings anomaly alerts</strong>
            <span>{formatSavingsAnomalyAlerts(anomalyAlerts)}</span>
          </div>
        ) : null}
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
                <strong>{ledgerEmptyState.title}</strong>
                <span>{ledgerEmptyState.detail}</span>
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
