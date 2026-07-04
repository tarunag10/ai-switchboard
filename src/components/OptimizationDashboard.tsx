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
  formatCompactNumber,
  getPromptCacheAction,
  loadOptimizationActionPolicy,
  loadOptimizationSnapshot,
  saveOptimizationActionPolicy,
  type OptimizationActionPolicy,
  type OptimizationSnapshot,
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
        <span className="optimize-minimal__meta">{saving ? "Saving" : "Ready"}</span>
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
              <h2 id="optimization-dashboard-title">Codex Optimization</h2>
              <p className="optimize-minimal__meta">
                {snapshot.source === "tauri" ? "Live Tauri telemetry" : "Local fallback telemetry"}.
              </p>
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
      <TokenXrayPanel snapshot={snapshot.tokenXray} />
      <RedundancyPanel findings={snapshot.redundancy} />
      <AgentSessionPanel />
      <RoutingDecisionList decisions={snapshot.routing} />

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
