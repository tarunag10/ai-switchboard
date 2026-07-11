import { describe, expect, it } from "vitest";
import { gatewayProfileStatus, gatewayProfiles } from "./gatewayProfiles";

describe("gatewayProfiles", () => {
  it("declares boundaries, disclosure, Doctor evidence, and cleanup for every profile", () => {
    expect(gatewayProfiles).toHaveLength(4);
    for (const profile of gatewayProfiles) {
      expect(profile.disclosure).not.toHaveLength(0);
      expect(profile.privacyCaveat).not.toHaveLength(0);
      expect(profile.requiredEvidence.length).toBeGreaterThan(0);
      expect(profile.doctorChecks.length).toBeGreaterThan(0);
      expect(profile.rollbackGuidance).not.toHaveLength(0);
      expect(profile.offModeGuidance).not.toHaveLength(0);
    }
  });

  it("keeps remote routing profiles explicitly disclosed and non-managed", () => {
    const remote = gatewayProfiles.filter((profile) => profile.trafficBoundary === "remote");
    expect(remote.map((profile) => profile.id)).toEqual([
      "langfuse-export",
      "cloudflare-ai-gateway",
      "kong-enterprise-gateway",
    ]);
    expect(remote.every((profile) => profile.disclosure.length > 30)).toBe(true);
    expect(remote.every((profile) => profile.state === "guided" || profile.state === "gated")).toBe(true);
  });

  it("marks guided and gated profiles without implying setup was applied", () => {
    expect(gatewayProfileStatus(gatewayProfiles[0])).toMatchObject({
      label: "Guided setup",
      actionable: true,
    });
    expect(gatewayProfileStatus(gatewayProfiles[3])).toMatchObject({
      label: "Gated",
      actionable: false,
    });
  });
});
