import type {
  ClientConnectorStatus,
  RuntimeStatus,
  SwitchboardMode,
} from "./types";

export function switchboardModeLabel(mode: SwitchboardMode): string {
  switch (mode) {
    case "full":
      return "Full optimization";
    case "headroom":
      return "Headroom only";
    case "rtk":
      return "RTK only";
    case "off":
    default:
      return "Off";
  }
}

export function switchboardModeSummary(mode: SwitchboardMode): string {
  switch (mode) {
    case "full":
      return "Headroom proxy routing and RTK command compression are both active.";
    case "headroom":
      return "LLM traffic is routed through Headroom. RTK command compression is off.";
    case "rtk":
      return "RTK command compression is active. Coding clients are not routed through Headroom.";
    case "off":
    default:
      return "No optimization layer is active right now.";
  }
}

export function switchboardModeEffect(mode: SwitchboardMode): string {
  switch (mode) {
    case "full":
      return "Routes supported clients through Headroom and compresses shell output with RTK.";
    case "headroom":
      return "Routes supported clients through Headroom while leaving shell output unchanged.";
    case "rtk":
      return "Keeps client traffic direct and compresses shell output with RTK.";
    case "off":
    default:
      return "Removes routing hooks and leaves client traffic and shell commands unmodified.";
  }
}

export function switchboardModeSafetyNotes(mode: SwitchboardMode): string[] {
  switch (mode) {
    case "full":
      return [
        "Client routing and RTK shell compression are both managed by Mac AI Switchboard.",
        "Use Doctor if a supported client is installed but not verified.",
      ];
    case "headroom":
      return [
        "Client routing is managed, but shell output is not rewritten by RTK.",
        "Use RTK only if several active Codex goals are putting pressure on compression.",
      ];
    case "rtk":
      return [
        "Coding clients bypass Headroom while RTK can still compact command output.",
        "Return to Full optimization after compacting long Codex conversations.",
      ];
    case "off":
    default:
      return [
        "Routing hooks and RTK shell integration are disabled for normal client behavior.",
        "Repo Intelligence summaries remain local until cleared from Addons.",
      ];
  }
}

export interface SwitchboardModeFootprint {
  label: string;
  state: "on" | "off" | "local";
  detail: string;
}

export function switchboardModeFootprint(
  mode: SwitchboardMode,
): SwitchboardModeFootprint[] {
  switch (mode) {
    case "full":
      return [
        {
          label: "Client routing",
          state: "on",
          detail: "Managed through Headroom",
        },
        {
          label: "Shell output",
          state: "on",
          detail: "RTK compacts noisy commands",
        },
        {
          label: "Repo packs",
          state: "local",
          detail: "Local copy/export only",
        },
      ];
    case "headroom":
      return [
        {
          label: "Client routing",
          state: "on",
          detail: "Managed through Headroom",
        },
        { label: "Shell output", state: "off", detail: "RTK hooks disabled" },
        {
          label: "Repo packs",
          state: "local",
          detail: "Local copy/export only",
        },
      ];
    case "rtk":
      return [
        {
          label: "Client routing",
          state: "off",
          detail: "Clients use provider directly",
        },
        {
          label: "Shell output",
          state: "on",
          detail: "RTK compacts noisy commands",
        },
        {
          label: "Repo packs",
          state: "local",
          detail: "Local copy/export only",
        },
      ];
    case "off":
    default:
      return [
        {
          label: "Client routing",
          state: "off",
          detail: "Managed routing removed",
        },
        { label: "Shell output", state: "off", detail: "RTK hooks disabled" },
        {
          label: "Repo packs",
          state: "local",
          detail: "Saved locally until cleared",
        },
      ];
  }
}

export function switchboardAttentionCopy(
  desiredMode: SwitchboardMode,
  effectiveMode: SwitchboardMode,
): string {
  if (desiredMode === effectiveMode) {
    return "";
  }
  const effectiveModeLabel = switchboardModeLabel(effectiveMode);
  switch (desiredMode) {
    case "full":
      return effectiveMode === "rtk"
        ? `Active now: ${effectiveModeLabel}. Connect a supported client or repair Headroom routing in Doctor.`
        : `Active now: ${effectiveModeLabel}. Run Doctor to restore Headroom and RTK together.`;
    case "headroom":
      return `Active now: ${effectiveModeLabel}. Connect a supported client or repair Headroom routing in Doctor.`;
    case "rtk":
      return `Active now: ${effectiveModeLabel}. Install or enable RTK from Doctor or Addons.`;
    case "off":
    default:
      return `Active now: ${effectiveModeLabel}. Use Doctor if local routing hooks need cleanup.`;
  }
}

export function deriveSwitchboardMode(
  runtime: RuntimeStatus | null,
  enabledClients: ClientConnectorStatus[],
): SwitchboardMode {
  const rtkEnabled =
    runtime?.rtk.installed === true && runtime.rtk.enabled === true;
  const headroomEnabled =
    runtime?.running === true &&
    runtime.proxyReachable === true &&
    runtime.paused !== true &&
    enabledClients.length > 0;

  if (headroomEnabled && rtkEnabled) {
    return "full";
  }
  if (headroomEnabled) {
    return "headroom";
  }
  if (rtkEnabled) {
    return "rtk";
  }
  return "off";
}

export function formatSwitchboardModeShareText({
  requestedMode,
  effectiveMode,
  needsAttention,
  summary,
}: {
  requestedMode: SwitchboardMode;
  effectiveMode?: SwitchboardMode;
  needsAttention?: boolean;
  summary: string;
}): string {
  const activeMode = effectiveMode ?? requestedMode;
  const requestedLabel = switchboardModeLabel(requestedMode);
  const activeLabel = switchboardModeLabel(activeMode);
  const attentionCopy = needsAttention
    ? switchboardAttentionCopy(requestedMode, activeMode)
    : "";
  const footprint = switchboardModeFootprint(requestedMode).map(
    (item) => `- ${item.label}: ${item.state} (${item.detail})`,
  );

  return [
    "Mac AI Switchboard mode state",
    `Requested mode: ${requestedLabel}`,
    `Active mode: ${activeLabel}`,
    `Needs attention: ${needsAttention ? "yes" : "no"}`,
    `Summary: ${summary}`,
    ...(attentionCopy ? [`Attention: ${attentionCopy}`] : []),
    `Effect: ${switchboardModeEffect(requestedMode)}`,
    "Safety notes:",
    ...switchboardModeSafetyNotes(requestedMode).map((note) => `- ${note}`),
    "Local footprint:",
    ...footprint,
  ].join("\n");
}
