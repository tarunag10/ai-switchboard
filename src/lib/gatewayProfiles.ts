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

/**
 * Profile lifecycle state. The registry currently ships only guided/gated
 * add-ons; the additional states keep the contract explicit for a future
 * managed integration without making a guided profile look live.
 */
export type GatewayProfileState =
  | "managed"
  | "guided"
  | "detected"
  | "gated"
  | "unsupported";
export type GatewayLifecycleState = "disabled" | "enabled";
export type GatewayTrafficBoundary = "local" | "remote";
export type GatewaySavingsEvidence = "estimated" | "external" | "none";

export type GatewayLifecycleStageId =
  | "detect"
  | "preview"
  | "backup"
  | "apply"
  | "verify"
  | "rollback"
  | "offCleanup";

export type GatewayLifecycleStageState = "available" | "guided" | "blocked";

export interface GatewayLifecycleStage {
  id: GatewayLifecycleStageId;
  label: string;
  state: GatewayLifecycleStageState;
  evidence: string;
}

export interface GatewayLifecycleContract {
  automationEnabled: boolean;
  stages: GatewayLifecycleStage[];
}

export const gatewayLifecycleStageOrder: GatewayLifecycleStageId[] = [
  "detect",
  "preview",
  "backup",
  "apply",
  "verify",
  "rollback",
  "offCleanup",
];

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
  /** Explicit lifecycle evidence; guided profiles never imply live setup. */
  lifecycle: GatewayLifecycleContract;
}

export function gatewayProfileLifecycleIssues(profile: GatewayProfile): string[] {
  const issues: string[] = [];
  const prefix = `${profile.id || "unknown"}:`;
  const lifecycle = profile.lifecycle;
  if (!lifecycle || !Array.isArray(lifecycle.stages)) {
    return [`${prefix} lifecycle stages must be declared`];
  }

  const ids = lifecycle.stages.map((stage) => stage.id);
  if (ids.length !== gatewayLifecycleStageOrder.length) {
    issues.push(`${prefix} lifecycle must declare exactly ${gatewayLifecycleStageOrder.length} stages`);
  }
  if (ids.some((id, index) => id !== gatewayLifecycleStageOrder[index])) {
    issues.push(`${prefix} lifecycle stages must follow detect, preview, backup, apply, verify, rollback, offCleanup order`);
  }
  if (new Set(ids).size !== ids.length) {
    issues.push(`${prefix} lifecycle stages must not contain duplicate ids`);
  }
  for (const stage of lifecycle.stages) {
    if (!stage.label.trim() || !stage.evidence.trim()) {
      issues.push(`${prefix} every lifecycle stage needs a label and evidence description`);
    }
  }
  const hasBlockedStage = lifecycle.stages.some((stage) => stage.state !== "available");
  if (lifecycle.automationEnabled && hasBlockedStage) {
    issues.push(`${prefix} automationEnabled requires every lifecycle stage to be available`);
  }
  if (profile.state === "managed" && !lifecycle.automationEnabled) {
    issues.push(`${prefix} managed profiles must have automationEnabled lifecycle evidence`);
  }
  if (profile.state !== "managed" && lifecycle.automationEnabled) {
    issues.push(`${prefix} non-managed profiles may not enable lifecycle automation`);
  }
  return issues;
}

/**
 * Validate the trust-boundary contract before a profile is rendered or
 * included in release evidence. Keeping this as a pure read-model check makes
 * it safe to run in the browser and in tests without touching credentials or
 * gateway endpoints.
 */
