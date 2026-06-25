import { codexDoctorHint } from "./codexErrorGuidance";

export function doctorRepairLabel(action: string): string {
  switch (action) {
    case "repair_runtime":
      return "Restart Headroom";
    case "reset_codex_bypass":
      return "Reset Codex";
    case "repair_codex_setup":
      return "Repair Codex";
    case "repair_client_setups":
      return "Repair clients";
    case "repair_rtk_integrations":
      return "Repair RTK";
    case "repair_rtk_runtime":
      return "Install RTK";
    default:
      return "Repair";
  }
}

export function doctorRepairHint(action: string): string {
  const codexHint = codexDoctorHint(action);
  if (codexHint) {
    return codexHint;
  }

  switch (action) {
    case "repair_runtime":
      return "Restarts local Headroom engine and refreshes switchboard status.";
    case "repair_client_setups":
      return "Re-applies reversible setup for installed managed clients.";
    case "repair_rtk_integrations":
      return "Restores RTK PATH and hook wiring without reinstalling the binary.";
    case "repair_rtk_runtime":
      return "Installs or enables RTK in managed storage for local shell-output compression.";
    default:
      return "Runs the safest available repair for this issue.";
  }
}

export function canRepairIssue(action: string | null | undefined): boolean {
  return typeof action === "string" && action.length > 0;
}

export function doctorIssueActionKind(
  action: string | null | undefined,
): "automatic" | "manual" {
  return canRepairIssue(action) ? "automatic" : "manual";
}

export function doctorIssueActionLabel(
  action: string | null | undefined,
): string {
  return doctorIssueActionKind(action) === "automatic"
    ? "Auto repair"
    : "Manual step";
}

export function doctorIssueActionHint(
  action: string | null | undefined,
): string {
  return doctorIssueActionKind(action) === "automatic"
    ? doctorRepairHint(action as string)
    : "No automatic repair is available yet. Follow the issue guidance, then re-run Doctor.";
}
