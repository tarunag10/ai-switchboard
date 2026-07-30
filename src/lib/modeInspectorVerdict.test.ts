import { describe, expect, it } from "vitest";

import { deriveModeInspectorVerdict } from "./modeInspectorVerdict";

describe("deriveModeInspectorVerdict", () => {
  it("returns aligned when modes and proxy evidence match", () => {
    const result = deriveModeInspectorVerdict({
      requestedMode: "Full optimization",
      activeMode: "Full optimization",
      proxyStatus: "Running",
      proxyAuthStatus: "session_token_available",
      rows: [{ id: "claude", label: "Claude", status: "Routed" }],
    });
    expect(result.verdict).toBe("aligned");
  });

  it("returns blocked when requested and active modes differ", () => {
    const result = deriveModeInspectorVerdict({
      requestedMode: "Off",
      activeMode: "Full optimization",
      proxyStatus: "Running",
      rows: [],
    });
    expect(result.verdict).toBe("blocked");
  });

  it("returns attention for stale shells or enforced-but-unvalidated proxy auth", () => {
    const stale = deriveModeInspectorVerdict({
      requestedMode: "Full optimization",
      activeMode: "Full optimization",
      proxyStatus: "Running",
      staleShellWarning: true,
      rows: [],
    });
    expect(stale.verdict).toBe("attention");

    const auth = deriveModeInspectorVerdict({
      requestedMode: "Full optimization",
      activeMode: "Full optimization",
      proxyStatus: "Running",
      proxyAuthStatus: "session_token_enforced",
      rows: [],
    });
    expect(auth.verdict).toBe("attention");
  });
});
