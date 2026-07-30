import { describe, expect, it } from "vitest";

import { deriveRepoMemoryMcpSupervisionSummary } from "./repoMemoryMcpSupervision";

describe("deriveRepoMemoryMcpSupervisionSummary", () => {
  it("reports relaunch verification success", () => {
    const result = deriveRepoMemoryMcpSupervisionSummary({
      supervisionStatus: "verified_active",
      relaunchSurvivalStatus: "verified",
      supervisionScope: "relaunch_verified",
      active: true,
    });
    expect(result.tone).toBe("success");
    expect(result.summary).toContain("survived app relaunch");
  });

  it("warns when supervision is app-session scoped", () => {
    const result = deriveRepoMemoryMcpSupervisionSummary({
      supervisionStatus: "verified_active",
      relaunchSurvivalStatus: "not_applicable",
      supervisionScope: "app_session",
      active: true,
    });
    expect(result.tone).toBe("warning");
    expect(result.summary).toContain("app session only");
  });
});