export function gatewayProfileGovernanceIssues(profile: GatewayProfile): string[] {
  const issues: string[] = [];
  const prefix = `${profile.id || "unknown"}:`;
  const nonEmpty = (value: string, field: string) => {
    if (!value.trim()) issues.push(`${prefix} ${field} must not be empty`);
  };

  nonEmpty(profile.name, "name");
  nonEmpty(profile.disclosure, "disclosure");
  nonEmpty(profile.privacyCaveat, "privacyCaveat");
  nonEmpty(profile.rollbackGuidance, "rollbackGuidance");
  nonEmpty(profile.offModeGuidance, "offModeGuidance");
  nonEmpty(profile.setupGuidance, "setupGuidance");

  if (!profile.supportedClients.length) {
    issues.push(`${prefix} supportedClients must contain at least one client`);
  }
  if (!profile.requiredEvidence.length) {
    issues.push(`${prefix} requiredEvidence must contain at least one item`);
  }
  if (!profile.doctorChecks.length) {
    issues.push(`${prefix} doctorChecks must contain at least one item`);
  }
  if (profile.doctorChecks.some((check) => !check.label.trim() || !check.evidence.trim())) {
    issues.push(`${prefix} every Doctor check needs a label and evidence description`);
  }
  if (profile.trafficBoundary === "remote" && !/remote|gateway|endpoint|trace|export/i.test(profile.disclosure)) {
    issues.push(`${prefix} remote profiles must disclose the remote trust boundary`);
  }
  if (profile.trafficBoundary === "local" && profile.canModifyProviderRouting) {
    issues.push(`${prefix} local profiles may not modify provider routing`);
  }
  if (profile.needsSecrets && !/secret|token|key|secure|credential/i.test(profile.setupGuidance)) {
    issues.push(`${prefix} secret-bearing profiles must explain secure secret handling`);
  }
  if (profile.canModifyProviderRouting && !/route|routing|config|url|gateway/i.test(profile.rollbackGuidance)) {
    issues.push(`${prefix} routing profiles must document how routing is restored`);
  }

  issues.push(...gatewayProfileLifecycleIssues(profile));

  return issues;
}

/** Return all governance violations, including duplicate profile identifiers. */
export function gatewayProfileGovernanceIssuesFor(
  profiles: readonly GatewayProfile[],
): string[] {
  const issues: string[] = [];
  const seen = new Set<string>();
  for (const profile of profiles) {
    if (seen.has(profile.id)) issues.push(`${profile.id}: duplicate profile id`);
    seen.add(profile.id);
    issues.push(...gatewayProfileGovernanceIssues(profile));
  }
  return issues;
}

