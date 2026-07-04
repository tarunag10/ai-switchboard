import { Terminal } from "@phosphor-icons/react";
import type { PlannedAddon } from "../lib/plannedAddons";
import type { PlannedConnector } from "../lib/plannedConnectors";
import { plannedConnectors } from "../lib/plannedConnectors";
import { PlannedConnectorRoadmap } from "./PlannedConnectorRoadmap";

export function PlannedAddonCard({
  addon,
  onCopyConnectorConfigPlan,
  onOpenRepoIntelligence,
}: {
  addon: PlannedAddon;
  onCopyConnectorConfigPlan?: (connector: PlannedConnector) => void;
  onOpenRepoIntelligence?: () => void;
}) {
  const showConnectorRoadmap = addon.id === "agent_connectors";
  const showRepoIntelligencePreview = addon.id === "repo_intelligence";
  const isRepoIntelligence = addon.id === "repo_intelligence";
  const cardClassName = isRepoIntelligence
    ? "addon-card addon-card--active"
    : "addon-card addon-card--planned";
  const badgeClassName = isRepoIntelligence
    ? "addon-card__badge addon-card__badge--ready"
    : "addon-card__badge addon-card__badge--planned";

  return (
    <li className={cardClassName}>
      <div className="addon-card__body">
        <div className="addon-card__heading">
          <span className="addon-card__name">{addon.name}</span>
          <span className={badgeClassName}>{addon.statusLabel}</span>
        </div>
        <p className="addon-card__description">{addon.description}</p>
        <ul className="addon-card__bullets">
          {addon.bullets.map((bullet) => (
            <li key={bullet}>{bullet}</li>
          ))}
        </ul>
        <div className="addon-card__evidence-grid">
          <section>
            <h4>Health checks</h4>
            <ul>
              {addon.healthChecks.map((check) => (
                <li key={check}>{check}</li>
              ))}
            </ul>
          </section>
          <section>
            <h4>Savings sources</h4>
            <ul>
              {addon.savingsSources.map((source) => (
                <li key={source}>{source}</li>
              ))}
            </ul>
          </section>
        </div>
        {addon.verificationCommand ? (
          <p className="addon-card__verification">
            <Terminal size={13} weight="duotone" />
            <code>{addon.verificationCommand}</code>
          </p>
        ) : null}
        {showConnectorRoadmap ? (
          <PlannedConnectorRoadmap
            connectors={plannedConnectors}
            onCopyConfigPlan={
              onCopyConnectorConfigPlan ??
              (() => undefined)
            }
          />
        ) : null}
        {showRepoIntelligencePreview ? (
          <div className="repo-intelligence-addon-cta">
            <strong>Dedicated workspace is available.</strong>
            <span>
              Open Repo Intelligence to index a repository and copy real local
              packs.
            </span>
            <button
              className="addon-card__action addon-card__action--primary"
              onClick={onOpenRepoIntelligence}
              type="button"
            >
              Open Repo Intelligence
            </button>
          </div>
        ) : null}
      </div>
      <div className="addon-card__actions">
        <button type="button" className="addon-card__action" disabled>
          {isRepoIntelligence ? "Open from sidebar" : "Review gated readiness"}
        </button>
      </div>
    </li>
  );
}
