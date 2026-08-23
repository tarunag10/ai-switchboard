import { invoke } from "@tauri-apps/api/core";

import type { OssCapabilityRegistry } from "./ossCapabilities";
import type { OssHarnessReplayReference } from "./ossHarnessReplay";
import type { SwitchboardMode } from "./types";

export type WorkbenchTaskClass = "coding" | "review" | "analysis" | "planning";
export type WorkbenchSessionStatus = "active" | "paused" | "cancelled" | "completed";
export type WorkbenchSessionAction = "pause" | "resume" | "cancel" | "complete";

export interface WorkbenchEvent {
  eventId: string;
  sessionId: string;
  sequence: number;
  kind: string;
  parentEventId: string | null;
  occurredAt: string;
}

export interface WorkbenchSession {
  schemaVersion: number;
  sessionId: string;
  workspaceDigest: string;
  taskClass: WorkbenchTaskClass;
  status: WorkbenchSessionStatus;
  parentSessionId: string | null;
  forkEventId: string | null;
  createdAt: string;
  updatedAt: string;
  executionMode: "plan_only";
  providerTraffic: "none";
  events: WorkbenchEvent[];
}

export interface RouterDecisionReference {
  decisionId: string;
  decisionStage: "observe" | "userApproved" | "automaticAllowlisted";
  routingMode: "observe_only";
  evidenceDigest: string;
}

export interface WorkbenchPlanPreset {
  schemaVersion: number;
  presetId: string;
  label: string;
  description: string;
  requiredCapabilityIds: string[];
  evidenceSource: "native_router_decision_receipt" | "native_router_and_replay_receipts";
  routingMode: "observe_only";
  executionMode: "plan_only";
  providerTraffic: "none";
  writesEnabled: false;
}

export interface WorkbenchAdapterReadiness {
  schemaVersion: number;
  adapterId: "codex" | "claude_code";
  adapterContractVersion: number;
  logicalBinary: "codex" | "claude";
  knownCandidatePresent: boolean;
  discoveryMode: "fixed_known_location_metadata_only";
  cliVersionProbeState: "not_probed";
  versionProbeReason: string;
  processStartEnabled: false;
  providerTraffic: "none";
  writesEnabled: false;
}

export interface WorkbenchAdapterCommandReadiness extends WorkbenchAdapterReadiness {
  adapterPlanId: string;
}

export interface WorkbenchProcessRunSpec {
  schemaVersion: number;
  runId: string;
  sessionId: string;
  adapterPlanId: string;
  adapterId: "codex" | "claude_code";
  adapterContractVersion: number;
  workspaceDigest: string;
  owner: "workbench_native";
  state: "not_started";
  startAuthorization: "not_granted";
  launchMode: "native_adapter_only";
  processGroup: "required_on_unix";
  stdin: "null";
  output: "piped_bounded_redacted";
  timeoutPolicy: "native_fixed_policy_required";
  cancellation: "group_sigterm_then_sigkill";
  providerTraffic: "none";
  writesEnabled: false;
}

export interface WorkbenchProcessStartGrantPolicy {
  confirmationTemplate: string;
  ttlSeconds: number;
  executionEnabled: false;
  providerTraffic: "none";
  writesEnabled: false;
}

export interface WorkbenchProcessStartGrantView {
  schemaVersion: number;
  grantId: string;
  sessionId: string;
  planId: string;
  processRunId: string;
  capabilityId: "adapter_process_start";
  issuedAt: string;
  expiresAt: string;
  effectiveState: "active" | "expired" | "revoked";
  executionEnabled: false;
  providerTraffic: "none";
  writesEnabled: false;
  receiptDigest: string;
}

export interface WorkbenchRunSpecInput {
  sessionId: string;
  adapterId: "claude_code" | "codex" | "gemini_cli";
  workspaceDigest: string;
  contextPackDigest: string | null;
  routerDecisionId: string;
  replayReferenceId: string | null;
  presetId: string | null;
  requiredCapabilityIds: string[];
  requestedMode: SwitchboardMode;
}