/** A redacted, non-persistent readiness report returned by the desktop app. */
export interface GatewayReadinessReport {
  profileId: GatewayProfileId;
  configuration: Array<{ label: string; environmentVariable: string; present: boolean }>;
  credentials: Array<{ label: string; environmentVariable: string; present: boolean }>;
  connectivity: { attempted: boolean; status: string; detail: string };
  /** Always false: a local preflight cannot prove a profile is live. */
  live: false;
  guidance: string;
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

export function gatewayReadinessSummary(report: GatewayReadinessReport): string {
  const configurationPresent = report.configuration.filter((item) => item.present).length;
  const credentialsPresent = report.credentials.filter((item) => item.present).length;
  return `${configurationPresent}/${report.configuration.length} configuration and ${credentialsPresent}/${report.credentials.length} credential variables present (values redacted). ${report.connectivity.detail}`;
}

export function gatewayProfileLifecycleSummary(profile: GatewayProfile): string {
  const blocked =
    profile.lifecycle.stages.find((stage) => stage.state === "blocked") ??
    profile.lifecycle.stages.find((stage) => stage.state !== "available");
  if (profile.lifecycle.automationEnabled) {
    return "Managed lifecycle evidence is complete for every stage.";
  }
  return blocked
    ? `Guided lifecycle; automation is gated at ${blocked.label.toLowerCase()}.`
    : "Guided lifecycle; no live setup or traffic change is implied.";
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
    lifecycle: {
      automationEnabled: false,
      stages: [
        { id: "detect", label: "Detect local proxy", state: "available", evidence: "Read-only environment presence and explicit loopback preflight are available." },
        { id: "preview", label: "Preview manual setup", state: "available", evidence: "Switchboard can copy a redacted, not-applied configuration preview." },
        { id: "backup", label: "Back up configuration", state: "guided", evidence: "The user owns LiteLLM files and must create backups outside Switchboard." },
        { id: "apply", label: "Apply local cache", state: "blocked", evidence: "Switchboard never writes LiteLLM or provider configuration." },
        { id: "verify", label: "Verify cache health", state: "guided", evidence: "Doctor evidence requires user-provided proxy, backend, and cache-hit proof." },
        { id: "rollback", label: "Rollback cache setup", state: "guided", evidence: "The user removes manually added environment variables or proxy configuration." },
        { id: "offCleanup", label: "Clean up in Off mode", state: "guided", evidence: "Off guidance explains how to stop the proxy and remove manual variables." },
      ],
    },
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
    lifecycle: {
      automationEnabled: false,
      stages: [
        { id: "detect", label: "Detect endpoint", state: "available", evidence: "Read-only environment presence check reports endpoint and key variables without values." },
        { id: "preview", label: "Preview export setup", state: "available", evidence: "Switchboard provides copyable setup guidance without sending a trace." },
        { id: "backup", label: "Back up export settings", state: "guided", evidence: "Instrumentation and secure-store backups remain user-owned." },
        { id: "apply", label: "Apply trace export", state: "blocked", evidence: "Switchboard does not install instrumentation or change client export settings." },
        { id: "verify", label: "Verify test trace", state: "guided", evidence: "A user-owned endpoint and explicit test trace are required for live proof." },
        { id: "rollback", label: "Rollback export", state: "guided", evidence: "Disable export in the instrumented client and revoke keys as needed." },
        { id: "offCleanup", label: "Clean up in Off mode", state: "guided", evidence: "Off guidance requires the user to disable instrumentation; no traces are sent by Switchboard." },
      ],
    },
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
    lifecycle: {
      automationEnabled: false,
      stages: [
        { id: "detect", label: "Detect gateway URL", state: "available", evidence: "Read-only environment presence check reports the configured URL without exposing its value." },
        { id: "preview", label: "Preview remote routing", state: "available", evidence: "Switchboard can copy a redacted remote-routing preview and disclosure." },
        { id: "backup", label: "Back up client routing", state: "guided", evidence: "The user must back up the client environment or settings before a manual route change." },
        { id: "apply", label: "Apply remote routing", state: "blocked", evidence: "Guided Cloudflare setup never writes provider or client configuration." },
        { id: "verify", label: "Verify passthrough", state: "guided", evidence: "A harmless request and user-owned gateway evidence are required; Switchboard does not contact remote gateways." },
        { id: "rollback", label: "Rollback remote routing", state: "guided", evidence: "Restore the direct provider URL and revoke the gateway token through the client or account owner." },
        { id: "offCleanup", label: "Clean up in Off mode", state: "guided", evidence: "Remove the manually added URL/token outside Switchboard; no provider files are changed here." },
      ],
    },
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
      "Remote enterprise gateway routing is not managed by Switchboard. A Kong deployment may process prompts and outputs before they reach a provider.",
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
    setupGuidance: `# Kong Enterprise Gateway (gated)\n# Use your enterprise gateway runbook. Switchboard does not install, configure,\n# or route Kong traffic. Keep credentials in the enterprise secure store, never\n# in this repository. Document owner, rollback, and approved health evidence\n# before enabling a client-side route.`,
    lifecycle: {
      automationEnabled: false,
      stages: [
        { id: "detect", label: "Detect enterprise gateway", state: "guided", evidence: "An enterprise operator must provide an approved deployment and ownership signal." },
        { id: "preview", label: "Preview enterprise route", state: "guided", evidence: "Switchboard only provides a manual dossier; no Kong topology is inspected." },
        { id: "backup", label: "Back up enterprise route", state: "guided", evidence: "The enterprise change process owns backups and restore points." },
        { id: "apply", label: "Apply enterprise routing", state: "blocked", evidence: "Kong remains gated; Switchboard never installs or mutates enterprise gateway configuration." },
        { id: "verify", label: "Verify enterprise health", state: "guided", evidence: "Use an operator-approved health signal without enterprise-only assumptions in Switchboard." },
        { id: "rollback", label: "Rollback enterprise routing", state: "guided", evidence: "Follow the enterprise runbook to restore direct provider routing." },
        { id: "offCleanup", label: "Clean up in Off mode", state: "guided", evidence: "The enterprise operator removes client routing and revokes access through the approved process." },
      ],
    },
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
