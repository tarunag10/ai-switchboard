import { CheckCircle, ClipboardText, Copy, Package, WarningCircle } from "@phosphor-icons/react";
import { useMemo, useState } from "react";

import {
  AGENT_SESSION_PRESETS,
  buildAgentSessionPayload,
  getAgentSessionActionLabel,
  prepareStartAgentSessionPack,
} from "../lib/agentSessionPacks";
import { formatCompactNumber } from "../lib/optimization";

export function AgentSessionPanel() {
  const [agentId, setAgentId] = useState(AGENT_SESSION_PRESETS[0]?.id ?? "");
  const agent =
    AGENT_SESSION_PRESETS.find((preset) => preset.id === agentId) ??
    AGENT_SESSION_PRESETS[0];
  const [packId, setPackId] = useState(agent?.packs[0]?.id ?? "");
  const [budget, setBudget] = useState(agent?.defaultBudget ?? 16_000);
  const [task, setTask] = useState("Implement the next scoped optimization slice.");
  const [copied, setCopied] = useState(false);

  const activePackId =
    agent?.packs.some((pack) => pack.id === packId) ? packId : agent?.packs[0]?.id;
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
  const canCopy = preparation.inject && payload.length > 0;

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
          onClick={copyPayload}
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
        </p>
      ) : null}

      <pre className="optimize-minimal__meta">
        <code>{preparation.stablePrefixMarkdown || payload}</code>
      </pre>
    </section>
  );
}