export interface WorkbenchProcessStartGrantInput {
  runSpec: WorkbenchRunSpecInput;
  expectedPlanId: string;
  expectedProcessRunId: string;
  confirmationPhrase: string;
}

export interface WorkbenchProcessAdmission {
  schemaVersion: number;
  admissionId: string;
  sessionId: string;
  planId: string;
  processRunId: string;
  grantId: string;
  adapterId: "codex";
  admittedAt: string;
  state: "authorized_not_started";
  executionEnabled: false;
  providerTraffic: "none";
  writesEnabled: false;
  receiptDigest: string;
}

export type WorkbenchProcessAdmissionEligibility =
  | "active"
  | "expired"
  | "revoked"
  | "session_paused"
  | "session_terminal"
  | "superseded"
  | "unavailable";

export type WorkbenchProcessAdmissionEligibilityReason =
  | "bound_and_current"
  | "grant_expired"
  | "clock_rollback"
  | "grant_revoked"
  | "grant_missing"
  | "session_paused"
  | "session_cancelled"
  | "session_completed"
  | "plan_changed"
  | "process_containment_changed"
  | "process_containment_removed";

export interface WorkbenchAdmissionEligibility extends WorkbenchProcessAdmission {
  currentEligibility: WorkbenchProcessAdmissionEligibility;
  reason: WorkbenchProcessAdmissionEligibilityReason;
  grantEffectiveState: "active" | "expired" | "revoked" | null;
  evaluatedAt: string;
  requiresStartRevalidation: true;
}

export interface WorkbenchAdmissionEligibilitySnapshot {
  schemaVersion: number;
  sessionId: string;
  evaluatedAt: string;
  currentPlanId: string | null;
  currentProcessRunId: string | null;
  receipts: WorkbenchAdmissionEligibility[];
  executionEnabled: false;
  providerTraffic: "none";
  writesEnabled: false;
}

export interface WorkbenchProcessAdmissionInput {
  runSpec: WorkbenchRunSpecInput;
  expectedPlanId: string;
  expectedProcessRunId: string;
  grantId: string;
}

export interface WorkbenchCapabilityRequest {
  capabilityId: string;
  scope: "session";
  approvalState: "pending";
  executionEnabled: false;
}

export interface WorkbenchRunPlan {
  schemaVersion: number;
  planId: string;
  sessionId: string;
  adapterId: string;
  workspaceDigest: string;
  contextPackDigest: string | null;
  routerDecision: RouterDecisionReference;
  replayReference: OssHarnessReplayReference | null;
  preset: WorkbenchPlanPreset | null;
  requestedMode: SwitchboardMode;
  adapterPlanId: string;
  adapterAction: "apply_managed_routing" | "cleanup_managed_routing";
  adapterReversible: boolean;
  commandReadiness: WorkbenchAdapterCommandReadiness | null;
  processContainment: WorkbenchProcessRunSpec | null;
  capabilityRequests: WorkbenchCapabilityRequest[];
  executionMode: "plan_only";
  providerTraffic: "none";
  writesEnabled: false;
}

export interface WorkbenchCapabilityProjection {
  schemaVersion: number;
  executionMode: "plan_only";
  writesEnabled: false;
  providerTraffic: "none";
  registry: OssCapabilityRegistry;
  presets: WorkbenchPlanPreset[];
  adapterReadiness: WorkbenchAdapterReadiness[];
  processStartGrantPolicy: WorkbenchProcessStartGrantPolicy;
}

