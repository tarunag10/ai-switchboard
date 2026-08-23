import { ArrowClockwise, ChartBar } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { loadSavingsAttributionCounters } from "../lib/savingsAttributionCounters";
import type { SavingsAttributionCounter } from "../lib/types";

const label = (source: SavingsAttributionCounter["source"]) => source === "headroom_engine"
  ? "Headroom"
  : source === "repo_intelligence"
    ? "Repo Intelligence"
    : source === "compact_chinese"
      ? "Compact Chinese"
      : source.toUpperCase();

const number = (value: number) => new Intl.NumberFormat("en-US", { notation: value >= 1000 ? "compact" : "standard", maximumFractionDigits: 1 }).format(value);

export function SavingsAttributionCountersPanel({ hidden }: { hidden: boolean }) {
  const [counters, setCounters] = useState<SavingsAttributionCounter[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      setCounters(await loadSavingsAttributionCounters());
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Savings counters are unavailable.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!hidden) void refresh();
  }, [hidden]);

  return (
    <article className="repo-map-panel savings-attribution-counters" aria-labelledby="savings-attribution-counters-title">
      <div className="repo-map-panel__header">
        <ChartBar size={18} weight="duotone" aria-hidden="true" />
        <div>
          <h2 id="savings-attribution-counters-title">Source counters</h2>
          <p className="optimize-minimal__meta">Compact local attribution totals; detailed evidence remains in the savings ledger above.</p>
        </div>
        <button className="secondary-button secondary-button--small" type="button" onClick={() => void refresh()} disabled={loading}>
          <ArrowClockwise size={12} weight="bold" aria-hidden="true" />
          {loading ? "Refreshing" : "Refresh"}
        </button>
      </div>
      {error ? <p className="install-progress__error" role="alert">{error}</p> : null}
      {!error && !loading && counters.length === 0 ? <p className="loading-copy">No source counters recorded yet.</p> : null}
      {counters.length > 0 ? (
        <ul className="repo-map-tool-list">
          {counters.map((counter) => (
            <li className="repo-map-tool-list__item" key={`${counter.scope}-${counter.source}`}>
              <span>{label(counter.source)}<small>{counter.eventCount} events · {counter.measuredEventCount} measured</small></span>
              <strong>{number(counter.deltaTokensSaved)} tokens saved<small>{number(counter.totalTokensSent)} sent</small></strong>
            </li>
          ))}
        </ul>
      ) : null}
    </article>
  );
}
