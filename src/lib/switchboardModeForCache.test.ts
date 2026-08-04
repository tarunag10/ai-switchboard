import { describe, expect, it } from "vitest";

import { resolveSwitchboardModeForCache } from "./switchboardModeForCache";
import type { RuntimeStatus } from "./types";

const baseRuntime = {
  running: true,
  proxyReachable: true,
} as RuntimeStatus;

describe("resolveSwitchboardModeForCache", () => {
  it("returns off when runtime is not running", () => {
    expect(resolveSwitchboardModeForCache({ ...baseRuntime, running: false })).toBe(
      "off",
    );
  });

  it("returns rtk when RTK is enabled without proxy reachability", () => {
    expect(
      resolveSwitchboardModeForCache({
        ...baseRuntime,
        proxyReachable: false,
        rtk: { enabled: true },
      } as RuntimeStatus),
    ).toBe("rtk");
  });

  it("returns headroom when proxy is reachable with RTK enabled", () => {
    expect(
      resolveSwitchboardModeForCache({
        ...baseRuntime,
        rtk: { enabled: true },
      } as RuntimeStatus),
    ).toBe("headroom");
  });

  it("returns full when proxy is reachable without RTK", () => {
    expect(
      resolveSwitchboardModeForCache({
        ...baseRuntime,
        rtk: { enabled: false },
      } as RuntimeStatus),
    ).toBe("full");
  });
});
