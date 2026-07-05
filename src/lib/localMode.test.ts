import { afterEach, describe, expect, it, vi } from "vitest";

describe("local mode", () => {
  afterEach(() => {
    vi.resetModules();
    vi.unstubAllEnvs();
  });

  it("defaults to local-only for free builds", async () => {
    const { localOnlyModeEnabled } = await import("./localMode");

    expect(localOnlyModeEnabled()).toBe(true);
  });

  it("allows explicit remote-service opt in for forks", async () => {
    vi.stubEnv("VITE_HEADROOM_BUILD_FLAVOR", "operator");
    vi.stubEnv("VITE_HEADROOM_LOCAL_ONLY", "0");
    vi.stubEnv("VITE_HEADROOM_REMOTE_SERVICES", "1");
    const { localOnlyModeEnabled } = await import("./localMode");

    expect(localOnlyModeEnabled()).toBe(false);
  });

  it("treats the local-free build flavor as local-only", async () => {
    vi.stubEnv("VITE_HEADROOM_BUILD_FLAVOR", "local-free");
    vi.stubEnv("VITE_HEADROOM_REMOTE_SERVICES", "1");
    const { localOnlyModeEnabled } = await import("./localMode");

    expect(localOnlyModeEnabled()).toBe(true);
  });
});
