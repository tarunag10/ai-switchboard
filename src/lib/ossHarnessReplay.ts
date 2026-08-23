import { invoke } from "@tauri-apps/api/core";

export interface OssHarnessReplayResult {
  schemaVersion: number;
  replayMode: "redacted_observe_only";
  automaticPromotion: "disabled";
  providerTraffic: "none";
  eventCount: number;
  routeCounts: Record<string, number>;
  outcomeCounts: Record<string, number>;
  latency: { sampleCount: number; p95Ms: number | null };
  replayDigest: string;
}

export interface OssHarnessReplayReference {
  schemaVersion: 1;
  replayId: string;
  validatedAt: string;
  replayMode: "redacted_observe_only";
  automaticPromotion: "disabled";
  providerTraffic: "none";
  eventCount: number;
  replayDigest: string;
  receiptDigest: string;
}

export interface OssHarnessReplayValidation {
  result: OssHarnessReplayResult;
  reference: OssHarnessReplayReference;
}

export function replayRedactedRouteEvents(path: string): Promise<OssHarnessReplayValidation> {
  return invoke<OssHarnessReplayValidation>("replay_redacted_route_events", { path });
}

export function listOssHarnessReplayReferences(): Promise<OssHarnessReplayReference[]> {
  return invoke<OssHarnessReplayReference[]>("list_oss_harness_replay_references");
}

export function resolveOssHarnessReplayReference(
  replayId: string,
): Promise<OssHarnessReplayReference> {
  return invoke<OssHarnessReplayReference>("resolve_oss_harness_replay_reference", { replayId });
}
