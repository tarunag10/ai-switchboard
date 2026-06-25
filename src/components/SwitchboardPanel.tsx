import {
  Brain,
  Power,
  Sparkle,
  Terminal,
  type Icon,
} from "@phosphor-icons/react";

import { codexConcurrencyGuidance } from "../lib/codexConcurrencyGuidance";
import {
  localOnlySetupLabel,
  remoteServicesCopy,
} from "../lib/remoteServices";
import { switchboardModeDiagnostic } from "../lib/switchboardDiagnostics";
import {
  switchboardModeEffect,
  switchboardModeLabel,
} from "../lib/switchboardDisplay";
import type { SwitchboardMode } from "../lib/types";

interface SwitchboardPanelProps {
  mode: SwitchboardMode;
  effectiveMode?: SwitchboardMode;
  needsAttention?: boolean;
  summary: string;
  localOnly: boolean;
  proxyStatus: string;
  headroomDetail: string;
  rtkStatus: string;
  rtkDetail: string;
  remoteServicesEnabled: boolean;
  paused: boolean;
  resuming: boolean;
  modeBusy: SwitchboardMode | null;
  modeError: string | null;
  onSetMode: (mode: SwitchboardMode) => void;
  onResume: () => void;
  onManageClients: () => void;
  onManageRtk: () => void;
}

const SWITCHBOARD_MODES: SwitchboardMode[] = ["full", "headroom", "rtk", "off"];
const SWITCHBOARD_MODE_ICONS: Record<SwitchboardMode, Icon> = {
  full: Sparkle,
  headroom: Brain,
  rtk: Terminal,
  off: Power,
};

export function SwitchboardPanel({
  mode,
  effectiveMode,
  needsAttention,
  summary,
  localOnly,
  proxyStatus,
  headroomDetail,
  rtkStatus,
  rtkDetail,
  remoteServicesEnabled,
  paused,
  resuming,
  modeBusy,
  modeError,
  onSetMode,
  onResume,
  onManageClients,
  onManageRtk,
}: SwitchboardPanelProps) {
  const modeDiagnostic = switchboardModeDiagnostic(
    mode,
    effectiveMode,
    needsAttention,
  );
  const modeLabel = modeDiagnostic.requestedLabel;
  const modeEffect = switchboardModeEffect(mode);
  const setupLabel = localOnlySetupLabel(localOnly);
  const remoteCopy = remoteServicesCopy(remoteServicesEnabled);
  const codexGuidance = codexConcurrencyGuidance(mode, headroomDetail);

  return (
    <section
      className="switchboard-panel"
      aria-label="Local switchboard status"
    >
      <div className="switchboard-panel__head">
        <div>
          <p className="switchboard-panel__eyebrow">
            {setupLabel}
          </p>
          <h2>{modeLabel}</h2>
        </div>
        <span
          className={`switchboard-panel__badge switchboard-panel__badge--${mode}`}
        >
          {modeLabel}
        </span>
      </div>
      <p className="switchboard-panel__copy">{summary}</p>
      {needsAttention ? (
        <p className="switchboard-panel__attention">
          {modeDiagnostic.attentionCopy}
        </p>
      ) : null}
      <div
        className="switchboard-panel__modes"
        role="group"
        aria-label="Switch optimization mode"
      >
        {SWITCHBOARD_MODES.map((option) => {
          const ModeIcon = SWITCHBOARD_MODE_ICONS[option];

          return (
            <button
              key={option}
              type="button"
              className={`switchboard-panel__mode${option === mode ? " is-active" : ""}`}
              onClick={() => onSetMode(option)}
              disabled={modeBusy !== null}
              aria-pressed={option === mode}
              aria-label={
                modeBusy === option
                  ? `Applying ${switchboardModeLabel(option)}`
                  : `${switchboardModeLabel(option)}: ${switchboardModeEffect(option)}`
              }
              title={switchboardModeEffect(option)}
            >
              <ModeIcon aria-hidden weight="duotone" />
              <span>
                {modeBusy === option ? "Applying" : switchboardModeLabel(option)}
              </span>
            </button>
          );
        })}
      </div>
      <p className="switchboard-panel__mode-effect">{modeEffect}</p>
      {codexGuidance ? (
        <div className="switchboard-panel__recommendation">
          <div>
            <strong>{codexGuidance.title}</strong>
            <p>{codexGuidance.body}</p>
          </div>
          <button
            type="button"
            className="switchboard-panel__recommendation-action"
            disabled={modeBusy !== null}
            onClick={() => onSetMode(codexGuidance.recommendedMode)}
          >
            {modeBusy === codexGuidance.recommendedMode
              ? "Applying"
              : codexGuidance.actionLabel}
          </button>
        </div>
      ) : null}
      {modeError ? (
        <p className="switchboard-panel__error">{modeError}</p>
      ) : null}
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
            <strong>{remoteCopy.label}</strong>
            <small>{remoteCopy.detail}</small>
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
        <button
          type="button"
          className="switchboard-panel__action"
          onClick={onManageClients}
        >
          Manage clients
        </button>
        <button
          type="button"
          className="switchboard-panel__action"
          onClick={onManageRtk}
        >
          Manage RTK
        </button>
      </div>
    </section>
  );
}
