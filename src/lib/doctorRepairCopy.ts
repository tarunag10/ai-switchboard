import { codexDoctorHint } from "./codexErrorGuidance";
import {
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorReadinessContract,
  plannedConnectors,
} from "./plannedConnectors";
import type { DoctorIssue, DoctorReport } from "./types";

export type DoctorTimelineEventKind =
  | "install"
  | "enable"
  | "disable"
  | "repair"
  | "backup"
  | "rollback"
  | "failed_repair"
  | "index_refresh"
  | "connector_setup";

export interface DoctorTimelineEvent {
  id: string;
  kind: DoctorTimelineEventKind;
  title: string;
  body: string;
  occurredAt: string;
  status: "ok" | "warning" | "error";
  actor: "switchboard" | "doctor" | "user";
  target?: string | null;
}

export function doctorRepairLabel(action: string): string {
  switch (action) {
    case "verify_off_mode":
      return "Verify Off";
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
    case "repair_caveman_guidance":
      return "Repair Caveman";
    case "repair_ponytail_plugin":
      return "Repair Ponytail";
    case "clear_repo_intelligence_index":
      return "Clear index";
    default:
      return "Repair";
  }
}

export function doctorTimelineKindLabel(kind: DoctorTimelineEventKind): string {
  switch (kind) {
    case "failed_repair":
      return "Failed repair";
    case "index_refresh":
      return "Index refresh";
    case "connector_setup":
      return "Connector setup";
    default:
      return kind.replace(/_/g, " ").replace(/^\w/, (match) =>
        match.toUpperCase(),
      );
  }
}

export function sortDoctorTimelineEvents(
  events: DoctorTimelineEvent[],
): DoctorTimelineEvent[] {
  return [...events].sort((left, right) => {
    const timeDelta =
      Date.parse(right.occurredAt) - Date.parse(left.occurredAt);
    return timeDelta || left.title.localeCompare(right.title);
  });
}

export function formatDoctorTimelineShareText(
  events: DoctorTimelineEvent[],
): string {
  const sorted = sortDoctorTimelineEvents(events);
  if (sorted.length === 0) {
    return "Mac AI Switchboard Doctor timeline\nNo Doctor timeline events recorded.";
  }

  return [
    "Mac AI Switchboard Doctor timeline",
    `Events: ${sorted.length}`,
    "",
    ...sorted.flatMap((event, index) => [
      `${index + 1}. ${event.title}`,
      `Kind: ${doctorTimelineKindLabel(event.kind)}`,
      `Status: ${event.status}`,
      `Actor: ${event.actor}`,
      `When: ${event.occurredAt}`,
      `Target: ${event.target ?? "not recorded"}`,
      `Body: ${event.body}`,
      "",
    ]),
  ]
    .join("\n")
    .trimEnd();
}

export function doctorRepairHint(action: string): string {
  const codexHint = codexDoctorHint(action);
  if (codexHint) {
    return codexHint;
  }

  switch (action) {
    case "verify_off_mode":
      return "Doctor will re-check active engine, client, and RTK evidence without changing local routing.";
    case "repair_runtime":
      return "Restarts the local Headroom engine and refreshes switchboard status.";
    case "repair_client_setups":
      return "Re-applies reversible setup for installed managed clients.";
    case "repair_rtk_integrations":
      return "Restores RTK PATH and hook wiring without reinstalling the binary.";
    case "repair_rtk_runtime":
      return "Installs or enables RTK in managed storage for local shell-output compression.";
    case "repair_caveman_guidance":
      return "Recreates the Caveman receipt and rewrites the managed guidance block for configured Claude Code and Codex instruction files.";
    case "repair_ponytail_plugin":
      return "Re-registers the Ponytail plugin with available Claude Code and Codex hosts.";
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

export function plannedConnectorDoctorGuidance(): string {
  const firstBlockedStage =
    plannedConnectors
      .map((connector) =>
        getPlannedConnectorReadinessContract(connector).stages.find(
          (stage) => stage.state === "blocked",
        ),
      )
      .find(Boolean)?.label ?? "backup coverage";
  const badgeLabels = new Set(
    plannedConnectors.flatMap((connector) =>
      getPlannedConnectorReadinessBadges(connector).map((badge) => badge.label),
    ),
  );

  return [
    "Open Settings and review each planned connector's detection evidence, readiness stages, safety badges, and manual guide.",
    `Doctor keeps these as manual steps because the next automation gate is ${firstBlockedStage.toLowerCase()}.`,
    `Look for ${Array.from(badgeLabels).join(", ")} before choosing a workflow.`,
    "Use RTK-only mode or Repo Intelligence packs; keep provider routing manual until backup, verify, rollback, and Off mode cleanup are available.",
  ].join(" ");
}

export function doctorIssueGuidance(issue: DoctorIssue): string {
  if (doctorIssueActionKind(issue.repairAction) === "automatic") {
    return doctorRepairHint(issue.repairAction as string);
  }

  switch (issue.id) {
    case "switchboard_mode_degraded":
      return "Requested mode and active mode differ. Run automatic repairs for runtime, client, or RTK issues below, complete any manual connector steps that remain, then re-run Doctor until requested mode becomes active.";
    case "planned_connectors_detected":
      return plannedConnectorDoctorGuidance();
    case "repo_intelligence_repo_missing":
      return "Clear the saved Repo Intelligence index, then open Addons and index an available local repo when ready.";
    case "repo_intelligence_stale":
      return "Clear the stale saved Repo Intelligence index, then open Addons and re-index the repo before copying packs into another agent.";
    case "repo_intelligence_storage_corrupt":
      return "Clear the unreadable Repo Intelligence index, then open Addons and re-index a local repo before copying packs into another agent.";
    case "headroom_paused":
      return "Choose Full optimization or Headroom only to resume routing, or stay in Off mode if you want clients to bypass Headroom.";
    case "off_mode_not_clean":
      return "Run Verify Off after disabling routing or restarting affected shells; Doctor will re-check active engine, client, and RTK evidence.";
    default:
      return doctorIssueActionHint(issue.repairAction);
  }
}

export function formatDoctorReportShareText(report: DoctorReport): string {
  const lines = [
    "Mac AI Switchboard Doctor report",
    `Status: ${report.status}`,
    `Summary: ${report.summary}`,
    `Issues: ${report.issues.length}`,
  ];

  if (report.issues.length === 0) {
    return [...lines, "No Doctor issues found."].join("\n");
  }

  return [
    ...lines,
    "",
    ...report.issues.flatMap((issue, index) => {
      const actionKind = doctorIssueActionKind(issue.repairAction);
      const repairLabel = canRepairIssue(issue.repairAction)
        ? doctorRepairLabel(issue.repairAction as string)
        : "Manual step";

      return [
        `${index + 1}. ${issue.title}`,
        `Severity: ${issue.severity}`,
        `Action: ${actionKind} / ${repairLabel}`,
        `Body: ${issue.body}`,
        `Guidance: ${doctorIssueGuidance(issue)}`,
        "",
      ];
    }),
  ]
    .join("\n")
    .trimEnd();
}
