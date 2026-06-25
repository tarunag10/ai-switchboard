import { describe, expect, it } from "vitest";

import { localOnlySetupLabel, remoteServicesCopy } from "./remoteServices";

describe("remote services copy", () => {
  it("labels local-only remote services as fully off", () => {
    expect(remoteServicesCopy(false)).toEqual({
      label: "Off",
      detail: "No pricing, trial, Clarity, Sentry, or Aptabase calls.",
    });
  });

  it("labels remote services as available only when enabled", () => {
    expect(remoteServicesCopy(true)).toEqual({
      label: "Available",
      detail: "Account features and optional remote telemetry are enabled.",
    });
  });

  it("uses explicit setup labels for local-only and cloud-capable modes", () => {
    expect(localOnlySetupLabel(true)).toBe("Local-only Mac setup");
    expect(localOnlySetupLabel(false)).toBe("Headroom cloud setup");
  });
});
