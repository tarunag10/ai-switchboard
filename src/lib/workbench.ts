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

export interface WorkbenchRunSpecInput {
  sessionId: string;
  adapterId: "claude_code" | "codex" | "gemini_cli";
  workspaceDigest: string;
  contextPackDigest: string | null;
  routerDecisionId: string;
  replayReferenceId: string | null;
  requiredCapabilityIds: string[];
  requestedMode: SwitchboardMode;
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
  requestedMode: SwitchboardMode;
  adapterPlanId: string;
  adapterAction: "apply_managed_routing" | "cleanup_managed_routing";
  adapterReversible: boolean;
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

export function getWorkbenchCapabilityProjection(): Promise<WorkbenchCapabilityProjection> {
  return invoke<WorkbenchCapabilityProjection>("get_workbench_capability_projection");
}
