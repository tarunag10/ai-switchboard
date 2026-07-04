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
      label: "Local-only",
      detail:
        "AI Switchboard local-only mode is on. Diagnostics, analytics, update, and support endpoints stay paused: Sentry diagnostics, Microsoft Clarity analytics, Product analytics, Tauri update feed, External support links. Account and paid pricing APIs are not part of this app.",
    });
  });

  it("labels remote services available only when enabled", () => {
    expect(remoteServicesCopy(true)).toEqual({
      label: "Available",
      detail:
        "Update, support, and optional telemetry destinations are enabled. Account and paid pricing APIs are not part of this app.",
    });
  });

  it("keeps local-only blocked destination registry explicit", () => {
    expect(
      remoteServiceDestinations.map((destination) => destination.id),
    ).toEqual([
      "sentry",
      "clarity",
      "product_analytics",
      "tauri_updater",
      "support_links",
    ]);
    expect(blockedLocalOnlyDestinations()).toHaveLength(
      remoteServiceDestinations.length,
    );
    expect(allowedRemoteDestinations(true)).toEqual([]);
  });

  it("documents endpoint evidence for every remote destination without bundling operator env names", () => {
    expect(
      remoteServiceDestinations.every(
        (destination) =>
          destination.endpointExample.length > 0 &&
          destination.source.length > 0,
      ),
    ).toBe(true);
    expect(
      remoteServiceDestinations.every(
        (destination) =>
          !destination.id.includes("account") &&
          !destination.id.includes("pricing"),
      ),
    ).toBe(true);
  });

  it("uses explicit setup labels for local-only and cloud-capable modes", () => {
    expect(localOnlySetupLabel(true)).toBe("Local-only Mac setup");
    expect(localOnlySetupLabel(false)).toBe("AI Switchboard cloud setup");
  });
});
