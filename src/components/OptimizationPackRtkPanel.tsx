import { Package, TerminalWindow } from "@phosphor-icons/react";
import type { OptimizationSnapshot } from "../lib/optimization";

export function OptimizationPackRtkPanel({
  snapshot,
}: {
  snapshot: OptimizationSnapshot;
}) {
  return (
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
  );
}
