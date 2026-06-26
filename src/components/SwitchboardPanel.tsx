import { useState } from "react";
import {
  Brain,
  Copy,
  Power,
  Sparkle,
  Terminal,
  type Icon,
} from "@phosphor-icons/react";

import { codexConcurrencyGuidance } from "../lib/codexConcurrencyGuidance";
import { localOnlySetupLabel, remoteServicesCopy } from "../lib/remoteServices";
import { switchboardModeDiagnostic } from "../lib/switchboardDiagnostics";
import {
  formatSwitchboardModeShareText,
  switchboardModeEffect,
  switchboardModeFootprint,
  switchboardModeLabel,
  switchboardModeSafetyNotes,
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
  const modeSafetyNotes = switchboardModeSafetyNotes(mode);
  const modeFootprint = switchboardModeFootprint(mode);
  const setupLabel = localOnlySetupLabel(localOnly);
  const remoteCopy = remoteServicesCopy(remoteServicesEnabled);
  const codexGuidance = codexConcurrencyGuidance(mode, headroomDetail);

  const [copyNotice, setCopyNotice] = useState<string | null>(null);

  async function copySwitchboardState() {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }

    await navigator.clipboard.writeText(
      formatSwitchboardModeShareText({
        requestedMode: mode,
        effectiveMode,
        needsAttention,
        summary,
      }),
    );
    setCopyNotice("Copied state.");
  }

  return (
    <section
      className="switchboard-panel"
      aria-label="Local switchboard status"
    >
      <div className="switchboard-panel__head">
        <div>
          <p className="switchboard-panel__eyebrow">{setupLabel}</p>
          <h2>{modeLabel}</h2>
        </div>
        <div className="switchboard-panel__head-actions">
          <button
            type="button"
            className="switchboard-panel__copy-state"
            onClick={copySwitchboardState}
            title="Copy switchboard state"
          >
            <Copy aria-hidden="true" weight="bold" />
            <span>{copyNotice ?? "Copy state"}</span>
          </button>
          <span
            className={`switchboard-panel__badge switchboard-panel__badge--${mode}`}
          >
            {modeLabel}
          </span>
        </div>
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
                {modeBusy === option
                  ? "Applying"
                  : switchboardModeLabel(option)}
              </span>
            </button>
          );
        })}
      </div>
      <p className="switchboard-panel__mode-effect">{modeEffect}</p>
      <ul
        className="switchboard-panel__mode-notes"
        aria-label={`${modeLabel} safety notes`}
      >
        {modeSafetyNotes.map((note) => (
          <li key={note}>{note}</li>
        ))}
      </ul>
      <div
        className="switchboard-panel__footprint"
        aria-label={`${modeLabel} local footprint`}
      >
        {modeFootprint.map((item) => (
          <div className="switchboard-panel__footprint-item" key={item.label}>
            <span className="switchboard-panel__footprint-label">
              {item.label}
            </span>
            <strong
              className={`switchboard-panel__footprint-state switchboard-panel__footprint-state--${item.state}`}
            >
              {item.state === "on"
                ? "On"
                : item.state === "off"
                  ? "Off"
                  : "Local"}
            </strong>
            <small>{item.detail}</small>
          </div>
        ))}
      </div>
      {codexGuidance ? (
        <div className="switchboard-panel__recommendation">
          <div>
            <strong>{codexGuidance.title}</strong>
            <p>{codexGuidance.body}</p>
            <div
              className="switchboard-panel__recommendation-policies"
              aria-label="Codex parallel-session policy"
            >
              {codexGuidance.policies.map((policy) => (
                <span key={policy}>{policy}</span>
              ))}
            </div>
            <ul
              className="switchboard-panel__recommendation-steps"
              aria-label="Codex multiple-goal prevention steps"
            >
              {codexGuidance.steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ul>
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
