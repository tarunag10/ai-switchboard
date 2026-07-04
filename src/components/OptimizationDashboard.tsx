import { useEffect, useState } from "react";
import {
  ArrowClockwise,
  CheckCircle,
  Database,
  Lightning,
  Package,
  TerminalWindow,
  WarningCircle,
} from "@phosphor-icons/react";
import {
  type OptimizationActionPolicy,
  type ModelRoutingValidationReceipt,
  type PreemptiveCompactionReceipt,
  type OptimizationSnapshot,
  type PromptCacheClientProof,
  formatCompactNumber,
  getPromptCacheAction,
  loadOptimizationActionPolicy,
  loadOptimizationSnapshot,
  saveOptimizationActionPolicy,
  runPreemptiveCompaction,
  validateModelRouting,
} from "../lib/optimization";
import { AgentSessionPanel } from "./AgentSessionPanel";
import { RedundancyPanel } from "./RedundancyPanel";
import { RoutingDecisionList, TokenXrayPanel } from "./TokenXrayPanel";

function statusIcon(status: string) {
  if (status === "blocked") {
    return <WarningCircle weight="duotone" aria-hidden="true" />;
  }
  return <CheckCircle weight="duotone" aria-hidden="true" />;
}


function OptimizationActionPanel() {
  const [policy, setPolicy] = useState<OptimizationActionPolicy | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void loadOptimizationActionPolicy().then((nextPolicy) => {
      if (!cancelled) setPolicy(nextPolicy);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  async function toggle(key: keyof Pick<
    OptimizationActionPolicy,
    | "promptCacheReorderEnabled"
    | "preemptiveCompactionEnabled"
    | "modelRoutingEnabled"
  >) {
    if (!policy) return;
    const nextPolicy = { ...policy, [key]: !policy[key] };
    setPolicy(nextPolicy);
    setSaving(true);
    try {
      setPolicy(await saveOptimizationActionPolicy(nextPolicy));
    } finally {
      setSaving(false);
    }
  }

  async function enableAll() {
    if (!policy) return;
    const nextPolicy = {
      ...policy,
      promptCacheReorderEnabled: true,
      preemptiveCompactionEnabled: true,
      modelRoutingEnabled: true,
    };
    setPolicy(nextPolicy);
    setSaving(true);
    try {
      setPolicy(await saveOptimizationActionPolicy(nextPolicy));
    } finally {
      setSaving(false);
    }
  }

  if (!policy) return null;

  return (
    <section className="optimize-minimal" aria-labelledby="optimization-action-title">
      <div className="optimize-card__head">
        <div>
          <h2 id="optimization-action-title">Action Policy</h2>
          <p className="optimize-minimal__meta">
            Controls that allow Switchboard to move from observe-only to guarded actions.
          </p>
        </div>
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void enableAll()}
          disabled={saving}
        >
          Enable all
        </button>
      </div>
      <div className="optimize-projects">
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void toggle("promptCacheReorderEnabled")}
        >
          Prompt cache reorder: {policy.promptCacheReorderEnabled ? "on" : "off"}
        </button>
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void toggle("preemptiveCompactionEnabled")}
        >
          Preemptive compaction: {policy.preemptiveCompactionEnabled ? "on" : "off"}
        </button>
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void toggle("modelRoutingEnabled")}
        >
          Model routing: {policy.modelRoutingEnabled ? "on" : "off"}
        </button>
      </div>
    </section>
  );
}


