import {
  aggregateClientConnectors,
  sortClientConnectors,
  connectorDashboardStatus,
} from "../lib/dashboardHelpers";
import type { ClientConnectorStatus } from "../lib/types";

export function AddonClientChips({
  connectors,
}: {
  connectors: ClientConnectorStatus[];
}) {
  const clients = sortClientConnectors(aggregateClientConnectors(connectors));
  if (clients.length === 0) {
    return null;
  }
  return (
    <div className="addon-card__clients">
      {clients.map((connector) => {
        const status = connectorDashboardStatus(connector);
        return (
          <span
            className="callout-banner__chip"
            key={connector.clientId}
            title={status.label}
          >
            <span
              className={`callout-banner__chip-dot callout-banner__chip-dot--${status.tone}`}
              aria-hidden="true"
            />
            <span className="callout-banner__chip-name">{connector.name}</span>
            <span className="visually-hidden">{status.label}</span>
          </span>
        );
      })}
    </div>
  );
}
