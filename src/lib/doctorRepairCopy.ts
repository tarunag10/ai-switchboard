import { codexDoctorHint } from "./codexErrorGuidance";
import type { DoctorIssue } from "./types";

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
    case "clear_repo_intelligence_index":
      return "Clear index";
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
      return "Restarts the local Headroom engine and refreshes switchboard status.";
    case "repair_client_setups":
      return "Re-applies reversible setup for installed managed clients.";
    case "repair_rtk_integrations":
      return "Restores RTK PATH and hook wiring without reinstalling the binary.";
    case "repair_rtk_runtime":
      return "Installs or enables RTK in managed storage for local shell-output compression.";
    case "clear_repo_intelligence_index":
      return "Clears the saved Repo Intelligence summary so a stale or missing repo path no longer appears in Doctor. Re-index from Addons when ready.";
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

export function doctorIssueActionLabel(action: string | null | undefined): string {
  return doctorIssueActionKind(action) === "automatic"
    ? "Auto repair"
    : "Manual step";
}

export function doctorIssueActionHint(action: string | null | undefined): string {
  return doctorIssueActionKind(action) === "automatic"
    ? doctorRepairHint(action as string)
    : "No automatic repair is available yet. Follow the issue guidance, then re-run Doctor.";
}

export function doctorIssueGuidance(issue: DoctorIssue): string {
  if (doctorIssueActionKind(issue.repairAction) === "automatic") {
    return doctorRepairHint(issue.repairAction as string);
  }

  switch (issue.id) {
case "switchboard_mode_degraded":
return "Requested mode and active mode differ. Review automatic repairs below, then keep manual connector or install steps visible until requested mode becomes active.";
    case "planned_connectors_detected":
      return "Open Settings, review detected evidence and each planned connector guide. Use RTK-only mode or Repo Intelligence packs; keep provider routing manual until backup, restore, and Off mode cleanup are available.";
    case "repo_intelligence_repo_missing":
      return "Clear the saved Repo Intelligence index, then open Addons and index an available local repo when ready.";
    case "repo_intelligence_stale":
      return "Clear the stale saved Repo Intelligence index, then open Addons and re-index the repo before copying packs into another agent.";
    case "headroom_paused":
      return "Choose Full optimization or Headroom only to resume routing, or stay in Off mode if you want clients to bypass Headroom.";
    default:
      return doctorIssueActionHint(issue.repairAction);
  }
}
