import { ArrowClockwise, Cpu, GitBranch, Package, Stack } from "@phosphor-icons/react";
import {
  formatCompactNumber,
  getTokenReductionPercent,
  TokenXraySnapshot
} from "../lib/optimization";

interface TokenXrayPanelProps {
  snapshot: TokenXraySnapshot;
}

export function TokenXrayPanel({ snapshot }: TokenXrayPanelProps) {
  const buckets = [
    ["System", snapshot.systemTokens],
    ["User", snapshot.userTokens],
    ["Tools", snapshot.toolTokens],
    ["Pack", snapshot.packTokens]
  ] as const;

  return (
    <section className="optimize-minimal" aria-labelledby="token-xray-title">
      <div className="optimize-card__title-row">
        <span className="optimize-card__title-icon" aria-hidden="true">
          <Cpu weight="duotone" />
        </span>
        <div>
          <h2 id="token-xray-title">Token X-ray</h2>
          <p className="optimize-minimal__meta">
            {getTokenReductionPercent(snapshot)}% reduction from original prompt.
          </p>
        </div>
      </div>
      <div className="install-progress__steps">
        <div className="install-progress__step">
          <Stack weight="duotone" aria-hidden="true" />
          <span>Original {formatCompactNumber(snapshot.originalTokens)}</span>
        </div>
        <div className="install-progress__step">
          <ArrowClockwise weight="duotone" aria-hidden="true" />
          <span>Optimized {formatCompactNumber(snapshot.optimizedTokens)}</span>
        </div>
      </div>
      <div className="optimize-projects">
        {buckets.map(([label, value]) => (
          <div className="optimize-project-row" key={label}>
            <div className="optimize-project-row__main">
              <span className="optimize-project-row__name">{label}</span>
              <span className="optimize-project-row__training">
                {formatCompactNumber(value)} tokens
              </span>
            </div>
          </div>
        ))}
      </div>
      <p className="optimize-minimal__meta">
        <Package weight="duotone" aria-hidden="true" /> Pack tokens stay visible
        so injection cost is not hidden inside system prompt totals.
      </p>
    </section>
  );
}

export function RoutingDecisionList({
  decisions
}: {
  decisions: Array<{
    task: string;
    selectedModel: string;
    fallbackModel: string;
    reason: string;
    estimatedSavingsPercent: number;
  }>;
}) {
  return (
    <section className="optimize-minimal" aria-labelledby="routing-title">
      <div className="optimize-card__title-row">
        <span className="optimize-card__title-icon" aria-hidden="true">
          <GitBranch weight="duotone" />
        </span>
        <div>
          <h2 id="routing-title">Model Routing</h2>
          <p className="optimize-minimal__meta">Decision trace per task type.</p>
        </div>
      </div>
      <div className="optimize-projects">
        {decisions.map((decision) => (
          <div className="optimize-project-row" key={decision.task}>
            <div className="optimize-project-row__main">
              <span className="optimize-project-row__name">{decision.task}</span>
              <span className="optimize-project-row__training">
                {decision.selectedModel} / fallback {decision.fallbackModel} / saves{" "}
                {decision.estimatedSavingsPercent}%
              </span>
              <span className="optimize-minimal__meta">{decision.reason}</span>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
