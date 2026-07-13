import { describe, expect, it } from "vitest";
import {
  emptyGatewayProfileLocalState,
  gatewayDoctorSummary,
  gatewayProfileGovernanceIssues,
  gatewayProfileGovernanceIssuesFor,
  gatewayProfileConfigPreview,
  gatewayProfileLifecycleIssues,
  gatewayProfileLifecycleSummary,
  gatewayProfileStatus,
  gatewayProfiles,
  gatewayReadinessSummary,
  parseGatewayProfileLocalState,
  type GatewayProfile,
} from "./gatewayProfiles";

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

  it("passes the trust-boundary governance contract for the shipped registry", () => {
    expect(gatewayProfileGovernanceIssuesFor(gatewayProfiles)).toEqual([]);
  });

  it("rejects unsafe profile metadata without contacting a gateway", () => {
    const invalid = {
      ...gatewayProfiles[2],
      id: "unsafe-profile",
      trafficBoundary: "remote" as const,
      disclosure: "",
      setupGuidance: "Configure manually",
      rollbackGuidance: "Do the thing",
      supportedClients: [],
      doctorChecks: [{ label: "", evidence: "" }],
    } as unknown as GatewayProfile;
    const issues = gatewayProfileGovernanceIssues(invalid);
    expect(issues).toEqual(expect.arrayContaining([
      "unsafe-profile: disclosure must not be empty",
      "unsafe-profile: supportedClients must contain at least one client",
      "unsafe-profile: every Doctor check needs a label and evidence description",
      "unsafe-profile: remote profiles must disclose the remote trust boundary",
      "unsafe-profile: routing profiles must document how routing is restored",
    ]));
    expect(issues.some((issue) => issue.includes("secret handling"))).toBe(true);
  });

  it("rejects incomplete lifecycle evidence and keeps guided profiles non-automated", () => {
    const invalid = {
      ...gatewayProfiles[0],
      lifecycle: {
        automationEnabled: true,
        stages: gatewayProfiles[0].lifecycle.stages.slice(0, 2),
      },
    } as GatewayProfile;
    const issues = gatewayProfileLifecycleIssues(invalid);
    expect(issues).toEqual(expect.arrayContaining([
      "litellm-local-cache: lifecycle must declare exactly 7 stages",
      "litellm-local-cache: non-managed profiles may not enable lifecycle automation",
    ]));
    expect(gatewayProfileLifecycleSummary(gatewayProfiles[0])).toContain(
      "automation is gated at apply",
    );
  });

  it("flags duplicate profile ids in a registry", () => {
    expect(gatewayProfileGovernanceIssuesFor([gatewayProfiles[0], gatewayProfiles[0]])).toContain(
      "litellm-local-cache: duplicate profile id",
    );
  });

  it("renders redacted readiness without promoting local evidence to live status", () => {
    expect(gatewayReadinessSummary({
      profileId: "litellm-local-cache",
      configuration: [{ label: "URL", environmentVariable: "LITELLM_BASE_URL", present: true }],
      credentials: [{ label: "Key", environmentVariable: "LITELLM_API_KEY", present: false }],
      connectivity: { attempted: false, status: "not-run", detail: "No connectivity preflight was run." },
      live: false,
      guidance: "Advisory only.",
    })).toContain("values redacted");
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

  it("creates content-free preview and local Doctor evidence state", () => {
    const profile = gatewayProfiles[2];
    expect(gatewayProfileConfigPreview(profile)).toContain("not applied");
    expect(gatewayProfileConfigPreview(profile)).toContain("<set-outside-switchboard>");
    expect(gatewayDoctorSummary(profile, emptyGatewayProfileLocalState())).toContain("no endpoint");
    expect(parseGatewayProfileLocalState("not-json")).toEqual(emptyGatewayProfileLocalState());
  });
});
