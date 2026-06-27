import { describe, expect, it } from "vitest";
import {
  allowedRemoteDestinations,
  blockedLocalOnlyDestinations,
  localOnlySetupLabel,
  remoteServiceDestinations,
  remoteServicesCopy,
} from "./remoteServices";

describe("remote services copy", () => {
  it("labels local-only remote services fully off", () => {
    expect(remoteServicesCopy(false)).toEqual({
      label: "Off",
      detail:
        "Blocked in local-only mode: Headroom account API, Headroom pricing and trial API, Sentry diagnostics, Microsoft Clarity analytics, Aptabase analytics, Tauri update feed, External support links.",
    });
  });

  it("labels remote services available only when enabled", () => {
    expect(remoteServicesCopy(true)).toEqual({
      label: "Available",
      detail:
        "Account, pricing, update, support, and optional telemetry destinations are enabled.",
    });
  });

  it("keeps local-only blocked destination registry explicit", () => {
    expect(remoteServiceDestinations.map((destination) => destination.id)).toEqual([
      "headroom_account_api",
      "headroom_pricing_api",
      "sentry",
      "clarity",
      "aptabase",
      "tauri_updater",
      "support_links",
    ]);
    expect(blockedLocalOnlyDestinations()).toHaveLength(
      remoteServiceDestinations.length,
    );
    expect(allowedRemoteDestinations(true)).toEqual([]);
  });

  it("documents env and endpoint evidence for every remote destination", () => {
    expect(
      remoteServiceDestinations.every(
        (destination) =>
          destination.endpointExample.length > 0 && destination.source.length > 0,
      ),
    ).toBe(true);
    expect(
      remoteServiceDestinations
        .filter((destination) => destination.kind !== "support")
        .every((destination) => Boolean(destination.envVar)),
    ).toBe(true);
  });

  it("uses explicit setup labels for local-only and cloud-capable modes", () => {
    expect(localOnlySetupLabel(true)).toBe("Local-only Mac setup");
    expect(localOnlySetupLabel(false)).toBe("Headroom cloud setup");
  });
});
