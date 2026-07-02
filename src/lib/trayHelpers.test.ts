import { describe, expect, it } from "vitest";

import {
  activityFeedSignature,
  notificationActionTargetId,
  notificationActionView,
  safeNotificationActionView,
  safeTrayViewForMode,
  shouldShowCodexNudge,
} from "./trayHelpers";
import type {
  ActivityFeedResponse,
  ClientConnectorStatus,
  HeadroomPricingStatus,
} from "./types";

const emptySnapshot: ActivityFeedResponse = {
  proxyReachable: true,
  tiles: {
    transformation: null,
    record: null,
    rtkToday: null,
    learningsMilestone: null,
    weeklyRecap: null,
    trainSuggestion: null,
  },
};

const codexConnector: ClientConnectorStatus = {
  clientId: "codex",
  name: "Codex",
  installed: true,
  enabled: false,
  verified: false,
  lastConfiguredAt: null,
};

function pricing(optimizationAllowed: boolean): HeadroomPricingStatus {
  return { optimizationAllowed } as HeadroomPricingStatus;
}

describe("notificationActionView", () => {
  it("routes auth-related actions to upgradeAuth", () => {
    expect(notificationActionView("signin")).toBe("upgradeAuth");
    expect(notificationActionView("signup")).toBe("upgradeAuth");
    expect(notificationActionView("billing")).toBe("upgradeAuth");
  });

  it("routes runtime/connectors actions to settings", () => {
    expect(notificationActionView("runtime")).toBe("settings");
    expect(notificationActionView("connectors")).toBe("settings");
  });

  it("routes release and rollback evidence actions to settings", () => {
    expect(notificationActionView("release")).toBe("settings");
    expect(notificationActionView("release-readiness")).toBe("settings");
    expect(notificationActionView("rollback")).toBe("settings");
    expect(notificationActionView("rollback-center")).toBe("settings");
  });

  it("routes optimize/activity actions to their respective views", () => {
    expect(notificationActionView("optimize")).toBe("optimization");
    expect(notificationActionView("activity")).toBe("notifications");
  });

  it("returns null for unknown actions and explicit null", () => {
    expect(notificationActionView(null)).toBeNull();
    expect(notificationActionView("not-a-real-action")).toBeNull();
    expect(notificationActionView("")).toBeNull();
  });
});

describe("notificationActionTargetId", () => {
  it("maps release and rollback actions to settings card anchors", () => {
    expect(notificationActionTargetId("release")).toBe("release-readiness");
    expect(notificationActionTargetId("release-readiness")).toBe("release-readiness");
    expect(notificationActionTargetId("rollback")).toBe("rollback-center");
    expect(notificationActionTargetId("rollback-center")).toBe("rollback-center");
  });

  it("ignores actions without section targets", () => {
    expect(notificationActionTargetId("runtime")).toBeNull();
    expect(notificationActionTargetId(null)).toBeNull();
  });
});

describe("safeTrayViewForMode", () => {
  it("redirects upgrade views to home in local-only mode", () => {
    expect(safeTrayViewForMode("upgrade", true)).toBe("home");
    expect(safeTrayViewForMode("upgradeAuth", true)).toBe("home");
  });

  it("keeps local utility views in local-only mode", () => {
    expect(safeTrayViewForMode("home", true)).toBe("home");
    expect(safeTrayViewForMode("optimization", true)).toBe("optimization");
    expect(safeTrayViewForMode("repoIntelligence", true)).toBe(
      "repoIntelligence",
    );
    expect(safeTrayViewForMode("settings", true)).toBe("settings");
  });

  it("keeps upgrade views when remote services are enabled", () => {
    expect(safeTrayViewForMode("upgrade", false)).toBe("upgrade");
    expect(safeTrayViewForMode("upgradeAuth", false)).toBe("upgradeAuth");
  });
});

describe("safeNotificationActionView", () => {
  it("redirects auth notification actions to home in local-only mode", () => {
    expect(safeNotificationActionView("signin", true)).toBe("home");
    expect(safeNotificationActionView("signup", true)).toBe("home");
    expect(safeNotificationActionView("billing", true)).toBe("home");
  });

  it("keeps local notification actions in local-only mode", () => {
    expect(safeNotificationActionView("runtime", true)).toBe("settings");
    expect(safeNotificationActionView("release-readiness", true)).toBe("settings");
    expect(safeNotificationActionView("rollback-center", true)).toBe("settings");
    expect(safeNotificationActionView("activity", true)).toBe("notifications");
  });

  it("keeps auth notification actions when remote services are enabled", () => {
    expect(safeNotificationActionView("signin", false)).toBe("upgradeAuth");
  });
});

