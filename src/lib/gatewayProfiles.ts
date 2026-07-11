export type GatewayProfileId =
  | "litellm-local-cache"
  | "langfuse-export"
  | "cloudflare-ai-gateway"
  | "kong-enterprise-gateway";

export type GatewayProfileCategory =
  | "local cache"
  | "observability"
  | "remote gateway"
  | "enterprise gateway";

export type GatewayProfileState = "guided" | "gated";
export type GatewayLifecycleState = "disabled" | "enabled";
export type GatewayTrafficBoundary = "local" | "remote";
export type GatewaySavingsEvidence = "estimated" | "external" | "none";

export interface GatewayDoctorCheck {
  label: string;
  evidence: string;
}

export interface GatewayProfile {
  id: GatewayProfileId;
  name: string;
  category: GatewayProfileCategory;
  state: GatewayProfileState;
  trafficBoundary: GatewayTrafficBoundary;
  canSeePromptsAndOutputs: boolean;
  canModifyProviderRouting: boolean;
  needsSecrets: boolean;
  supportedClients: string[];
  disclosure: string;
  privacyCaveat: string;
  requiredEvidence: string[];
  doctorChecks: GatewayDoctorCheck[];
  rollbackGuidance: string;
  offModeGuidance: string;
  savingsEvidence: GatewaySavingsEvidence;
  setupGuidance: string;
}

export interface GatewayProfileStatus {
  profileId: GatewayProfileId;
  label: "Guided setup" | "Gated";
  detail: string;
  actionable: boolean;
}

export interface GatewayProfileReceipt {
  id: string;
  profileId: GatewayProfileId;
  action: "enabled" | "disabled" | "evidence-reviewed";
  createdAt: string;
  detail: string;
}

export interface GatewayProfileLocalState {
  version: 1;
  profiles: Partial<Record<GatewayProfileId, GatewayLifecycleState>>;
  reviewedChecks: Partial<Record<GatewayProfileId, string[]>>;
  receipts: GatewayProfileReceipt[];
}

export const gatewayProfileStorageKey = "ai-switchboard.gateway-profiles.v1";

export function emptyGatewayProfileLocalState(): GatewayProfileLocalState {
  return { version: 1, profiles: {}, reviewedChecks: {}, receipts: [] };
}

export function parseGatewayProfileLocalState(value: string | null): GatewayProfileLocalState {
  if (!value) return emptyGatewayProfileLocalState();
  try {
    const parsed = JSON.parse(value) as Partial<GatewayProfileLocalState>;
    if (parsed.version !== 1 || typeof parsed.profiles !== "object" || !parsed.profiles) {
      return emptyGatewayProfileLocalState();
    }
    return {
      version: 1,
      profiles: parsed.profiles,
      reviewedChecks: parsed.reviewedChecks && typeof parsed.reviewedChecks === "object" ? parsed.reviewedChecks : {},
      receipts: Array.isArray(parsed.receipts) ? parsed.receipts.slice(0, 30) : [],
    };
  } catch {
    return emptyGatewayProfileLocalState();
  }
}

export function gatewayProfileConfigPreview(profile: GatewayProfile): string {
  const label = profile.name.toUpperCase().replace(/[^A-Z0-9]+/g, "_");
  return [
    `# Switchboard preview only — not applied`,
    `# ${profile.name}: ${profile.trafficBoundary} boundary`,
    `# Keep actual URLs and secrets in your secure environment.`,
    `${label}_ENABLED=1`,
    profile.canModifyProviderRouting ? `${label}_BASE_URL=<set-outside-switchboard>` : `# No provider routing is changed by this profile.`,
    profile.needsSecrets ? `${label}_API_KEY=<set-outside-switchboard>` : "# No profile secret is required by Switchboard.",
    "# Rollback: set ENABLED=0 and remove these manual variables.",
  ].join("\n");
}

export function gatewayDoctorSummary(profile: GatewayProfile, state: GatewayProfileLocalState): string {
  const reviewed = state.reviewedChecks[profile.id]?.length ?? 0;
  return `${reviewed}/${profile.doctorChecks.length} evidence items reviewed locally; no endpoint, credential, or live health check was run.`;
}

