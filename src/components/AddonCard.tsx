import type { ReactNode } from "react";
import type { ClientConnectorStatus } from "../lib/types";
import { AddonClientChips } from "./AddonClientChips";

export interface AddonCopy {
  whatItDoes: string;
  installing?: string;
  uninstalling?: string;
  installed?: string;
  uninstalled?: string;
  enabling?: string;
  disabling?: string;
  disabled?: string;
}

function formatAddonVersion(version: string): string {
  return /^\d/.test(version) ? `v${version}` : version;
}

export function AddonCard({
  name,
  version,
  installed,
  enabled,
  description,
  copy,
  infoOpen,
  onToggleInfo,
  busy,
  busyLabel,
  resultMessage,
  onDismissResult,
  sourceUrl,
  onOpenSource,
  connectors,
  showClients,
  actionsDisabled,
  onInstall,
  onToggleEnabled,
  onUninstall,
  children,
}: {
  name: string;
  version?: string | null;
  installed: boolean;
  enabled: boolean;
  description: ReactNode;
  copy?: AddonCopy;
  infoOpen: boolean;
  onToggleInfo: () => void;
  busy: boolean;
  busyLabel: string | null;
  resultMessage: string | null;
  onDismissResult: () => void;
  sourceUrl: string;
  onOpenSource: () => void;
  connectors: ClientConnectorStatus[];
  showClients: boolean;
  actionsDisabled: boolean;
  onInstall: () => void;
  onToggleEnabled: () => void;
  onUninstall: () => void;
  children?: ReactNode;
}) {
  return (
    <li className="addon-card">
      <div className="addon-card__body">
        <div className="addon-card__heading">
          <span className="addon-card__name">{name}</span>
          {installed && version ? (
            <span className="addon-card__version">
              {formatAddonVersion(version)}
            </span>
          ) : null}
          {copy ? (
            <button
              type="button"
              className="addon-card__info"
              aria-label={`What ${name} does`}
              aria-expanded={infoOpen}
              onClick={onToggleInfo}
            >
              i
            </button>
          ) : null}
          {installed ? (
            <span
              className={`addon-card__badge addon-card__badge--${enabled ? "on" : "off"}`}
            >
              {enabled ? "Enabled" : "Disabled"}
            </span>
          ) : null}
        </div>
        {infoOpen && copy ? (
          <p className="addon-card__info-text">{copy.whatItDoes}</p>
        ) : null}
        <p className="addon-card__description">{description}</p>
        {showClients ? <AddonClientChips connectors={connectors} /> : null}
        <button
          type="button"
          className="addon-card__link"
          onClick={onOpenSource}
        >
          {sourceUrl}
        </button>
        {busy && busyLabel ? (
          <p className="addon-card__progress">{busyLabel}</p>
        ) : resultMessage ? (
          <p className="addon-card__result">
            {resultMessage}
            <button
              type="button"
              className="addon-card__result-dismiss"
              aria-label="Dismiss"
              onClick={onDismissResult}
            >
              ×
            </button>
          </p>
        ) : null}
        {children}
      </div>
      <div className="addon-card__actions">
        {!installed ? (
          <button
            type="button"
            className="addon-card__action addon-card__action--primary"
            disabled={actionsDisabled}
            onClick={onInstall}
          >
            Install
          </button>
        ) : (
          <>
            <button
              type="button"
              className="addon-card__action"
              disabled={actionsDisabled}
              onClick={onToggleEnabled}
            >
              {enabled ? "Disable" : "Enable"}
            </button>
            <button
              type="button"
              className="addon-card__action addon-card__action--danger"
              disabled={actionsDisabled}
              onClick={onUninstall}
            >
              Uninstall
            </button>
          </>
        )}
      </div>
    </li>
  );
}
