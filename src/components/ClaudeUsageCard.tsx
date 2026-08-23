import { ArrowClockwise, Clock, Info } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { loadClaudeUsage, type ClaudeUsage, type ClaudeUsageWindow } from "../lib/claudeUsage";

function percent(value: number): string {
  return `${Math.max(0, Math.min(100, value)).toFixed(value % 1 ? 1 : 0)}% used`;
}

function reset(value: string): string {
  const timestamp = Date.parse(value);
  return timestamp ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(timestamp) : "Reset time unavailable";
}

function UsageWindow({ label, window }: { label: string; window: ClaudeUsageWindow | null }) {
  return (
    <div className="claude-usage-card__window">
      <div><strong>{label}</strong><span>{window ? percent(window.utilization) : "Unavailable"}</span></div>
      <small>{window ? `Resets ${reset(window.resetsAt)}` : "No credible usage window returned."}</small>
    </div>
  );
}

export function ClaudeUsageCard({ hidden }: { hidden: boolean }) {
  const [usage, setUsage] = useState<ClaudeUsage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      setUsage(await loadClaudeUsage());
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Claude usage is unavailable.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!hidden) void refresh();
  }, [hidden]);

  return (
    <article className="repo-map-panel claude-usage-card" aria-labelledby="claude-usage-title">
      <div className="repo-map-panel__header">
        <Clock size={18} weight="duotone" aria-hidden="true" />
        <div>
          <h2 id="claude-usage-title">Claude usage windows</h2>
          <p className="optimize-minimal__meta">Read-only account limits from Claude’s usage endpoint; this does not change routing or settings.</p>
        </div>
        <button className="secondary-button secondary-button--small" type="button" onClick={() => void refresh()} disabled={loading}>
          <ArrowClockwise size={12} weight="bold" aria-hidden="true" />
          {loading ? "Refreshing" : "Refresh usage"}
        </button>
      </div>
      <p className="optimize-minimal__meta"><Info size={13} aria-hidden="true" /> Refreshing contacts Anthropic using the locally captured Claude OAuth session; no prompt or response content is sent.</p>
      {error ? <p className="install-progress__error" role="alert">{error}</p> : null}
      {!error && !loading && usage ? (
        <>
          <div className="claude-usage-card__windows">
            <UsageWindow label="Five-hour window" window={usage.fiveHour} />
            <UsageWindow label="Seven-day window" window={usage.sevenDay} />
          </div>
          <p className="optimize-minimal__meta">
            Extra usage: {usage.extraUsage ? usage.extraUsage.isEnabled ? "enabled" : "disabled" : "Unavailable"}
            {usage.extraUsage?.utilization !== null && usage.extraUsage?.utilization !== undefined ? ` · ${percent(usage.extraUsage.utilization)}` : ""}
            {usage.extraUsage?.usedCredits !== null && usage.extraUsage?.usedCredits !== undefined ? ` · ${usage.extraUsage.usedCredits} credits used` : ""}
            {usage.extraUsage?.monthlyLimit !== null && usage.extraUsage?.monthlyLimit !== undefined ? ` of ${usage.extraUsage.monthlyLimit} monthly` : ""}
          </p>
        </>
      ) : null}
      {!error && !loading && !usage ? <p className="loading-copy">No Claude usage response yet.</p> : null}
    </article>
  );
}
