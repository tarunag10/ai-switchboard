export type RepoMemoryMcpState =
  | "active"
  | "configured"
  | "needs_attention"
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
}

export const repoMemoryMcpInstallCommand = "install_repo_memory_mcp";
export const repoMemoryMcpStartCommand = "start_repo_memory_mcp";
export const repoMemoryMcpStopCommand = "stop_repo_memory_mcp";
export const repoMemoryMcpVerifyCommand = "npm run check:repo-memory-mcp";

export function repoMemoryMcpLifecycle(
  input: RepoMemoryMcpStatusInput,
): RepoMemoryMcpLifecycle {
  if (input.configured === true && input.active === true) {
    const started = input.lastStartedAt
      ? ` Last started: ${input.lastStartedAt}.`
      : "";
    return {
      state: "active",
      status: "Active",
      detail: `Repo Memory MCP is app-managed, read-only, and marked active for supported agents.${started}`,
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy: [
        "Repo Memory MCP active: app-managed read-only repo_context_pack, repo_symbol_lookup, and repo_dependents_of tools are available.",
        `Start action: ${repoMemoryMcpStartCommand}`,
        `Stop action: ${repoMemoryMcpStopCommand}`,
        `Verify: ${repoMemoryMcpVerifyCommand}`,
      ].join("\n"),
    };
  }

  if (input.configured === true) {
    return {
      state: "configured",
      status: "Configured",
      detail:
        "Repo Memory MCP is app-managed, read-only, and available to supported agents.",
      installCommand: repoMemoryMcpInstallCommand,
      startCommand: repoMemoryMcpStartCommand,
      stopCommand: repoMemoryMcpStopCommand,
      verifyCommand: repoMemoryMcpVerifyCommand,
      copy:
        "Repo Memory MCP configured: app-managed read-only repo_context_pack, repo_symbol_lookup, and repo_dependents_of tools are available.",
    };
  }

  if (input.configured === false) {
    const detail =
      input.error?.trim() ||
      "Repo Memory MCP is not configured. Install from Mac AI Switchboard, then verify the read-only tool contract.";
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
        `Install action: ${repoMemoryMcpInstallCommand}`,
        `Verify: ${repoMemoryMcpVerifyCommand}`,
        "Safety: tools must stay read-only and must not expose secret-like repo paths.",
      ].join("\n"),
    };
  }

  return {
    state: "unknown",
    status: "Unknown",
    detail:
      "Repo Memory MCP lifecycle has not been verified. Run the installer and smoke check before relying on agent MCP handoffs.",
    installCommand: repoMemoryMcpInstallCommand,
    startCommand: repoMemoryMcpStartCommand,
    stopCommand: repoMemoryMcpStopCommand,
    verifyCommand: repoMemoryMcpVerifyCommand,
    copy: [
      "Repo Memory MCP status unknown.",
      `Install action: ${repoMemoryMcpInstallCommand}`,
      `Verify: ${repoMemoryMcpVerifyCommand}`,
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
