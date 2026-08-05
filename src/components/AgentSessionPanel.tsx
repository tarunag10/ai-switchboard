import { CheckCircle, ClipboardText, Copy, Package, WarningCircle } from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";

import {
  agentSessionChecklistStatusLabel,
  buildAgentSessionCompressionChecklist,
} from "../lib/agentSessionCompressionChecklist";
import {
  AGENT_SESSION_PRESETS,
  buildAgentSessionPayload,
  getAgentSessionActionLabel,
  prepareStartAgentSessionPack,
  recommendAgentSessionPackId,
  resolveAgentSessionPreferredPackId,
} from "../lib/agentSessionPacks";
import { getRepoIndexFreshness, type RepoIntelligenceSummary } from "../lib/repoIntelligence";
import { formatCompactNumber } from "../lib/optimization";
import type { RuntimeStatus, SwitchboardMode, SwitchboardState } from "../lib/types";

type SemanticCacheStatus = {
  enabled: boolean;
};

export function AgentSessionPanel() {
  const [agentId, setAgentId] = useState(AGENT_SESSION_PRESETS[0]?.id ?? "");
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus | null>(null);
  const [switchboardMode, setSwitchboardMode] = useState<SwitchboardMode>("full");
  const [semanticCacheEnabled, setSemanticCacheEnabled] = useState(false);
  const [indexFreshness, setIndexFreshness] = useState(
    getRepoIndexFreshness({}),
  );
  const [acknowledged, setAcknowledged] = useState(false);
  const agent =
    AGENT_SESSION_PRESETS.find((preset) => preset.id === agentId) ??
    AGENT_SESSION_PRESETS[0];
  const [packId, setPackId] = useState(agent?.packs[0]?.id ?? "");
  const [budget, setBudget] = useState(agent?.defaultBudget ?? 16_000);
  const [task, setTask] = useState("Implement the next scoped optimization slice.");
  const [copied, setCopied] = useState(false);

  const activePackId = resolveAgentSessionPreferredPackId({
    task,
    tokenBudget: budget,
    preferredPackId: packId,
    candidates: agent?.packs ?? [],
  });
  const request = {
    agentId: agent?.id ?? "codex",
    task,
    tokenBudget: budget,
    enabled: true,
    preferredPackId: activePackId,
    candidates: agent?.packs ?? [],
  };
  const preparation = prepareStartAgentSessionPack(request);
  const payload = useMemo(
    () => buildAgentSessionPayload(request),
    [agent?.id, activePackId, budget, task],
  );
  const selectedPack = agent?.packs.find((pack) => pack.id === activePackId);
  const recommendation = useMemo(
    () =>
      recommendAgentSessionPackId({
        task,
        tokenBudget: budget,
        candidates: agent?.packs ?? [],
      }),
    [agent?.packs, budget, task],
  );
  const checklist = useMemo(
    () =>
      buildAgentSessionCompressionChecklist({
        agentId: agent?.id ?? "codex",
        packEstimatedTokens: selectedPack?.estimatedTokens ?? 0,
        tokenBudget: budget,
        switchboardMode,
        indexFreshness,
        runtimeStatus,
        semanticCacheEnabled,
      }),
    [
      agent?.id,
      budget,
      indexFreshness,
      runtimeStatus,
      selectedPack?.estimatedTokens,
      semanticCacheEnabled,
      switchboardMode,
    ],
  );
  const canCopy =
    preparation.inject &&
    payload.length > 0 &&
    (!checklist.blocked || acknowledged) &&
    (!checklist.canCopyWithAcknowledgment || acknowledged);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      invoke<RuntimeStatus>("get_runtime_status").catch(() => null),
      invoke<SwitchboardState>("get_switchboard_state")
        .then((state) => state.mode)
        .catch(() => "full" as SwitchboardMode),
      invoke<SemanticCacheStatus>("get_semantic_cache_status")
        .then((status) => status.enabled)
        .catch(() => false),
      invoke<RepoIntelligenceSummary | null>(
        "get_latest_repo_intelligence_summary",
      ).catch(() => null),
    ]).then(([runtime, mode, cacheEnabled, summary]) => {
      if (cancelled) return;
      setRuntimeStatus(runtime);
      setSwitchboardMode(mode);
      setSemanticCacheEnabled(cacheEnabled);
      setIndexFreshness(getRepoIndexFreshness(summary ?? {}));
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    setAcknowledged(false);
  }, [agentId, activePackId, budget, checklist.blocked, checklist.hasWarnings]);

  useEffect(() => {
    if (!agent || !recommendation.packId) return;
    const current = agent.packs.find((pack) => pack.id === packId);
    if (!current || current.estimatedTokens > budget) {
      setPackId(recommendation.packId);
    }
  }, [agent, budget, packId, recommendation.packId]);

  async function copyPayload() {
    if (!canCopy) return;
    await navigator.clipboard.writeText(payload);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1400);
  }

  function selectAgent(nextAgentId: string) {
    const nextAgent = AGENT_SESSION_PRESETS.find((preset) => preset.id === nextAgentId);
    setAgentId(nextAgentId);
    setPackId(nextAgent?.packs[0]?.id ?? "");
    setBudget(nextAgent?.defaultBudget ?? 16_000);
    setCopied(false);
  }

  return (
    <section className="optimize-card" aria-labelledby="agent-session-title">
      <div className="optimize-card__head">
        <div className="optimize-card__title-row">
          <span className="optimize-card__title-icon">
            <ClipboardText weight="duotone" />
          </span>
          <div>
            <h2 id="agent-session-title">Start Agent Session</h2>
            <p className="optimize-minimal__meta">
              Prepare a stable-prefix payload before launching the next agent.
            </p>
          </div>
        </div>
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void copyPayload()}
          disabled={!canCopy}
        >
          <Copy weight="bold" size={12} aria-hidden="true" />
          {copied ? "Copied" : "Copy payload"}
        </button>
      </div>

      <div className="optimize-projects">
        <label className="optimize-project-row">
          <span className="optimize-project-row__main">
            <span className="optimize-project-row__name">Agent</span>
            <select value={agentId} onChange={(event) => selectAgent(event.target.value)}>
              {AGENT_SESSION_PRESETS.map((preset) => (
                <option key={preset.id} value={preset.id}>
                  {preset.label}
                </option>
              ))}
            </select>
          </span>
        </label>

        <label className="optimize-project-row">
          <span className="optimize-project-row__main">
            <span className="optimize-project-row__name">Pack</span>
            <select value={activePackId} onChange={(event) => setPackId(event.target.value)}>
              {agent?.packs.map((pack) => (
                <option key={pack.id} value={pack.id}>
                  {pack.name}
                  {recommendation.packId === pack.id ? " (recommended)" : ""}
                </option>
              ))}
            </select>
          </span>
        </label>

        <label className="optimize-project-row">
          <span className="optimize-project-row__main">
            <span className="optimize-project-row__name">Budget</span>
            <input
              min={0}
              step={500}
              type="number"
              value={budget}
              onChange={(event) => setBudget(Number(event.target.value))}
            />
          </span>
        </label>

        <label className="optimize-project-row">
          <span className="optimize-project-row__main">
            <span className="optimize-project-row__name">Task</span>
            <input value={task} onChange={(event) => setTask(event.target.value)} />
          </span>
        </label>

        <label className="optimize-project-row">
          <span className="optimize-project-row__main">
            <span className="optimize-project-row__name">Inject stable prefix</span>
            <input
              type="checkbox"
              checked
            readOnly
            />
          </span>
        </label>
      </div>

      <section
        className="agent-session-checklist"
        aria-label="Compression session checklist"
      >
        <h3 className="agent-session-checklist__title">Compression checklist</h3>
        <ul className="agent-session-checklist__list">
          {checklist.items.map((item) => (
            <li
              key={item.id}
              className={`agent-session-checklist__item agent-session-checklist__item--${item.status}`}
            >
              <span>{item.label}</span>
              <strong>{agentSessionChecklistStatusLabel(item.status)}</strong>
              <p>{item.detail}</p>
              {item.doctorLink ? (
                <p className="optimize-minimal__meta">
                  Open Doctor to repair this before relying on the handoff.
                </p>
              ) : null}
            </li>
          ))}
        </ul>
        {checklist.canCopyWithAcknowledgment ? (
          <label className="agent-session-checklist__ack">
            <input
              type="checkbox"
              checked={acknowledged}
              onChange={(event) => setAcknowledged(event.target.checked)}
            />
            <span>
              I reviewed the checklist warnings and still want to copy this
              payload.
            </span>
          </label>
        ) : null}
      </section>

      <div className="install-progress" aria-live="polite">
        <div className="install-progress__step">
          {preparation.inject ? (
            <CheckCircle weight="duotone" aria-hidden="true" />
          ) : (
            <WarningCircle weight="duotone" aria-hidden="true" />
          )}
          <span>{getAgentSessionActionLabel(preparation)}</span>
        </div>
        <div className="install-progress__step">
          <Package weight="duotone" aria-hidden="true" />
          <span>
            {formatCompactNumber(preparation.remainingBudget)} remaining,{" "}
            {formatCompactNumber(preparation.cacheableTokens)} cacheable
          </span>
        </div>
      </div>

      {selectedPack ? (
        <p className="optimize-minimal__meta">
          {selectedPack.summary} Estimated {formatCompactNumber(selectedPack.estimatedTokens)} tokens.
          {recommendation.packId === selectedPack.id ? (
            <>
              {" "}
              <strong>Recommended pack.</strong> {recommendation.reason}
            </>
          ) : (
            <> Recommended: {recommendation.reason}</>
          )}
        </p>
      ) : null}

      <p
        className={`optimize-minimal__meta${copied ? " optimize-minimal__meta--notice" : ""}`}
        role="note"
      >
        {copied
          ? "Payload copied. Paste it into your agent before the task prompt."
          : "Copy the payload, then paste it into Codex, Claude Code, or Cursor before launching the task."}
      </p>

      <pre className="optimize-minimal__meta">
        <code>{preparation.stablePrefixMarkdown || payload}</code>
      </pre>
    </section>
  );
}
