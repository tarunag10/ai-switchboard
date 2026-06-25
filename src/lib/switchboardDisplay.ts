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
      return "RTK command compression is active. No coding client is routed through Headroom.";
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
      return "Leaves client traffic and shell commands unmodified.";
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
