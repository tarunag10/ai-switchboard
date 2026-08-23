import { ArrowClockwise, CaretDown, Cpu, Info, WarningCircle } from "@phosphor-icons/react";
import { useEffect, useId, useRef, useState } from "react";
import { formatMetric, loadTokenXrayLiveUpdate, loadTokenXraySnapshot, type EvidenceConfidence, type Metric, type TokenXraySnapshot } from "../lib/usageAnalytics";

const metricNames: Array<[string, string, boolean]> = [["inputTokens", "Input", false], ["outputTokens", "Output", false], ["cacheReadTokens", "Cache read", false], ["cacheWriteTokens", "Cache write", false], ["providerBilledInputTokens", "Provider billed input", false], ["providerBilledBaselineTokens", "Provider billed baseline", false], ["compressionToolResultTokens", "Tool-result compression", false], ["compressionHistoryTokens", "History compression", false], ["compressionUserMessageTokens", "User-message compression", false], ["savedTokens", "Saved", false], ["avoidedTokens", "Avoided", false], ["estimatedCostUsd", "Cost", true], ["estimatedSavingsUsd", "Savings", true]];
const confidenceLabel = (value: EvidenceConfidence) => value;
const when = (value: number) => value ? new Date(value).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "Time unavailable";
const providerMetricKeys = ["inputTokens", "outputTokens", "cacheReadTokens", "cacheWriteTokens", "providerBilledInputTokens", "providerBilledBaselineTokens", "savedTokens", "avoidedTokens"] as const;
const unavailableMetric: Metric = { value: null, confidence: "unavailable", source: "Local analytics", observedAt: null, caveat: "No credible evidence is recorded for this metric." };
const snapshotMetric = (snapshot: TokenXraySnapshot, key: string) => snapshot.metrics[key] ?? unavailableMetric;

