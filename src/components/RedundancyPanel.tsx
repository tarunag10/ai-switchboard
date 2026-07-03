import { CopySimple, Scissors } from "@phosphor-icons/react";
import {
  formatCompactNumber,
  getRedundancyTokens,
  RedundancyFinding
} from "../lib/optimization";

interface RedundancyPanelProps {
  findings: RedundancyFinding[];
}

export function RedundancyPanel({ findings }: RedundancyPanelProps) {
  return (
    <section className="optimize-minimal" aria-labelledby="redundancy-title">
      <div className="optimize-card__title-row">
        <span className="optimize-card__title-icon" aria-hidden="true">
          <CopySimple weight="duotone" />
        </span>
        <div>
          <h2 id="redundancy-title">Redundancy</h2>
          <p className="optimize-minimal__meta">
            {formatCompactNumber(getRedundancyTokens(findings))} duplicate tokens
            detected.
          </p>
        </div>
      </div>
      <div className="optimize-projects">
        {findings.map((finding) => (
          <div className="optimize-project-row" key={finding.id}>
            <div className="optimize-project-row__main">
              <span className="optimize-project-row__name">{finding.label}</span>
              <span className="optimize-project-row__training">
                {formatCompactNumber(finding.duplicateTokens)} tokens in{" "}
                {finding.locations.join(", ")}
              </span>
              <span className="optimize-minimal__meta">
                <Scissors weight="duotone" aria-hidden="true" /> {finding.action}
              </span>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
