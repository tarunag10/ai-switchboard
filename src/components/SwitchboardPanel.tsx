import { switchboardModeLabel } from "../lib/switchboardDisplay";
import type { SwitchboardMode } from "../lib/types";

interface SwitchboardPanelProps {
  mode: SwitchboardMode;
  summary: string;
  localOnly: boolean;
  proxyStatus: string;
  headroomDetail: string;
  rtkStatus: string;
  rtkDetail: string;
  remoteServicesEnabled: boolean;
  paused: boolean;
  resuming: boolean;
  onResume: () => void;
  onManageClients: () => void;
  onManageRtk: () => void;
}

export function SwitchboardPanel({
  mode,
  summary,
  localOnly,
  proxyStatus,
  headroomDetail,
  rtkStatus,
  rtkDetail,
  remoteServicesEnabled,
  paused,
  resuming,
  onResume,
  onManageClients,
  onManageRtk
}: SwitchboardPanelProps) {
  const modeLabel = switchboardModeLabel(mode);

  return (
    <section className="switchboard-panel" aria-label="Local switchboard status">
      <div className="switchboard-panel__head">
        <div>
          <p className="switchboard-panel__eyebrow">
            {localOnly ? "Local-only Mac setup" : "Headroom cloud setup"}
          </p>
          <h2>{modeLabel}</h2>
        </div>
        <span className={`switchboard-panel__badge switchboard-panel__badge--${mode}`}>
          {modeLabel}
        </span>
      </div>
      <p className="switchboard-panel__copy">{summary}</p>
      <div className="switchboard-panel__grid">
        <div className="switchboard-panel__item">
          <span className="switchboard-panel__label">Headroom proxy</span>
          <strong>{proxyStatus}</strong>
          <small>{headroomDetail}</small>
        </div>
        <div className="switchboard-panel__item">
          <span className="switchboard-panel__label">RTK</span>
          <strong>{rtkStatus}</strong>
          <small>{rtkDetail}</small>
        </div>
        <div className="switchboard-panel__item">
          <span className="switchboard-panel__label">Remote services</span>
          <strong>{remoteServicesEnabled ? "Available" : "Off"}</strong>
          <small>
            {remoteServicesEnabled
              ? "Account features enabled"
              : "No pricing, trial, Clarity, or Sentry calls"}
          </small>
        </div>
      </div>
      <div className="switchboard-panel__actions">
        {paused ? (
          <button
            type="button"
            className="switchboard-panel__action switchboard-panel__action--primary"
            onClick={onResume}
            disabled={resuming}
          >
            {resuming ? "Restarting…" : "Resume Headroom"}
          </button>
        ) : null}
        <button type="button" className="switchboard-panel__action" onClick={onManageClients}>
          Manage clients
        </button>
        <button type="button" className="switchboard-panel__action" onClick={onManageRtk}>
          Manage RTK
        </button>
      </div>
    </section>
  );
}

