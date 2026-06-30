export type RepoMemoryMcpState =
  | "active"
  | "configured"
  | "stale"
  | "restart_required"
  | "needs_attention"
  | "smoke_failed"
  | "unknown";

export interface RepoMemoryMcpLifecycle {
  state: RepoMemoryMcpState;
  status: string;
  detail: string;
  installCommand: string;
  startCommand: string;
  stopCommand: string;
  verifyCommand: string;
  copy: string;
}

export interface RepoMemoryMcpStatusInput {
  configured?: boolean | null;
  error?: string | null;
  active?: boolean | null;
  lastStartedAt?: string | null;
  lastCheckedAt?: string | null;
  supervisionStatus?: string | null;
  service?: {
    managedByApp: boolean;
    readOnly: boolean;
    transport: string;
    command: string;
    descriptorPath: string;
    descriptorPresent?: boolean;
    scriptPath?: string;
    scriptPresent?: boolean;
    nodeAvailable?: boolean;
  } | null;
}

export const repoMemoryMcpInstallCommand = "install_repo_memory_mcp";
export const repoMemoryMcpStartCommand = "start_repo_memory_mcp";
export const repoMemoryMcpStopCommand = "stop_repo_memory_mcp";
export const repoMemoryMcpVerifyCommand = "npm run check:repo-memory-mcp";

