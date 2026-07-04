import {
  mergeProviderSavingsForDisplay,
  currencyExact,
  compactNumber,
  type SavingsChartDatum,
} from "../lib/dashboardHelpers";

export type SavingsChartMode = "usd" | "tokens";

export function SavingsChartTooltip({
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
