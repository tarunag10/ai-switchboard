import { ArrowClockwise, Database, Lightning } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import {
  type OptimizationSnapshot,
  formatCompactNumber,
  getPromptCacheAction,
  loadOptimizationSnapshot,
} from "../lib/optimization";
import { AgentSessionPanel } from "./AgentSessionPanel";
import { RepoMemoryMcpSupervisionCard } from "./RepoMemoryMcpSupervisionCard";
import { RedundancyPanel } from "./RedundancyPanel";
import { RoutingDecisionList, TokenXrayPanel } from "./TokenXrayPanel";
import { OptimizationCompressionOverview } from "./OptimizationCompressionOverview";
import {
  OptimizationActionPanel,
  PreemptiveCompactionButton,
} from "./OptimizationActionControls";
import {
  OptimizationStatusIcon,
  PromptCacheClientProofList,
  RoutingValidationPanel,
} from "./OptimizationValidationPanels";
import { OptimizationPackRtkPanel } from "./OptimizationPackRtkPanel";
import { TransportObservationsPanel } from "./TransportObservationsPanel";

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
                {snapshot.source === "tauri" ? "Live Tauri telemetry" : "No live Tauri telemetry yet"}.
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
            <OptimizationStatusIcon status={snapshot.compaction.state} />
            <span>
              Compaction {snapshot.compaction.contextUsedPercent}% used, trigger at{" "}
              {snapshot.compaction.triggerAtPercent}%
            </span>
          </div>
          <div className="install-progress__step">
            <OptimizationStatusIcon status={snapshot.agentPack.status} />
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

      <OptimizationCompressionOverview />
      <OptimizationActionPanel />
      <PreemptiveCompactionButton />
      <PromptCacheClientProofList clients={snapshot.promptCacheClients} />
      <TokenXrayPanel snapshot={snapshot.tokenXray} />
      <RedundancyPanel findings={snapshot.redundancy} />
      <AgentSessionPanel />
      <RepoMemoryMcpSupervisionCard />
      <RoutingDecisionList decisions={snapshot.routing} />
      <RoutingValidationPanel />
      <OptimizationPackRtkPanel snapshot={snapshot} />
      <TransportObservationsPanel />
    </section>
  );
}