export function repoMemoryMcpLifecycle(
  input: RepoMemoryMcpStatusInput,
): RepoMemoryMcpLifecycle {
  const service = input.service ?? null;
  const serviceDetail = input.service
    ? ` Service: ${input.service.transport} ${input.service.command}.`
    : "";
  const serviceCopy = input.service
    ? [
        `Service transport: ${input.service.transport}`,
        `Service command: ${input.service.command}`,
        `Descriptor: ${input.service.descriptorPath}`,
        `Descriptor present: ${input.service.descriptorPresent !== false ? "yes" : "no"}`,
        `Script: ${input.service.scriptPath ?? "unknown"}`,
        `Script present: ${input.service.scriptPresent !== false ? "yes" : "no"}`,
        `Node available: ${input.service.nodeAvailable !== false ? "yes" : "no"}`,
      ]
    : [];
  const unsafeService =
    service &&
    (service.managedByApp !== true ||
      service.readOnly !== true ||
      service.descriptorPresent === false ||
      service.scriptPresent === false ||
      service.nodeAvailable === false);
  if (unsafeService) {
    const ownership = service.managedByApp
      ? "app-managed"
      : "not app-managed";
    const access = service.readOnly ? "read-only" : "not read-only";
    const missing = [
      service.descriptorPresent === false ? "descriptor missing" : null,
      service.scriptPresent === false ? "script missing" : null,
      service.nodeAvailable === false ? "node unavailable" : null,
    ].filter(Boolean);
    const health = missing.length ? ` ${missing.join(", ")}.` : "";
    return {
      state: "needs_attention",
      status: "Needs attention",
      detail: `Repo Memory MCP service descriptor is ${ownership} and ${access}.${health} Use Prepare MCP to restore the app-managed read-only descriptor before agent handoffs.${serviceDetail}`,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP descriptor is unsafe for agent handoffs.",
        `Descriptor ownership: ${ownership}`,
        `Descriptor access: ${access}`,
        ...missing.map((item) => `Service health: ${item}`),
        `Prepare action: ${repoMemoryMcpInstallCommand} then ${repoMemoryMcpStartCommand}`,
        `Verify: ${repoMemoryMcpVerifyCommand}`,
        ...serviceCopy,
        "Safety: MCP context must be app-managed and read-only before agents rely on it.",
      ].join("\n"),
    };
  }
  if (input.supervisionStatus === "smoke_failed") {
    const checked = input.lastCheckedAt ? ` Last checked: ${input.lastCheckedAt}.` : "";
    return {
      state: "smoke_failed",
      status: "Smoke failed",
      detail: `Repo Memory MCP is configured, but the read-only smoke check did not pass.${checked}${serviceDetail}`,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP smoke failed: configured state is not enough to rely on agent MCP handoffs.",
        `Start action: ${repoMemoryMcpStartCommand}`,
        `Verify: ${repoMemoryMcpVerifyCommand}`,
        ...serviceCopy,
        "Safety: do not mark MCP active until repo_context_pack, repo_symbol_lookup, and repo_dependents_of pass the read-only smoke.",
      ].join("\n"),
    };
  }

  if (input.supervisionStatus === "stale_config") {
    const checked = input.lastCheckedAt
      ? ` Last checked: ${input.lastCheckedAt}.`
      : "";
    return {
      state: "stale",
      status: "Stale",
      detail: `Repo Memory MCP was marked active, but the managed MCP config is no longer present.${checked}${serviceDetail}`,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP stale: active session state no longer matches managed MCP configuration.",
        `Install action: ${repoMemoryMcpInstallCommand}`,
        `Stop action: ${repoMemoryMcpStopCommand}`,
        `Verify: ${repoMemoryMcpVerifyCommand}`,
        ...serviceCopy,
        "Safety: do not rely on repo-memory MCP handoffs until configuration is repaired.",
      ].join("\n"),
    };
  }

  if (input.supervisionStatus === "service_unhealthy") {
    const checked = input.lastCheckedAt
      ? ` Last checked: ${input.lastCheckedAt}.`
      : "";
    return {
      state: "needs_attention",
      status: "Needs attention",
      detail: `Repo Memory MCP is configured, but current service evidence is unhealthy.${checked}${serviceDetail} Use Prepare MCP to restore the app-managed read-only service before agent handoffs.`,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP service is unhealthy: configured state is not enough to rely on agent MCP handoffs.",
        `Prepare action: ${repoMemoryMcpInstallCommand} then ${repoMemoryMcpStartCommand}`,
        `Verify: ${repoMemoryMcpVerifyCommand}`,
        ...serviceCopy,
        "Safety: descriptor, repo-memory script, and Node runtime evidence must be healthy before agents rely on MCP context.",
      ].join("\n"),
    };
  }

  if (input.supervisionStatus === "restart_required") {
    const started = input.lastStartedAt
      ? ` Last started: ${input.lastStartedAt}.`
      : "";
    const checked = input.lastCheckedAt
      ? ` Last checked: ${input.lastCheckedAt}.`
      : "";
    return {
      state: "restart_required",
      status: "Verifying",
      detail: `Repo Memory MCP was active in a previous app process. Mac AI Switchboard will re-run the read-only smoke check automatically; click Start MCP if you want to retry now.${started}${checked}${serviceDetail}`,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP is being re-verified for this app session.",
        `Start action: ${repoMemoryMcpStartCommand}`,
        `Verify: ${repoMemoryMcpVerifyCommand}`,
        ...serviceCopy,
        "Safety: previous-process active state is not trusted until the app re-runs the read-only smoke check.",
      ].join("\n"),
    };
  }

  if (
    input.configured === true &&
    input.active === true &&
    input.supervisionStatus === "verified_active"
  ) {
    const started = input.lastStartedAt
      ? ` Last started: ${input.lastStartedAt}.`
      : "";
    const checked = input.lastCheckedAt ? ` Last checked: ${input.lastCheckedAt}.` : "";
    return {
      state: "active",
      status: "Active",
      detail: `Repo Memory MCP is app-managed, read-only, smoke-tested, and active for supported agents.${started}${checked}${serviceDetail}`,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP active: app-managed read-only repo_context_pack, repo_symbol_lookup, and repo_dependents_of tools passed smoke verification.",
        `Start action: ${repoMemoryMcpStartCommand}`,
        `Stop action: ${repoMemoryMcpStopCommand}`,
        `Verify: ${repoMemoryMcpVerifyCommand}`,
        ...serviceCopy,
      ].join("\n"),
    };
  }

  if (input.configured === true && input.active === true) {
    const checked = input.lastCheckedAt ? ` Last checked: ${input.lastCheckedAt}.` : "";
    return {
      state: "unknown",
      status: "Needs verification",
      detail: `Repo Memory MCP is marked active, but smoke verification has not been recorded.${checked}${serviceDetail}`,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP active state needs verification.",
        `Start action: ${repoMemoryMcpStartCommand}`,
        `Verify: ${repoMemoryMcpVerifyCommand}`,
        ...serviceCopy,
        "Safety: run Start MCP again so the app records smoke-tested active state.",
      ].join("\n"),
    };
  }

  if (input.configured === true) {
    return {
      state: "configured",
      status: "Configured",
      detail:
        `Repo Memory MCP is app-managed and read-only. Click Start MCP to run the smoke check and mark it active for supported agents.${serviceDetail}`,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP configured: app-managed read-only repo_context_pack, repo_symbol_lookup, and repo_dependents_of tools are available.",
        ...serviceCopy,
      ].join("\n"),
    };
  }

  if (input.configured === false) {
    const detail =
      input.error?.trim() ||
      "Repo Memory MCP is not configured. Use Prepare MCP from Mac AI Switchboard to install it, start it, and verify the read-only tool contract.";
    return {
      state: "needs_attention",
      status: "Needs attention",
      detail,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP needs attention.",
        `Detail: ${detail}`,
        "Prepare action: install_repo_memory_mcp then start_repo_memory_mcp",
        `Optional terminal verify: ${repoMemoryMcpVerifyCommand}`,
        "Safety: tools must stay read-only and must not expose secret-like repo paths.",
      ].join("\n"),
    };
  }

  return {
    state: "unknown",
    status: "Unknown",
    detail:
      "Repo Memory MCP lifecycle has not been verified. Use Prepare MCP to install it and run the smoke check before relying on agent MCP handoffs.",
    installCommand: repoMemoryMcpInstallCommand,
    startCommand: repoMemoryMcpStartCommand,
    stopCommand: repoMemoryMcpStopCommand,
    verifyCommand: repoMemoryMcpVerifyCommand,
    copy: [
      "Repo Memory MCP status unknown.",
      "Prepare action: install_repo_memory_mcp then start_repo_memory_mcp",
      `Optional terminal verify: ${repoMemoryMcpVerifyCommand}`,
      "Safety: repo-memory MCP must remain app-managed and read-only.",
    ].join("\n"),
  };
}

export function repoMemoryMcpInspectorRow(input: RepoMemoryMcpStatusInput) {
  const lifecycle = repoMemoryMcpLifecycle(input);
  return {
    label: "Repo Memory MCP",
    status: lifecycle.status,
    detail: lifecycle.detail,
  };
}