export function TokenXrayView({ hidden }: { hidden: boolean }) {
  const [snapshot, setSnapshot] = useState<TokenXraySnapshot | null>(null);
  const [loading, setLoading] = useState(true); const [error, setError] = useState<string | null>(null); const [showProvenance, setShowProvenance] = useState(false);
  const revisionRef = useRef<number | null>(null);
  const provenanceId = useId();
  const refresh = async () => { setLoading(true); setError(null); try { const value = await loadTokenXraySnapshot(); setSnapshot(value); } catch (reason) { setError(reason instanceof Error ? reason.message : "Token X-Ray is unavailable."); } finally { setLoading(false); } };
  useEffect(() => {
    if (hidden) return;
    revisionRef.current = null;
    void refresh();
  }, [hidden]);
  useEffect(() => {
    if (hidden) return;
    let cancelled = false;
    const poll = async () => {
      try {
        const update = await loadTokenXrayLiveUpdate(revisionRef.current);
        if (cancelled || !update) return;
        revisionRef.current = update.revision;
        setSnapshot((current) => current ? {
          ...current,
          schemaVersion: update.schemaVersion,
          generatedAt: update.generatedAt,
          agent: update.agent,
          provider: update.provider,
          model: update.model,
          freshness: update.freshness,
          metrics: update.metrics,
          contextPressure: update.contextPressure,
          timeline: update.timeline,
        } : current);
      } catch (reason) {
        if (!cancelled && !snapshot) setError(reason instanceof Error ? reason.message : "Live Token X-Ray updates are unavailable.");
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 2500);
    return () => { cancelled = true; window.clearInterval(timer); };
  }, [hidden]);
  return <div className="tray-content" hidden={hidden}><section className="repo-intelligence-view" aria-labelledby="token-xray-view-title">
    <header className="repo-intelligence-view__header"><div><h1 id="token-xray-view-title">Live Token X-Ray</h1><p className="repo-intelligence-view__subtitle">A local, content-free view of this session’s context, savings, and pressure.</p></div><button className="secondary-button secondary-button--small" aria-label="Refresh Token X-Ray evidence" onClick={() => void refresh()} disabled={loading} type="button"><ArrowClockwise className={loading ? "is-spinning" : undefined} size={15} />{loading ? "Refreshing…" : "Refresh evidence"}</button></header>
    {error ? <article className="repo-map-error" role="alert"><WarningCircle size={16} /> {error} <button className="secondary-button secondary-button--small" onClick={() => void refresh()} type="button">Retry</button></article> : null}
    {loading ? <div className="savings-chart__skeleton" role="status"><p className="loading-copy">Reading local session telemetry…</p></div> : null}
    {!loading && !error && snapshot?.freshness === "unavailable" ? <article className="repo-map-panel"><h2>No live session telemetry yet</h2><p>Start an agent session through Switchboard. The X-Ray will show local evidence once it is available; it never reads prompt content.</p></article> : null}
    {!loading && snapshot && snapshot.freshness !== "unavailable" ? <>
      <article className={`repo-map-hero repo-map-hero--${snapshot.contextPressure.band === "critical" || snapshot.contextPressure.band === "high" ? "warning" : "healthy"}`}><div className="repo-map-hero__icon"><Cpu size={28} weight="duotone" /></div><div className="repo-map-hero__copy"><p className="repo-map-eyebrow">{snapshot.freshness} local evidence</p><h2>{snapshot.agent ?? "Agent unavailable"}{snapshot.model ? ` · ${snapshot.model}` : ""}</h2><p>{snapshot.provider ?? "Provider unavailable"} · session {snapshot.sessionId ?? "not attributed"}</p></div><div className="repo-map-hero__status"><span>{snapshot.generatedAt ? `Updated ${when(snapshot.generatedAt)}` : "Timestamp unavailable"}</span></div></article>
      <article className="repo-map-panel"><div className="repo-map-panel__header"><Cpu size={18} weight="duotone" /><h2>Context pressure</h2></div><div className="repo-map-metrics"><div><dt>Used</dt><dd>{snapshot.contextPressure.usedTokens === null ? "Unavailable" : formatMetric({ value: snapshot.contextPressure.usedTokens, confidence: "inferred", source: "", observedAt: null, caveat: null })}</dd></div><div><dt>Known limit</dt><dd>{snapshot.contextPressure.limitTokens === null ? "Unavailable" : formatMetric({ value: snapshot.contextPressure.limitTokens, confidence: "inferred", source: "", observedAt: null, caveat: null })}</dd></div><div><dt>Pressure</dt><dd>{snapshot.contextPressure.percent === null ? "Absolute evidence only" : `${snapshot.contextPressure.percent}%`}</dd></div><div><dt>Next turn</dt><dd>{snapshot.contextPressure.projectedPercent === null ? "Unavailable" : `${snapshot.contextPressure.projectedPercent}%`}</dd></div></div><p className="optimize-minimal__meta">{snapshot.contextPressure.limitSource}. {snapshot.contextPressure.caveat ?? ""}</p></article>
      <section className="stat-grid stat-grid--2col" aria-label="Token composition">{metricNames.map(([key, label, currency]) => { const item = snapshotMetric(snapshot, key); return <article className="soft-card stat-card" key={key}><span className="stat-card__label">{label}<small className="repo-intelligence-view__badge">{confidenceLabel(item.confidence)}</small></span><strong className="stat-value--blue">{formatMetric(item, currency)}</strong><small>{item.caveat ?? item.source}</small></article>; })}</section>
      <article className="repo-map-panel" aria-labelledby="provider-metrics-title"><div className="repo-map-panel__header"><Info size={18} weight="duotone" /><h2 id="provider-metrics-title">Provider-specific X-Ray metrics</h2></div><p>{snapshot.provider ? `Current session provider: ${snapshot.provider}${snapshot.model ? ` · ${snapshot.model}` : ""}.` : "Current session provider is unavailable."} The local X-Ray schema exposes session-level evidence only; it does not invent provider-specific measurements.</p><div className="repo-map-tool-list">{providerMetricKeys.map((key) => { const item = snapshotMetric(snapshot, key); const available = item.value !== null && item.confidence !== "unavailable"; return <div className="repo-map-tool-list__item" key={key}><span>{key.replace(/([A-Z])/g, " $1").replace(/^./, (value) => value.toUpperCase())}</span><strong>{available ? formatMetric(item) : "Unavailable"}</strong><small>{available ? `Source: ${item.source}. ${item.caveat ?? ""}` : `Source: ${item.source}. ${item.caveat ?? "No credible provider-specific evidence is recorded."}`}</small></div>; })}</div><p className="optimize-minimal__meta">Provider-specific breakdown: unavailable unless explicitly supplied by the runtime evidence source. Session-level values must not be presented as provider comparisons.</p></article>
      <article className="repo-map-panel"><div className="repo-map-panel__header"><Info size={18} weight="duotone" /><h2>Optimization impact</h2></div>{snapshot.sources.length ? <ul className="repo-map-tool-list">{snapshot.sources.map((source) => <li className="repo-map-tool-list__item repo-map-tool-list__item--ok" key={source.id}><span>{source.label}</span><strong>{formatMetric(source.tokensSaved)} <small>{source.tokensSaved.confidence}</small></strong><small>{source.detail}{source.caveat ? ` · ${source.caveat}` : ""}</small></li>)}</ul> : <p>No source-level impact has been attributed to this session.</p>}</article>
      {snapshot.anomalies.length ? <article className="repo-map-panel"><div className="repo-map-panel__header"><WarningCircle size={18} weight="duotone" /><h2>Attention</h2></div><ul className="repo-map-tool-list">{snapshot.anomalies.map((item) => <li className={`repo-map-tool-list__item repo-map-tool-list__item--${item.severity === "critical" ? "warning" : "not-run"}`} key={item.id}><span>{item.title}</span><small>{item.detail}</small></li>)}</ul></article> : null}
      <article className="repo-map-panel"><div className="repo-map-panel__header"><CaretDown size={18} weight="duotone" /><h2>Recent timeline</h2></div>{snapshot.timeline.length ? <ul className="repo-map-history">{snapshot.timeline.slice(0, 12).map((event) => <li key={event.id}><span>{event.title}<small>{event.detail}</small></span><small>{when(event.occurredAt)} · {event.confidence}</small></li>)}</ul> : <p>No material usage, compaction, fallback, or anomaly events have been recorded.</p>}</article>
      <div className="repo-map-disclosure"><button className="repo-map-disclosure__button" aria-controls={provenanceId} aria-expanded={showProvenance} onClick={() => setShowProvenance((value) => !value)} type="button">{showProvenance ? "Hide" : "Show"} provenance and metric caveats</button>{showProvenance ? <div className="repo-map-disclosure__panel" id={provenanceId}><p>Measured values come from local runtime evidence. Estimated values require a recorded model or pricing source. Inferred values are derived from local event structure. Unavailable means Switchboard does not have credible evidence. Provider-specific values are shown only when the runtime supplies an explicit provider attribution and source.</p></div> : null}</div>
    </> : null}
  </section></div>;
}
