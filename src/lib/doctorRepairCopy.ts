import { codexDoctorHint } from "./codexErrorGuidance";
import type { ManagedChangeRecord } from "./managedChanges";
import {
  repoMemoryMcpInstallCommand,
  repoMemoryMcpVerifyCommand,
} from "./repoMemoryMcp";
import {
  getPlannedConnectorConfigCreationPlan,
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorReadinessContract,
  getPlannedConnectorSafetyDossier,
  pendingPlannedConnectors,
} from "./plannedConnectors";
import type { DoctorIssue, DoctorReport } from "./types";

export interface PlannedConnectorDoctorPreviewRow {
  id: string;
  name: string;
  setupPhase: string;
  nextBlockedGate: string;
  automationEnabled: boolean;
  configSurface: string;
}

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
    case "install_repo_memory_mcp":
      return "Install MCP";
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

export function buildManagedChangeTimelineEvents(
  records: ManagedChangeRecord[],
  observedAt: string,
): DoctorTimelineEvent[] {
  return records.map((record) => ({
    id: `managed-change-${record.id}`,
    kind: record.backupPath ? "backup" : "rollback",
    title: `${record.owner} rollback coverage`,
    body: [
      record.rollback,
      "Per-change rollback: available from the rollback center when this managed footprint is present.",
      `Backup: ${record.backupPath ?? "not required"}.`,
      `Marker: ${record.markerId}.`,
      record.backupPath
        ? "Dry-run diff available; copied previews do not modify files and apply requires explicit confirmation."
        : "No config diff is required for this managed footprint.",
      record.backupPath
        ? "Apply gate: target, backup path, marker, rollback plan, and Off-mode cleanup boundary must be confirmed first."
        : "Apply gate: not applicable because this footprint is removed through cleanup inventory.",
    ].join(" "),
    occurredAt: observedAt,
    status: "warning",
    actor: "switchboard",
    target:
      record.paths.length > 0
        ? `${record.paths.length} managed path${record.paths.length === 1 ? "" : "s"}`
        : "managed footprint",
  }));
}

function doctorIssueTimelineKind(issue: DoctorIssue): DoctorTimelineEventKind {
  if (issue.severity === "error" && issue.repairAction) {
    return "failed_repair";
  }
  if (issue.id.startsWith("repo_intelligence_")) {
    return "index_refresh";
  }
  if (issue.id === "planned_connectors_detected") {
    return "connector_setup";
  }
  return issue.repairAction ? "repair" : "disable";
}

export function buildDoctorReportTimelineEvents(
  report: DoctorReport | null,
  successMessage: string | null,
  observedAt: string,
): DoctorTimelineEvent[] {
  const events: DoctorTimelineEvent[] = [
    {
      id: "latest-report",
      kind: "repair",
      title: report ? `Doctor status: ${report.status}` : "Doctor report pending",
      body: report?.summary ?? "Run Doctor to capture local setup evidence.",
      occurredAt: observedAt,
      status: report?.status ?? "warning",
      actor: "doctor",
      target: "switchboard setup",
    },
  ];

  for (const issue of report?.issues ?? []) {
    const repairLabel = issue.repairAction
      ? doctorRepairLabel(issue.repairAction)
      : "manual step";
    events.push({
      id: `doctor-issue-${issue.id}`,
      kind: doctorIssueTimelineKind(issue),
      title: issue.title,
      body: `${issue.body} Action: ${repairLabel}.`,
      occurredAt: observedAt,
      status: issue.severity,
      actor: "doctor",
      target: issue.repairAction ? repairLabel : "manual follow-up",
    });
  }

  if (successMessage) {
    events.push({
      id: "latest-repair-success",
      kind: "repair",
      title: "Latest repair completed",
      body: successMessage,
      occurredAt: observedAt,
      status: "ok",
      actor: "doctor",
      target: "automatic repair",
    });
  }

  return sortDoctorTimelineEvents(events);
}

function scrubTimelineText(value: string) {
  return value
    .replace(/~\/\S+/g, "[home-path]")
    .replace(/\/Users\/\S+/g, "[user-path]")
    .replace(/\b(?:sk|sk-proj|xai|ghp|github_pat)_[A-Za-z0-9_-]{12,}\b/g, "[secret]")
    .replace(
      /\b([A-Z][A-Z0-9_]*(?:API_KEY|TOKEN|SECRET|PASSWORD)\s*=\s*)[^\s,;]+/g,
      "$1[secret]",
    );
}