describe("shouldShowCodexNudge", () => {
  it("hides Codex nudge in local-only mode", () => {
    expect(shouldShowCodexNudge(codexConnector, null, false, true)).toBe(false);
  });

  it("shows Codex nudge for installed disabled Codex when remote pricing allows it", () => {
    expect(
      shouldShowCodexNudge(codexConnector, pricing(true), false, false),
    ).toBe(true);
  });

  it("hides Codex nudge when dismissed, gated, missing, or already enabled", () => {
    expect(
      shouldShowCodexNudge(codexConnector, pricing(true), true, false),
    ).toBe(false);
    expect(
      shouldShowCodexNudge(codexConnector, pricing(false), false, false),
    ).toBe(false);
    expect(shouldShowCodexNudge(null, pricing(true), false, false)).toBe(false);
    expect(
      shouldShowCodexNudge(
        { ...codexConnector, enabled: true },
        pricing(true),
        false,
        false,
      ),
    ).toBe(false);
  });
});

describe("activityFeedSignature", () => {
  it("returns a stable string for an empty snapshot", () => {
    const sig = activityFeedSignature(emptySnapshot);
    expect(sig).toBe("1|t:-|r:-|b:-|l:-|wr:-|ts:-");
  });

  it("differentiates proxyReachable false from proxyReachable true", () => {
    const offline = activityFeedSignature({
      ...emptySnapshot,
      proxyReachable: false,
    });
    const online = activityFeedSignature(emptySnapshot);
    expect(offline).not.toBe(online);
    expect(offline.startsWith("0|")).toBe(true);
    expect(online.startsWith("1|")).toBe(true);
  });

  it("changes when a tile slot's identifier flips", () => {
    const baseline = activityFeedSignature(emptySnapshot);
    const withTransform = activityFeedSignature({
      ...emptySnapshot,
      tiles: {
        ...emptySnapshot.tiles,
        transformation: {
          requestId: "req-123",
          timestamp: "2026-04-25T12:00:00Z",
          provider: "anthropic",
          model: "claude-opus-4-7",
          workspace: null,
          tokensSavedRaw: 1000,
          tokensSavedPercent: 12.5,
          estimatedCostSavingsUsd: 0.42,
          transforms: [],
          requestMessages: null,
          responseText: null,
          compressedMessages: null,
        } as never,
      },
    });
    expect(baseline).not.toBe(withTransform);
    expect(withTransform).toContain("t:req-123");
  });

  it("falls back to timestamp when transformation has no requestId", () => {
    const sig = activityFeedSignature({
      ...emptySnapshot,
      tiles: {
        ...emptySnapshot.tiles,
        transformation: {
          requestId: null,
          timestamp: "2026-04-25T12:00:00Z",
          provider: "anthropic",
          model: null,
          workspace: null,
          tokensSavedRaw: 0,
          tokensSavedPercent: 0,
          estimatedCostSavingsUsd: 0,
          transforms: [],
          requestMessages: null,
          responseText: null,
          compressedMessages: null,
        } as never,
      },
    });
    expect(sig).toContain("t:2026-04-25T12:00:00Z");
  });

  it("encodes record, rtkToday, learningsMilestone, weeklyRecap, trainSuggestion slots", () => {
    const sig = activityFeedSignature({
      proxyReachable: true,
      tiles: {
        transformation: null,
        record: { observedAt: "2026-04-25T11:00:00Z" } as never,
        rtkToday: { date: "2026-04-25", savedTokens: 1234 } as never,
        learningsMilestone: { observedAt: "2026-04-25T10:00:00Z" } as never,
        weeklyRecap: { weekStart: "2026-04-20" } as never,
        trainSuggestion: {
          projectPath: "/Users/x/proj",
          observedAt: "2026-04-25T09:00:00Z",
        } as never,
      },
    });
    expect(sig).toContain("r:2026-04-25T11:00:00Z");
    expect(sig).toContain("b:2026-04-25:1234");
    expect(sig).toContain("l:2026-04-25T10:00:00Z");
    expect(sig).toContain("wr:2026-04-20");
    expect(sig).toContain("ts:/Users/x/proj:2026-04-25T09:00:00Z");
  });
});
