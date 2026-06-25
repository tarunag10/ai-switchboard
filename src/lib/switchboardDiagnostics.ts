import {
  switchboardAttentionCopy,
  switchboardModeLabel,
} from "./switchboardDisplay";
import type { SwitchboardMode } from "./types";

export interface SwitchboardModeDiagnostic {
  requestedLabel: string;
  effectiveLabel: string;
  attentionCopy: string;
}

export function switchboardModeDiagnostic(
  requestedMode: SwitchboardMode,
  effectiveMode: SwitchboardMode | undefined,
  needsAttention: boolean | undefined,
): SwitchboardModeDiagnostic {
  const resolvedEffectiveMode = effectiveMode ?? requestedMode;
  const requestedLabel = switchboardModeLabel(requestedMode);
  const effectiveLabel = switchboardModeLabel(resolvedEffectiveMode);
  const attentionCopy =
    needsAttention === true
      ? switchboardAttentionCopy(requestedMode, resolvedEffectiveMode) ||
        `Active now: ${effectiveLabel}. Run Doctor to repair.`
      : "";

  return {
    requestedLabel,
    effectiveLabel,
    attentionCopy,
  };
}
