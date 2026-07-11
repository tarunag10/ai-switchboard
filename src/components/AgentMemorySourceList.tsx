import { ShieldWarning } from "@phosphor-icons/react";
import { formatMemoryTokens, type AgentMemorySource } from "../lib/agentMemory";

export function AgentMemorySourceList({ sources, onPreview }: { sources: AgentMemorySource[]; onPreview: (source: AgentMemorySource) => void }) {
  if (!sources.length) return <article className="repo-map-panel"><h2>No memory sources found</h2><p>Switchboard did not find readable Codex, Claude, repo, or generated-session memory for this scope.</p></article>;
  return <ul className="repo-map-tool-list" aria-label="Agent memory sources">{sources.map((source) => <li className={`repo-map-tool-list__item repo-map-tool-list__item--${source.secretScan.status === "blocked" ? "warning" : source.status === "missing" ? "not-run" : "ok"}`} key={source.id}>
    <span>{source.agent} · {source.scope} <small className="repo-intelligence-view__badge">{source.status}</small></span>
    <strong>{formatMemoryTokens(source.estimatedTokens)} <small>tokens</small></strong>
    <small title={source.sourcePath}>{source.sourcePath}</small>
    <small>{source.managedBySwitchboard ? "Switchboard-managed" : "User-managed"} · {formatMemoryTokens(source.duplicateTokens)} duplicate · {formatMemoryTokens(source.cacheableTokens)} cacheable · secret scan: {source.secretScan.status}</small>
    {source.secretScan.status === "blocked" ? <small><ShieldWarning size={13} /> {source.secretScan.reason ?? "A high-risk secret category was detected. Content is not shown."}</small> : null}
    {source.previewAvailable ? <button className="secondary-button secondary-button--small" onClick={() => onPreview(source)} type="button">Preview compaction</button> : null}
    {!source.managedBySwitchboard || source.status === "user-managed" || source.status === "blocked" || source.secretScan.status !== "safe" ? <small>Changes are unavailable: this source is user-managed or safety-blocked.</small> : null}
  </li>)}</ul>;
}
