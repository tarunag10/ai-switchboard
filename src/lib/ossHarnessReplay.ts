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

export function replayRedactedRouteEvents(path: string): Promise<OssHarnessReplayResult> {
  return invoke<OssHarnessReplayResult>("replay_redacted_route_events", { path });
}