export function formatDoctorTimelineShareText(
  events: DoctorTimelineEvent[],
): string {
  const sorted = sortDoctorTimelineEvents(events);
  if (sorted.length === 0) {
    return [
      "Mac AI Switchboard Doctor timeline",
      "No Doctor timeline events recorded.",
      "",
      repoIntelligenceDoctorAvailabilityGates(),
    ].join("\n");
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
      `Target: ${event.target ? scrubTimelineText(event.target) : "not recorded"}`,
      `Body: ${scrubTimelineText(event.body)}`,
      "",
    ]),
    repoIntelligenceDoctorAvailabilityGates(),
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
      return "Clears the saved Repo Intelligence summary so stale, missing, moved, or replaced repo paths no longer appear in Doctor. Re-index the current local repo path from Addons when ready.";
    case "install_repo_memory_mcp":
      return "Installs the app-managed read-only Repo Memory MCP server, then run npm run check:repo-memory-mcp to verify repo_context_pack, repo_symbol_lookup, and repo_dependents_of.";
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
  if (pendingPlannedConnectors.length === 0) {
    return [
      "All planned connector setup gates have been promoted to managed sidecar coverage.",
      "Use Settings or Doctor repair for managed connector verification, rollback, and Off mode cleanup.",
      "Repo Intelligence packs remain available for agent handoffs, while provider-specific config mutation stays guarded behind explicit connector evidence.",
    ].join(" ");
  }

  const firstBlockedStage =
    pendingPlannedConnectors
      .map((connector) =>
        getPlannedConnectorReadinessContract(connector).stages.find(
          (stage) => stage.state === "blocked",
        ),
      )
      .find(Boolean)?.label ?? "backup coverage";
  const badgeLabels = new Set(
    pendingPlannedConnectors.flatMap((connector) =>
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

export function formatPlannedConnectorDoctorDossiers(): string {
  if (pendingPlannedConnectors.length === 0) {
    return [
      "Planned connector config readiness dossiers",
      "",
      "No pending planned connector dossiers remain; connector setup has managed sidecar coverage.",
    ].join("\n");
  }

  return [
    "Planned connector config readiness dossiers",
    "",
    ...pendingPlannedConnectors.flatMap((connector) => {
      const readiness = getPlannedConnectorReadinessContract(connector);
      const plan = getPlannedConnectorConfigCreationPlan(connector);
      const dossier = getPlannedConnectorSafetyDossier(connector.id);
      const nextBlockedStage =
        readiness.stages.find((stage) => stage.id === readiness.nextBlockedStage)
          ?.label ?? "None";
      const blockedStages = readiness.stages.filter(
        (stage) => stage.state === "blocked",
      );

      return [
        `## ${connector.name}`,
        `Connector ID: ${connector.id}`,
        `Config surface: ${dossier?.configPathStrategy ?? connector.configSurfaces.join(", ")}`,
        `Next blocked gate: ${nextBlockedStage}`,
        `Automation enabled: ${plan.automationEnabled ? "yes" : "no"}`,
        `Safety: ${plan.safetyNote}`,
        "Blocked automation gates:",
        ...(blockedStages.length > 0
          ? blockedStages.map(
              (stage) => `- ${stage.label}: ${stage.evidence}`,
            )
          : ["- None"]),
        "Gated config-creation steps:",
        ...plan.steps.map(
          (step) =>
            `- ${step.label}: ${step.detail} Required evidence: ${step.requiredEvidence.join(" ")}`,
        ),
        "",
      ];
    }),
  ]
    .join("\n")
    .trimEnd();
}

export function plannedConnectorDoctorPreviewRows(): PlannedConnectorDoctorPreviewRow[] {
  return pendingPlannedConnectors.map((connector) => {
    const readiness = getPlannedConnectorReadinessContract(connector);
    const dossier = getPlannedConnectorSafetyDossier(connector.id);
    const nextBlockedGate =
      readiness.stages.find((stage) => stage.id === readiness.nextBlockedStage)
        ?.label ?? "None";

    return {
      id: connector.id,
      name: connector.name,
      setupPhase: connector.setupPhase,
      nextBlockedGate,
      automationEnabled: readiness.automationEnabled,
      configSurface:
        dossier?.configPathStrategy ?? connector.configSurfaces.join(", "),
    };
  });
}

export function repoIntelligenceDoctorApiContract(): string {
  return [
    "Repo Intelligence local API contract",
    "- get_repo_manifest: read latest bounded manifest.",
    "- get_repo_pack: read one bounded context pack.",
    "- get_agent_handoff: read one bounded agent handoff, including planned connector config readiness, next gate, evidence requirements, config path strategy, account caveat, and rollback strategy when the target is a planned connector.",
    "- get_index_freshness: read API availability, freshness, graph availability, indexer/parser versions, indexed/skipped counts, and missing/stale index state.",
    "- clear_repo_index: clears only Switchboard managed index metadata; never mutates the user repo.",
    "Availability gates: missing, stale, corrupt, or moved repo indexes stay visible in Doctor until cleared or re-indexed.",
    "Safety: read-only by default, secret-like paths excluded, generated/vendor paths skipped, outputs bounded by pack/token budgets, parser version reported, graph availability reported.",
  ].join("\n");
}

export function repoIntelligenceDoctorAvailabilityGates(): string {
  return [
    "Repo Intelligence Doctor availability gates",
    "- get_index_freshness is the trust gate before agents use saved packs.",
    "- Missing index: copy actions stay blocked until a real local repo is indexed.",
    "- Stale index: Doctor must keep the stale state visible until the index is cleared or refreshed.",
    "- Corrupt index: clear_repo_index removes only Switchboard managed index metadata, then the repo must be re-indexed.",
    "- Moved repo path: clear the saved index or re-index the new local path before handoff.",
    "- Evidence to copy: API availability, graph availability, indexer/parser versions, indexed/skipped counts, secret exclusion, and read-only safety.",
    "- Repo Memory MCP lifecycle: install through Mac AI Switchboard before agent consumption.",
    `- Repo Memory MCP install action: ${repoMemoryMcpInstallCommand}.`,
    `- Repo Memory MCP smoke check: ${repoMemoryMcpVerifyCommand}.`,
    "- Repo Memory MCP tools must stay read-only: repo_context_pack, repo_symbol_lookup, and repo_dependents_of.",
  ].join("\n");
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
    case "repo_intelligence_repo_moved":
      return "Clear the saved Repo Intelligence index, then re-index the current local repo path before copying packs into another agent.";
    case "repo_intelligence_stale":
      return "Clear the stale saved Repo Intelligence index, then open Addons and re-index the repo before copying packs into another agent.";
    case "repo_intelligence_storage_corrupt":
      return "Clear the unreadable Repo Intelligence index, then open Addons and re-index a local repo before copying packs into another agent.";
    case "repo_memory_mcp_not_configured":
      return "Install Repo Memory MCP from Doctor, then run npm run check:repo-memory-mcp before asking supported agents to consume repo-memory tools.";
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
  const includesPlannedConnectorIssue = report.issues.some(
    (issue) => issue.id === "planned_connectors_detected",
  );

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
    ...(includesPlannedConnectorIssue
      ? ["", formatPlannedConnectorDoctorDossiers()]
      : []),
    "",
    repoIntelligenceDoctorApiContract(),
  ]
    .join("\n")
    .trimEnd();
}

export function formatVerifyOffModeShareText(report: DoctorReport): string {
  const offModeIssue = report.issues.find(
    (issue) =>
      issue.id === "off_mode_not_clean" ||
      issue.repairAction === "verify_off_mode",
  );

  return [
    "Mac AI Switchboard Verify Off report",
    `Status: ${offModeIssue ? "active routing evidence found" : "clean"}`,
    `Doctor status: ${report.status}`,
    `Doctor summary: ${report.summary}`,
    "Checks: active engine, enabled clients, RTK routing evidence",
    offModeIssue
      ? `Evidence: ${offModeIssue.body}`
      : "Evidence: no Off mode routing issue is present in the current Doctor report.",
    `Guidance: ${
      offModeIssue
        ? doctorIssueGuidance(offModeIssue)
        : "Stay in Off mode for bypassed routing, or choose another mode to resume managed routing."
    }`,
  ].join("\n");
}