function PreemptiveCompactionButton() {
  const [receipt, setReceipt] = useState<PreemptiveCompactionReceipt | null>(null);
  const [busy, setBusy] = useState(false);

  async function run() {
    setBusy(true);
    try {
      setReceipt(await runPreemptiveCompaction());
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="optimize-minimal" aria-labelledby="preemptive-compaction-title">
      <div className="optimize-card__title-row">
        <span className="optimize-card__title-icon" aria-hidden="true">
          <ArrowClockwise weight="duotone" />
        </span>
        <h2 id="preemptive-compaction-title">Preemptive Compaction</h2>
      </div>
      <p className="optimize-minimal__meta">
        One click records the live threshold check and queues Switchboard's prevention path before
        clients hit an oversized-context failure.
      </p>
      <button
        className="secondary-button secondary-button--small"
        type="button"
        onClick={() => run()}
        disabled={busy}
      >
        <ArrowClockwise weight="bold" size={12} aria-hidden="true" />
        {busy ? "Running" : "Run compaction"}
      </button>
      {receipt ? (
        <p className="optimize-minimal__meta" role="status">
          {receipt.action} {receipt.contextUsedPercent}% used; trigger at {receipt.thresholdPercent}%.
        </p>
      ) : null}
    </section>
  );
}

function PromptCacheClientProofList({
  clients,
}: {
  clients: PromptCacheClientProof[];
}) {
  return (
    <section className="optimize-minimal" aria-labelledby="cache-proof-title">
      <div className="optimize-card__title-row">
        <span className="optimize-card__title-icon" aria-hidden="true">
          <Database weight="duotone" />
        </span>
        <h2 id="cache-proof-title">Cache Proof</h2>
      </div>
      <p className="optimize-minimal__meta">
        Provider cache reads by client. This is the live proof for prompt-cache efficiency.
      </p>
      {clients.length === 0 ? (
        <p className="optimize-minimal__meta">No provider cache telemetry yet.</p>
      ) : (
        <div className="optimize-projects">
          {clients.map((client) => (
            <div key={`${client.client}-${client.provider}`} className="optimize-project-row">
              <div className="optimize-project-row__main">
                <span className="optimize-project-row__name">{client.client}</span>
                <span className="optimize-project-row__training">
                  {client.provider} {client.efficiencyPercent}% efficient
                </span>
                <span className="optimize-minimal__meta">{client.proof}</span>
              </div>
              <span className="optimize-project-row__training">
                {client.cacheReadTokens.toLocaleString()} cache hits
              </span>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}


function RoutingValidationPanel() {
  const [receipt, setReceipt] = useState<ModelRoutingValidationReceipt | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function runValidation() {
    setBusy(true);
    setError(null);
    try {
      setReceipt(await validateModelRouting());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="optimize-minimal" aria-labelledby="routing-validation-title">
      <div className="optimize-card__title-row">
        <span className="optimize-card__title-icon" aria-hidden="true">
          <TerminalWindow weight="duotone" />
        </span>
        <h2 id="routing-validation-title">Routing Validation</h2>
      </div>
      <p className="optimize-minimal__meta">
        One-click read-only proof that managed clients route trivial work to the cheaper model candidate.
      </p>
      <button
        className="secondary-button secondary-button--small"
        type="button"
        onClick={() => void runValidation()}
        disabled={busy}
      >
        <ArrowClockwise weight="bold" size={12} aria-hidden="true" />
        {busy ? "Validating" : "Validate routing"}
      </button>
      {error ? <p className="optimize-minimal__meta">{error}</p> : null}
      {receipt ? (
        <div className="optimize-projects">
          {receipt.checks.map((check) => (
            <div key={`${check.client}-${check.task}`} className="optimize-project-row">
              <div className="optimize-project-row__main">
                <span className="optimize-project-row__name">{check.client}</span>
                <span className="optimize-project-row__training">
                  {check.status}: {check.selectedModel}
                </span>
                <span className="optimize-minimal__meta">{check.reason}</span>
              </div>
              <span className="optimize-project-row__training">
                fallback {check.fallbackModel}
              </span>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
export function OptimizationDashboard() {
  const [snapshot, setSnapshot] = useState<OptimizationSnapshot | null>(null);
  const [loading, setLoading] = useState(true);

  async function refresh() {
    setLoading(true);
    const nextSnapshot = await loadOptimizationSnapshot();
    setSnapshot(nextSnapshot);
    setLoading(false);
  }

  useEffect(() => {
    void refresh();
  }, []);

  if (!snapshot) {
    return (
      <section className="optimize-minimal" aria-live="polite">
        <p className="loading-copy">
          {loading ? "Loading optimization telemetry..." : "No telemetry yet."}
        </p>
      </section>
    );
  }

  return (
    <section className="panel-stack panel-stack--tight" aria-labelledby="optimization-dashboard-title">
      <div className="optimize-minimal">
        <div className="optimize-card__head">
          <div className="optimize-card__title-row">
            <span className="optimize-card__title-icon" aria-hidden="true">
              <Lightning weight="duotone" />
            </span>
            <div>
              <h2 id="optimization-dashboard-title">AI Switchboard Optimization</h2>
              <p className="optimize-minimal__meta">
                {snapshot.source === "tauri" ? "Live Tauri telemetry" : "Local fallback telemetry"}.
              </p>
              {snapshot.bypass.any ? (
                <p className="optimize-minimal__meta" role="alert">
                  Compression fail-open is active for{" "}
                  {[
                    snapshot.bypass.anthropic ? "Claude/Anthropic" : null,
                    snapshot.bypass.openai ? "Codex/OpenAI" : null,
                  ]
                    .filter(Boolean)
                    .join(" and ")}
                  . Native compaction remains unblocked, but Switchboard savings are paused for that
                  client.
                </p>
              ) : null}
            </div>
          </div>
          <button
            className="secondary-button secondary-button--small"
            type="button"
            onClick={() => void refresh()}
            disabled={loading}
          >
            <ArrowClockwise weight="bold" size={12} aria-hidden="true" />
            {loading ? "Refreshing" : "Refresh"}
          </button>
        </div>
        <div className="install-progress__steps">
          <div className="install-progress__step">
            <Database weight="duotone" aria-hidden="true" />
            <span>
              Prompt cache {snapshot.promptCache.efficiencyPercent}% hit rate,
              {formatCompactNumber(snapshot.promptCache.estimatedTokensSaved)} saved
            </span>
          </div>
          <div className="install-progress__step">
            {statusIcon(snapshot.compaction.state)}
            <span>
              Compaction {snapshot.compaction.contextUsedPercent}% used, trigger at{" "}
              {snapshot.compaction.triggerAtPercent}%
            </span>
          </div>
          <div className="install-progress__step">
            {statusIcon(snapshot.agentPack.status)}
            <span>
              {snapshot.agentPack.packName}{" "}
              {snapshot.agentPack.enabled ? "injection ready" : "injection off"}
            </span>
          </div>
        </div>
        <p className="optimize-minimal__meta">
          {getPromptCacheAction(snapshot)} {snapshot.compaction.nextAction}
        </p>
      </div>

      <OptimizationActionPanel />
      <PreemptiveCompactionButton />
      <PromptCacheClientProofList clients={snapshot.promptCacheClients} />
      <TokenXrayPanel snapshot={snapshot.tokenXray} />
      <RedundancyPanel findings={snapshot.redundancy} />
      <AgentSessionPanel />
      <RoutingDecisionList decisions={snapshot.routing} />
      <RoutingValidationPanel />

      <section className="optimize-minimal" aria-labelledby="pack-rtk-title">
        <div className="optimize-card__title-row">
          <span className="optimize-card__title-icon" aria-hidden="true">
            <Package weight="duotone" />
          </span>
          <div>
            <h2 id="pack-rtk-title">Pack + RTK</h2>
            <p className="optimize-minimal__meta">{snapshot.agentPack.message}</p>
          </div>
        </div>
        <div className="optimize-projects">
          {snapshot.rtkPresets.map((preset) => (
            <div className="optimize-project-row" key={preset.id}>
              <div className="optimize-project-row__main">
                <span className="optimize-project-row__name">{preset.label}</span>
                <span className="optimize-project-row__training">{preset.purpose}</span>
                <code className="install-prompt__cmd-text">
                  <TerminalWindow weight="duotone" aria-hidden="true" /> {preset.command}
                </code>
              </div>
            </div>
          ))}
        </div>
      </section>
    </section>
  );
}
