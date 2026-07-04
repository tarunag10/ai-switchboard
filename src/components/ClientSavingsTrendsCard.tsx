import {
  buildClientSavingsTrendRows,
  compactNumber,
  currencyExact,
  percent1,
  formatDateTime,
} from "../lib/dashboardHelpers";
import type { DashboardState } from "../lib/types";

export function ClientSavingsTrendsCard({
  dashboard,
}: {
  dashboard: DashboardState;
}) {
  const trends = buildClientSavingsTrendRows(
    dashboard.recentUsage,
    dashboard.hourlySavings,
  );
  const trendScope =
    trends[0]?.scope === "saved_history" ? "saved history" : "current session";

  return (
    <article className="soft-card client-savings-trends">
      <header className="client-savings-trends__header">
        <div>
          <h2>Per-client savings</h2>
          <p>
            {trendScope === "saved history"
              ? "Saved local history, grouped by connected coding tool."
              : "Current app session, grouped by connected coding tool."}
          </p>
        </div>
        <span>
          {compactNumber(trends.length)} client{trends.length === 1 ? "" : "s"}
        </span>
      </header>
      {trends.length > 0 ? (
        <div className="client-savings-trends__list">
          {trends.map((trend) => {
            const reduction =
              trend.totalTokensSent + trend.estimatedTokensSaved > 0
                ? (trend.estimatedTokensSaved /
                    (trend.totalTokensSent + trend.estimatedTokensSaved)) *
                  100
                : 0;
            return (
              <div className="client-savings-trends__row" key={trend.client}>
                <div>
                  <strong>{trend.client}</strong>
                  <span>
                    {trend.scope === "saved_history"
                      ? `Saved history · latest ${formatDateTime(trend.lastSeenAt)}`
                      : `${compactNumber(trend.requests)} request${
                          trend.requests === 1 ? "" : "s"
                        } · last ${formatDateTime(trend.lastSeenAt)}`}
                  </span>
                </div>
                <dl>
                  <div>
                    <dt>Saved</dt>
                    <dd>{compactNumber(trend.estimatedTokensSaved)}</dd>
                  </div>
                  <div>
                    <dt>Spent</dt>
                    <dd>{compactNumber(trend.totalTokensSent)}</dd>
                  </div>
                  <div>
                    <dt>USD</dt>
                    <dd>{currencyExact(trend.estimatedSavingsUsd)}</dd>
                  </div>
                  <div>
                    <dt>Reduction</dt>
                    <dd>{percent1(reduction)}%</dd>
                  </div>
                </dl>
              </div>
            );
          })}
        </div>
      ) : (
        <p className="client-savings-trends__empty">
          Send a prompt through Claude Code, Codex, or another connected tool to
          populate session-level client trends. Saved per-client history appears
          here after provider-attributed savings history is available.
        </p>
      )}
    </article>
  );
}