export const gatewayProfiles: readonly GatewayProfile[] = [
  {
    id: "litellm-local-cache",
    name: "Semantic Cache",
    category: "local cache",
    state: "guided",
    trafficBoundary: "local",
    canSeePromptsAndOutputs: true,
    canModifyProviderRouting: false,
    needsSecrets: false,
    supportedClients: ["Claude Code", "Codex", "Gemini CLI", "OpenCode"],
    disclosure:
      "Local-only guided setup. Switchboard does not start LiteLLM, write provider configuration, or reroute traffic.",
    privacyCaveat:
      "A local semantic cache can retain prompt and response-derived data. Disable it for sensitive or rapidly changing work.",
    requiredEvidence: [
      "A local LiteLLM proxy is running.",
      "A cache backend is configured.",
      "Cache-hit evidence is available before savings are attributed.",
    ],
    doctorChecks: [
      { label: "Local proxy", evidence: "Confirm the local LiteLLM port responds." },
      { label: "Cache backend", evidence: "Confirm the backend is configured separately." },
      { label: "Cache hits", evidence: "Record hits as estimated or external savings only." },
    ],
    rollbackGuidance: "Remove the LiteLLM environment variables or local proxy target you added manually.",
    offModeGuidance: "Stop the local proxy and remove its manual environment variables; Switchboard owns no config block.",
    savingsEvidence: "estimated",
    setupGuidance: `# Semantic Cache (manual, local-only)\n# Start/configure LiteLLM and its cache outside Switchboard.\n# Point an agent at your local proxy only after testing it.\n# Keep secrets in your shell or secure store, never in this repository.\n\n# Doctor evidence to collect\n# - local proxy endpoint responds\n# - cache backend configured\n# - cache hits observed before claiming savings`,
  },
  {
    id: "langfuse-export",
    name: "Self-hosted Langfuse",
    category: "observability",
    state: "guided",
    trafficBoundary: "remote",
    canSeePromptsAndOutputs: true,
    canModifyProviderRouting: false,
    needsSecrets: true,
    supportedClients: ["Any manually instrumented client"],
    disclosure:
      "Trace export is opt-in. A Langfuse endpoint can receive prompts, outputs, metadata, model names, and timing.",
    privacyCaveat:
      "Use a self-hosted endpoint you control and confirm its retention and access policy before sending traces.",
    requiredEvidence: [
      "Endpoint ownership and retention policy reviewed.",
      "Authentication succeeds without storing keys in repository files.",
      "A test trace is accepted before enabling real exports.",
    ],
    doctorChecks: [
      { label: "Endpoint", evidence: "Confirm the self-hosted endpoint is reachable." },
      { label: "Authentication", evidence: "Confirm auth without revealing secret values." },
      { label: "Test trace", evidence: "Confirm one explicit test trace is accepted." },
    ],
    rollbackGuidance: "Remove your instrumentation/export environment variables and revoke the export key if needed.",
    offModeGuidance: "Disable export at the instrumented client; no traces should be sent while disabled.",
    savingsEvidence: "none",
    setupGuidance: `# Self-hosted Langfuse (manual, opt-in)\n# Set endpoint and keys in your secure environment, not in repo files.\n# Send one explicit test trace before enabling production exports.\n\n# Doctor evidence to collect\n# - endpoint reachable\n# - auth accepted\n# - test trace accepted`,
  },
  {
    id: "cloudflare-ai-gateway",
    name: "Cloudflare AI Gateway",
    category: "remote gateway",
    state: "guided",
    trafficBoundary: "remote",
    canSeePromptsAndOutputs: true,
    canModifyProviderRouting: true,
    needsSecrets: true,
    supportedClients: ["Clients with a manual base URL setting"],
    disclosure:
      "Remote routing: requests pass through Cloudflare before the upstream provider. Prompts and outputs may be visible to that gateway depending on your configuration.",
    privacyCaveat:
      "Review Cloudflare account, retention, caching, and access controls before routing sensitive work through it.",
    requiredEvidence: [
      "Gateway URL and account boundary reviewed.",
      "Authentication is present without exposing the token value.",
      "A harmless passthrough request succeeds before real traffic is routed.",
    ],
    doctorChecks: [
      { label: "Gateway URL", evidence: "Confirm the configured endpoint is reachable." },
      { label: "Authentication", evidence: "Confirm auth exists without printing its value." },
      { label: "Passthrough", evidence: "Confirm a harmless provider request succeeds." },
    ],
    rollbackGuidance: "Restore the client base URL directly to its provider endpoint and revoke the gateway token if needed.",
    offModeGuidance: "Remove the manually added gateway URL and token from the client environment; Switchboard writes nothing.",
    savingsEvidence: "external",
    setupGuidance: `# Cloudflare AI Gateway (manual, remote routing)\n# Configure a client base URL and token outside Switchboard.\n# Test a harmless request before routing real work.\n# Never commit the gateway token or URL with embedded credentials.\n\n# Doctor evidence to collect\n# - endpoint reachable\n# - auth present\n# - harmless passthrough succeeds`,
  },
  {
    id: "kong-enterprise-gateway",
    name: "Kong Enterprise Gateway",
    category: "enterprise gateway",
    state: "gated",
    trafficBoundary: "remote",
    canSeePromptsAndOutputs: true,
    canModifyProviderRouting: true,
    needsSecrets: true,
    supportedClients: ["Enterprise-managed clients"],
    disclosure:
      "Enterprise routing is not managed by Switchboard. A Kong deployment may process prompts and outputs before they reach a provider.",
    privacyCaveat:
      "Deployment topology, retention, access controls, and rollback ownership must be agreed with the enterprise gateway operator.",
    requiredEvidence: [
      "Traffic boundary and data policy approved by the gateway owner.",
      "A reversible client routing change is documented.",
      "Health checks work without enterprise-only assumptions.",
    ],
    doctorChecks: [
      { label: "Deployment owner", evidence: "Identify the enterprise operator and support path." },
      { label: "Routing rollback", evidence: "Document how clients return to direct provider routing." },
      { label: "Health evidence", evidence: "Collect an operator-approved health signal." },
    ],
    rollbackGuidance: "Use the enterprise change process to restore direct routing; Switchboard has no managed lifecycle for Kong.",
    offModeGuidance: "Follow the enterprise runbook to remove client gateway routing and revoke access as required.",
    savingsEvidence: "none",
    setupGuidance: `# Kong Enterprise Gateway (gated)\n# Use your enterprise gateway runbook. Switchboard does not install, configure,\n# or route Kong traffic. Document owner, rollback, and approved health evidence\n# before enabling a client-side route.`,
  },
] as const;

export function getGatewayProfile(id: GatewayProfileId): GatewayProfile | null {
  return gatewayProfiles.find((profile) => profile.id === id) ?? null;
}

export function gatewayProfileStatus(profile: GatewayProfile): GatewayProfileStatus {
  return profile.state === "gated"
    ? {
        profileId: profile.id,
        label: "Gated",
        detail: "Documentation and evidence only; no install, configuration, or traffic change is available here.",
        actionable: false,
      }
    : {
        profileId: profile.id,
        label: "Guided setup",
        detail: "Copy the manual guide, collect Doctor evidence, and make any routing change outside Switchboard.",
        actionable: true,
      };
}
