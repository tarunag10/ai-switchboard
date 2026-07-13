import { ArrowClockwise, Copy, Export, WarningCircle } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import {
  clearUsageAnalytics,
  exportDailyUsageBriefing,
  formatMetric,
  loadDailyUsageBriefing,
  loadDailyUsageBriefingHistory,
  previewClearUsageAnalytics,
  type DailyUsageBriefing,
  type UsageAnalyticsClearPreview,
} from "../lib/usageAnalytics";

const totals: Array<[string, string, boolean]> = [["requests", "Requests", false], ["spentTokens", "Input tokens", false], ["savedTokens", "Tokens saved", false], ["cachedTokens", "Cached", false], ["avoidedTokens", "Avoided", false], ["estimatedCostUsd", "Estimated cost", true], ["estimatedSavingsUsd", "Estimated savings", true]];

export function DailyUsageBriefingView({ hidden, onNavigate }: { hidden: boolean; onNavigate: (view: string) => void }) {
  const [briefing, setBriefing] = useState<DailyUsageBriefing | null>(null);
  const [history, setHistory] = useState<DailyUsageBriefing[]>([]);
  const [loading, setLoading] = useState(true);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [clearBusy, setClearBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [historyUnavailable, setHistoryUnavailable] = useState<string | null>(null);
  const [clearError, setClearError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [clearPreview, setClearPreview] = useState<UsageAnalyticsClearPreview | null>(null);

  const refresh = async () => { setLoading(true); setError(null); try { setBriefing(await loadDailyUsageBriefing()); } catch (reason) { setError(reason instanceof Error ? reason.message : "Daily briefing is unavailable."); } finally { setLoading(false); } };
  const loadHistory = async () => { setHistoryLoading(true); setHistoryUnavailable(null); try { setHistory(await loadDailyUsageBriefingHistory()); } catch { setHistoryUnavailable("Saved briefing history is not available in this runtime yet."); } finally { setHistoryLoading(false); } };
  const copy = async () => { try { if (!navigator.clipboard) throw new Error("Clipboard unavailable."); await navigator.clipboard.writeText(await exportDailyUsageBriefing("markdown")); setNotice("Markdown briefing copied."); } catch (reason) { setNotice(reason instanceof Error ? reason.message : "Could not copy briefing."); } };
  const exportJson = async () => { try { if (!navigator.clipboard) throw new Error("Clipboard unavailable."); await navigator.clipboard.writeText(await exportDailyUsageBriefing("json")); setNotice("Secret-free JSON export copied."); } catch (reason) { setNotice(reason instanceof Error ? reason.message : "Could not export briefing."); } };
  const previewClear = async () => { setClearBusy(true); setClearError(null); try { setClearPreview(await previewClearUsageAnalytics()); } catch { setClearError("Analytics retention controls are not available in this runtime yet."); } finally { setClearBusy(false); } };
  const confirmClear = async () => { if (!clearPreview) return; setClearBusy(true); setClearError(null); try { const result = await clearUsageAnalytics(); setClearPreview(null); setHistory([]); setNotice(result.detail); await refresh(); } catch { setClearError("Local analytics could not be cleared. Your data was left unchanged."); } finally { setClearBusy(false); } };
  const openDestination = (destination: string | null | undefined) => onNavigate(destination?.replace(/^\//, "") || "usage");
  useEffect(() => { if (!hidden) void refresh(); }, [hidden]);

  return <div className="tray-content" hidden={hidden}><section className="repo-intelligence-view" aria-labelledby="daily-briefing-title">
    <header className="repo-intelligence-view__header"><div><h1 id="daily-briefing-title">Daily AI Usage Briefing</h1><p className="repo-intelligence-view__subtitle">A local summary of today’s agent use, evidence coverage, and next actions.</p></div><div className="repo-map-actions"><button className="secondary-button secondary-button--small" onClick={() => void refresh()} disabled={loading} type="button"><ArrowClockwise className={loading ? "is-spinning" : undefined} size={15} />Refresh</button><button className="secondary-button secondary-button--small" onClick={() => void copy()} type="button"><Copy size={15} />Copy</button><button className="secondary-button secondary-button--small" onClick={() => void exportJson()} type="button"><Export size={15} />JSON</button></div></header>
    {notice ? <p className="optimize-minimal__meta" role="status">{notice}</p> : null}
    {error ? <p className="repo-map-error" role="alert"><WarningCircle size={16} /> {error}</p> : null}
    {loading ? <div className="savings-chart__skeleton" role="status"><p className="loading-copy">Building today’s local rollup…</p></div> : null}
    {!loading && !error && briefing?.completeness === "insufficient-data" ? <article className="repo-map-panel"><h2>Not enough local evidence yet</h2><p>Today’s briefing will appear as Switchboard records content-free agent usage. No prompt or response text is stored here.</p></article> : null}
    {!loading && briefing && briefing.completeness !== "insufficient-data" ? <>
      <article className="repo-map-hero repo-map-hero--healthy"><div className="repo-map-hero__copy"><p className="repo-map-eyebrow">{briefing.completeness} evidence · {briefing.timezone}</p><h2>{briefing.headline ?? `Local usage for ${briefing.dayKey}`}</h2><p>Generated {briefing.generatedAt ? new Date(briefing.generatedAt).toLocaleTimeString() : "without a timestamp"}.</p></div></article>
      <section className="stat-grid stat-grid--2col">{totals.map(([key, label, currency]) => { const item = briefing.totals[key]; return <article className="soft-card stat-card" key={key}><span className="stat-card__label">{label}<small className="repo-intelligence-view__badge">{item.confidence}</small></span><strong className="stat-value--blue">{formatMetric(item, currency)}</strong><small>{item.caveat ?? item.source}</small></article>; })}</section>
      <article className="repo-map-panel"><div className="repo-map-panel__header"><h2>Agents</h2></div>{briefing.agents.length ? <ul className="repo-map-tool-list">{briefing.agents.map((agent) => <li className="repo-map-tool-list__item repo-map-tool-list__item--ok" key={agent.id}><span>{agent.label}</span><strong>{agent.requests} requests · {formatMetric(agent.spentTokens)}</strong><small>{formatMetric(agent.savedTokens)} saved ({agent.savedTokens.confidence}){agent.highestContextPercent !== null ? ` · peak context ${agent.highestContextPercent}%` : ""}{agent.detail ? ` · ${agent.detail}` : ""}</small></li>)}</ul> : <p>No agent-level attribution is available.</p>}</article>
      {briefing.attentionItems.length ? <article className="repo-map-panel"><div className="repo-map-panel__header"><WarningCircle size={18} weight="duotone" /><h2>Needs attention</h2></div><ul className="repo-map-tool-list">{briefing.attentionItems.map((item) => <li className={`repo-map-tool-list__item repo-map-tool-list__item--${item.severity === "info" ? "not-run" : "warning"}`} key={item.id}><span>{item.title}</span><small>{item.detail}</small>{item.destination ? <button className="secondary-button secondary-button--small" onClick={() => openDestination(item.destination)} type="button">Open</button> : null}</li>)}</ul></article> : null}
      <article className="repo-map-panel"><div className="repo-map-panel__header"><h2>Recommended next actions</h2></div>{briefing.recommendations.length ? <ol className="repo-map-hotspots">{briefing.recommendations.map((item) => <li key={item.id}><span><strong>{item.title}</strong><small>{item.evidence}</small></span>{item.destination ? <button className="secondary-button secondary-button--small" onClick={() => openDestination(item.destination)} type="button">Open</button> : null}</li>)}</ol> : <p>No prioritized actions from today’s local evidence.</p>}</article>
      <article className="repo-map-panel"><div className="repo-map-panel__header"><h2>Saved briefings</h2><button className="secondary-button secondary-button--small" onClick={() => void loadHistory()} disabled={historyLoading} type="button">{historyLoading ? "Loading…" : "Load history"}</button></div>{historyUnavailable ? <p className="optimize-minimal__meta">{historyUnavailable}</p> : null}{history.length ? <ul className="repo-map-history">{history.slice(0, 14).map((item) => <li key={`${item.dayKey}-${item.generatedAt}`}><span>{item.dayKey}<small>{item.headline ?? `${item.completeness} local evidence`}</small></span><small>{formatMetric(item.totals.spentTokens)} input · {formatMetric(item.totals.savedTokens)} saved</small></li>)}</ul> : !historyUnavailable && !historyLoading ? <p>Load saved, local daily rollups. History contains content-free analytics only.</p> : null}</article>
      <article className="repo-map-panel"><div className="repo-map-panel__header"><h2>Analytics retention</h2></div><p>Usage analytics stay on this device. Deletion requires a preview and does not remove the savings ledger unless the preview explicitly says so.</p>{clearError ? <p className="repo-map-error" role="alert">{clearError}</p> : null}{clearPreview ? <div className="repo-map-disclosure__panel"><strong>Ready to delete {clearPreview.briefingCount} saved briefings and {clearPreview.eventCount} detailed analytics events.</strong><p>{clearPreview.detail}</p><div className="repo-map-actions"><button className="secondary-button secondary-button--small" onClick={() => setClearPreview(null)} disabled={clearBusy} type="button">Cancel</button><button className="secondary-button secondary-button--small" onClick={() => void confirmClear()} disabled={clearBusy} type="button">{clearBusy ? "Deleting…" : "Delete local analytics"}</button></div></div> : <button className="secondary-button secondary-button--small" onClick={() => void previewClear()} disabled={clearBusy} type="button">{clearBusy ? "Preparing preview…" : "Preview local analytics deletion"}</button>}</article>
      <p className="optimize-minimal__meta">Coverage: {briefing.evidenceCoverage.measured} measured, {briefing.evidenceCoverage.estimated} estimated, {briefing.evidenceCoverage.inferred} inferred, {briefing.evidenceCoverage.unavailable} unavailable. Exports are secret-free and content-free.</p>
    </> : null}
  </section></div>;
}
