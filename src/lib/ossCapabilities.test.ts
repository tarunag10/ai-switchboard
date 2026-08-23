import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { loadOssCapabilityRegistry } from "./ossCapabilities";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("OSS capability registry", () => {
  it("loads the metadata-only fail-closed native registry", async () => {
    const registry = {
      schemaVersion: 1,
      registryMode: "metadata_only",
      writesEnabled: false,
      approvalMode: "fail_closed",
      providers: [],
      tools: [],
    } as const;
    vi.mocked(invoke).mockResolvedValueOnce(registry);
    await expect(loadOssCapabilityRegistry()).resolves.toEqual(registry);
    expect(invoke).toHaveBeenCalledWith("get_oss_capability_registry");
  });
});