export const WORKBENCH_CAPABILITIES = [
  {
    id: "repo_context",
    label: "Repo context",
    detail: "Reference a separately prepared, hashed local context pack.",
  },
  {
    id: "redacted_replay",
    label: "Redacted replay",
    detail: "Inspect a separately validated, content-free route-event replay.",
  },
  {
    id: "router_observe",
    label: "Router observation",
    detail: "Reference an observe-only Router decision and its evidence digest.",
  },
  {
    id: "client_adapter_plan",
    label: "Client adapter plan",
    detail: "Prepare an existing client adapter plan without applying it.",
  },
  {
    id: "adapter_command_readiness",
    label: "Adapter command readiness",
    detail: "Metadata-only Codex or Claude Code readiness; it never probes a CLI or starts a process.",
  },
] as const;

export function isWorkbenchDigest(value: string): boolean {
  return /^sha256:[a-fA-F0-9]{64}$/.test(value.trim());
}

export function createWorkbenchSession(input: {
  workspaceDigest: string;
  taskClass: WorkbenchTaskClass;
}): Promise<WorkbenchSession> {
  return invoke<WorkbenchSession>("create_workbench_session", { input });
}

export function listWorkbenchSessions(): Promise<WorkbenchSession[]> {
  return invoke<WorkbenchSession[]>("list_workbench_sessions");
}

export function getWorkbenchSession(sessionId: string): Promise<WorkbenchSession> {
  return invoke<WorkbenchSession>("get_workbench_session", { sessionId });
}

export function exportWorkbenchSession(sessionId: string): Promise<WorkbenchSession> {
  return invoke<WorkbenchSession>("export_workbench_session", { sessionId });
}

export function transitionWorkbenchSession(
  sessionId: string,
  action: WorkbenchSessionAction,
): Promise<WorkbenchSession> {
  return invoke<WorkbenchSession>("transition_workbench_session", {
    input: { sessionId, action },
  });
}

export function forkWorkbenchSession(
  sessionId: string,
  eventId: string,
): Promise<WorkbenchSession> {
  return invoke<WorkbenchSession>("fork_workbench_session", {
    input: { sessionId, eventId },
  });
}

export function prepareWorkbenchRunPlan(
  input: WorkbenchRunSpecInput,
): Promise<WorkbenchRunPlan> {
  return invoke<WorkbenchRunPlan>("prepare_workbench_run_plan", { input });
}

export function issueWorkbenchProcessStartGrant(
  input: WorkbenchProcessStartGrantInput,
): Promise<WorkbenchProcessStartGrantView> {
  return invoke<WorkbenchProcessStartGrantView>("issue_workbench_process_start_grant", { input });
}

export function listWorkbenchProcessStartGrants(
  sessionId: string,
): Promise<WorkbenchProcessStartGrantView[]> {
  return invoke<WorkbenchProcessStartGrantView[]>("list_workbench_process_start_grants", { sessionId });
}

export function revokeWorkbenchProcessStartGrant(
  grantId: string,
): Promise<WorkbenchProcessStartGrantView> {
  return invoke<WorkbenchProcessStartGrantView>("revoke_workbench_process_start_grant", { grantId });
}

export function admitWorkbenchProcess(
  input: WorkbenchProcessAdmissionInput,
): Promise<WorkbenchProcessAdmission> {
  return invoke<WorkbenchProcessAdmission>("admit_workbench_process", { input });
}

export function listWorkbenchProcessAdmissions(
  sessionId: string,
): Promise<WorkbenchProcessAdmission[]> {
  return invoke<WorkbenchProcessAdmission[]>("list_workbench_process_admissions", { sessionId });
}

export function deriveWorkbenchProcessAdmissionEligibility(
  runSpec: WorkbenchRunSpecInput,
): Promise<WorkbenchAdmissionEligibilitySnapshot> {
  return invoke<WorkbenchAdmissionEligibilitySnapshot>(
    "derive_workbench_process_admission_eligibility",
    { input: { runSpec } },
  );
}

export function getWorkbenchCapabilityProjection(): Promise<WorkbenchCapabilityProjection> {
  return invoke<WorkbenchCapabilityProjection>("get_workbench_capability_projection");
}
