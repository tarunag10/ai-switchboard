import { describe, expect, it, vi } from "vitest";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import {
  listOssHarnessReplayReferences,
  replayRedactedRouteEvents,
  resolveOssHarnessReplayReference,
} from "./ossHarnessReplay";

describe("OSS harness replay bridge", () => {
  it("uses scoped native validation and receipt commands", async () => {
    invoke.mockResolvedValue({});
    await replayRedactedRouteEvents("/tmp/redacted-replay.json");
    await listOssHarnessReplayReferences();
    await resolveOssHarnessReplayReference("replay-reference-00000000-0000-4000-8000-000000000001");

    expect(invoke).toHaveBeenNthCalledWith(1, "replay_redacted_route_events", {
      path: "/tmp/redacted-replay.json",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "list_oss_harness_replay_references");
    expect(invoke).toHaveBeenNthCalledWith(3, "resolve_oss_harness_replay_reference", {
      replayId: "replay-reference-00000000-0000-4000-8000-000000000001",
    });
  });
});
