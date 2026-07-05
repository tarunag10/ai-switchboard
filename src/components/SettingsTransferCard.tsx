import type { SettingsImportPreview } from "../lib/settingsTransfer";
import type { SavingsMode, SwitchboardMode } from "../lib/types";

export interface SettingsTransferCardProps {
  switchboardMode: SwitchboardMode;
  savingsMode: SavingsMode;
  connectorCount: number;
  addonCount: number;
  importText: string;
  importPreview: SettingsImportPreview | null;
  importBusy: boolean;
  notice: string | null;
  onCopyExport: () => void;
  onImportTextChange: (value: string) => void;
  onPreviewImport: () => void;
  onApplyImport: () => void;
}

export function SettingsTransferCard({
  switchboardMode,
  savingsMode,
  connectorCount,
  addonCount,
  importText,
  importPreview,
  importBusy,
  notice,
  onCopyExport,
  onImportTextChange,
  onPreviewImport,
  onApplyImport,
}: SettingsTransferCardProps) {
  return (
    <article className="soft-card panel-card settings-transfer-card">
      <div className="panel-card__header">
        <div>
          <h3>Settings import/export</h3>
          <p>
            Move safe AI Switchboard for Mac preferences without carrying secrets, local paths,
            message logs, billing state, or token history.
          </p>
        </div>
      </div>
      <div className="settings-transfer__summary">
        <span>
          Mode <strong>{switchboardMode}</strong>
        </span>
        <span>
          Savings <strong>{savingsMode}</strong>
        </span>
        <span>
          Connectors <strong>{connectorCount}</strong>
        </span>
        <span>
          Add-ons <strong>{addonCount}</strong>
        </span>
      </div>
      <p className="settings-transfer__note">
        Import applies only safe app preferences. Connector and add-on entries are shown as
        approval-review items so config writes still go through Doctor, Addons, and connector
        gates.
      </p>
      <div className="settings-transfer__actions">
        <button
          className="secondary-button secondary-button--small"
          onClick={() => onCopyExport()}
          type="button"
        >
          Copy settings export
        </button>
        {notice ? <span>{notice}</span> : null}
      </div>
      <textarea
        className="settings-transfer__textarea"
        onChange={(event) => onImportTextChange(event.target.value)}
        placeholder="Paste settings export JSON to preview safe preferences"
        rows={5}
        value={importText}
      />
      <div className="settings-transfer__actions">
        <button
          className="secondary-button secondary-button--small"
          disabled={importText.trim().length === 0}
          onClick={onPreviewImport}
          type="button"
        >
          Preview import
        </button>
        <button
          className="secondary-button secondary-button--small"
          disabled={importBusy || importText.trim().length === 0 || importPreview?.valid !== true}
          onClick={() => onApplyImport()}
          type="button"
        >
          {importBusy ? "Applying..." : "Apply safe preferences"}
        </button>
      </div>
      {importPreview ? (
        <div
          className={`settings-transfer__preview${
            importPreview.valid ? " is-valid" : " is-invalid"
          }`}
        >
          <strong>{importPreview.title}</strong>
          <p>{importPreview.detail}</p>
          {importPreview.errors.length > 0 ? (
            <ul>
              {importPreview.errors.map((error) => (
                <li key={error}>{error}</li>
              ))}
            </ul>
          ) : null}
          {Object.keys(importPreview.safePreferences).length > 0 ? (
            <p>
              Safe preferences:{" "}
              {Object.entries(importPreview.safePreferences)
                .map(([key, value]) => `${key} ${value}`)
                .join(", ")}
            </p>
          ) : null}
          {importPreview.migrationActions.length > 0 ? (
            <div
              className="settings-transfer__migration"
              aria-label="Settings migration actions"
            >
              {importPreview.migrationActions.slice(0, 8).map((action) => (
                <div
                  className={`settings-transfer__migration-row settings-transfer__migration-row--${action.status}`}
                  key={action.id}
                >
                  <span>{action.label}</span>
                  <strong>{action.status}</strong>
                  <small>{action.detail}</small>
                </div>
              ))}
            </div>
          ) : null}
          {importPreview.manualItems.length > 0 ? (
            <ul>
              {importPreview.manualItems.slice(0, 6).map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}
    </article>
  );
}

