import { connectorSupportsAutomaticSetup } from "./dashboardHelpers";
import { describeProxySessionAuthStatus } from "./proxySessionAuth";
import {
  repoMemoryMcpInspectorRow,
  type RepoMemoryMcpLifecycle,
} from "./repoMemoryMcp";
import type {
  ClientConnectorStatus,
  RuntimeStatus,
  SwitchboardState,
} from "./types";

export interface SwitchboardInspectorRow {
  label: string;
  status: string;
  detail: string;
  actionLabel?: string;
  actionBusyLabel?: string;
  actionDisabled?: boolean;
  onAction?: () => void;
}

export interface BuildSwitchboardInspectorRowsInput {
  runtimeStatus: RuntimeStatus | null;
  switchboardState: SwitchboardState | null;
  switchboardConnectors: ClientConnectorStatus[];
  doctorRepairBusy: string | null;
  handleDoctorRepair: (action: string) => void;
  openSettingsFocus: (targetId: string) => void;
  repoMemoryLifecycle: RepoMemoryMcpLifecycle;
  addonBusyId: string | null;
  addonBusyLabel: string | null;
  setRepoMemoryMcpActive: (active: boolean) => void | Promise<boolean>;
  prepareRepoMemoryMcp: () => Promise<boolean>;
}

function connectorRoutingRow(
  label: string,
  connector: ClientConnectorStatus | undefined,
  doctorRepairBusy: string | null,
  handleDoctorRepair: (action: string) => void,
): SwitchboardInspectorRow {
  const configured = connector?.enabled === true;
  const verified = connector?.verified === true;
  const canRepairManaged =
    connector?.installed === true &&
    connectorSupportsAutomaticSetup(connector) &&
    (!configured || !verified);
  const managedRepairAction =
    connector?.clientId === "codex"
      ? "repair_codex_setup"
      : connector?.clientId
        ? `repair_client_setup:${connector.clientId}`
        : "repair_client_setups";
  const actionLabel = canRepairManaged
    ? connector?.clientId === "codex"
      ? "Repair Codex"
      : "Auto-fix app-managed setup"
    : undefined;
  const actionDisabled = canRepairManaged
    ? doctorRepairBusy !== null
    : undefined;
  const onAction = canRepairManaged
    ? () => void handleDoctorRepair(managedRepairAction)
    : undefined;
  return {
    label,
    status: configured
      ? verified
        ? "Verified"
        : "Needs test"
      : canRepairManaged
        ? "Repair ready"
        : "Direct",
    detail: connector?.installed
      ? configured
        ? verified
          ? `${connector.name} is routed through Headroom and verified.`
          : `${connector.name} routing is configured; send a test prompt from Connectors.`
        : canRepairManaged
          ? `${connector.name} routing is repair ready. Use ${actionLabel} to re-apply reversible app-managed setup and verify routing evidence.`
          : `${connector.name} is detected but not routed.`
      : `${label.replace(" routing", "")} is not detected on this Mac.`,
    actionLabel,
    actionBusyLabel:
      canRepairManaged && doctorRepairBusy === managedRepairAction
        ? "Repairing"
        : undefined,
    actionDisabled,
    onAction,
  };
}

export function buildSwitchboardInspectorRows(
  input: BuildSwitchboardInspectorRowsInput,
): SwitchboardInspectorRow[] {
  const {
    runtimeStatus,
    switchboardState,
    switchboardConnectors,
    doctorRepairBusy,
    handleDoctorRepair,
    openSettingsFocus,
    repoMemoryLifecycle,
    addonBusyId,
    addonBusyLabel,
    setRepoMemoryMcpActive,
    prepareRepoMemoryMcp,
  } = input;

  const enabledSwitchboardConnectors = switchboardConnectors.filter(
    (connector) => connector.enabled,
  );
  const proxyListenerAddress =
    runtimeStatus?.proxyBindAddress ?? "127.0.0.1:6767";
  const proxyListenerDetail =
    runtimeStatus?.proxyReachable === true
      ? `${proxyListenerAddress} is accepting loopback traffic. ${runtimeStatus?.proxyAuthDetail ?? "The listener is local-only."}`
      : runtimeStatus?.paused
        ? `${proxyListenerAddress} is intentionally stopped while the Headroom engine is paused.`
        : `${proxyListenerAddress} is not accepting traffic.`;
  const backendStatus = runtimeStatus?.backendStatus ?? null;
  const backendPortDetail = backendStatus
    ? backendStatus.port === backendStatus.defaultPort
      ? `${backendStatus.bindAddress} is the default internal Headroom backend port.`
      : `${backendStatus.bindAddress} is the selected fallback internal backend port; ${backendStatus.defaultPort} was unavailable.`
    : "Internal backend port evidence is unavailable.";
  const backendPortStatus =
    backendStatus?.reachable === true
      ? "Reachable"
      : runtimeStatus?.paused
        ? "Paused"
        : "Unreachable";
  const switchboardHeadroomLabel =
    (switchboardState?.enabledClients ?? enabledSwitchboardConnectors).length >
    0
      ? (switchboardState?.enabledClients ?? enabledSwitchboardConnectors)
          .map((connector) => connector.name)
          .join(", ")
      : "No clients enabled";
  const launchAgentStatus = runtimeStatus?.launchAgentStatus ?? null;
  const launchAgentInstalled = launchAgentStatus?.installed === true;
  const legacyLaunchAgentInstalled =
    launchAgentStatus?.legacyInstalled === true;
  const launchAgentLoaded = launchAgentStatus?.loaded === true;
  const legacyLaunchAgentLoaded = launchAgentStatus?.legacyLoaded === true;
  const launchAgentDetail = legacyLaunchAgentInstalled
    ? `Legacy Headroom.plist exists at ${launchAgentStatus?.legacyPath ?? "~/Library/LaunchAgents/Headroom.plist"}. ${launchAgentStatus?.legacyLoadDetail ?? "Legacy launchd load state is unknown."} Run Doctor cleanup or uninstall to remove it.`
    : launchAgentInstalled
      ? `Launch at login plist exists at ${launchAgentStatus?.path ?? "~/Library/LaunchAgents/com.tarunagarwal.mac-ai-switchboard.plist"}. ${launchAgentStatus?.loadDetail ?? "launchd load state is unknown."}`
      : `No app-managed launch-at-login plist found. ${launchAgentStatus?.loadDetail ?? "launchd load state is unknown."}`;
  const switchboardRoutingConnectors =
    switchboardState?.clients ?? switchboardConnectors;
  const codexRoutingConnector = switchboardRoutingConnectors.find(
    (connector) => connector.clientId === "codex",
  );
  const claudeRoutingConnector = switchboardRoutingConnectors.find(
    (connector) => connector.clientId === "claude_code",
  );
  const additionalManagedRoutingConnectors = switchboardRoutingConnectors.filter(
    (connector) =>
      connector.installed === true &&
      connectorSupportsAutomaticSetup(connector) &&
      !["codex", "claude_code"].includes(connector.clientId),
  );
  const enabledConnectorVerifications = switchboardRoutingConnectors
    .filter((connector) => connector.enabled)
    .map((connector) => connector.setupVerification)
    .filter((verification): verification is NonNullable<typeof verification> =>
      Boolean(verification),
    );
  const managedShellBlockVerified = enabledConnectorVerifications.some(
    (verification) =>
      verification.checks.some((check) =>
        /managed shell block|shell profiles/i.test(check),
      ),
  );
  const managedShellBlockMissing = enabledConnectorVerifications.some(
    (verification) =>
      verification.failures.some((failure) =>
        /shell profiles|shell blocks/i.test(failure),
      ),
  );
  const codexProviderVerified =
    codexRoutingConnector?.setupVerification?.checks.some((check) =>
      /provider block/i.test(check),
    ) === true;
  const codexProviderMissing =
    codexRoutingConnector?.setupVerification?.failures.some((failure) =>
      /provider block/i.test(failure),
    ) === true;

  return [
    {
      label: "Proxy listener",
      status:
        runtimeStatus?.proxyReachable === true
          ? "Reachable"
          : runtimeStatus?.paused
            ? "Paused"
            : "Unreachable",
      detail: proxyListenerDetail,
    },
    {
      label: "Backend port",
      status: backendPortStatus,
      detail: backendPortDetail,
    },
    connectorRoutingRow(
      "Codex routing",
      codexRoutingConnector,
      doctorRepairBusy,
      handleDoctorRepair,
    ),
    connectorRoutingRow(
      "Claude routing",
      claudeRoutingConnector,
      doctorRepairBusy,
      handleDoctorRepair,
    ),
    ...additionalManagedRoutingConnectors.map((connector) =>
      connectorRoutingRow(
        `${connector.name} routing`,
        connector,
        doctorRepairBusy,
        handleDoctorRepair,
      ),
    ),
    {
      label: "Client routing",
      status:
        (switchboardState?.enabledClients ?? enabledSwitchboardConnectors)
          .length > 0
          ? "Managed"
          : "Direct",
      detail: switchboardHeadroomLabel,
    },
    {
      label: "Managed shell blocks",
      status: managedShellBlockVerified
        ? "Verified"
        : managedShellBlockMissing
          ? "Missing"
          : "No proof",
      detail: managedShellBlockVerified
        ? "Connector verification found managed shell routing blocks."
        : managedShellBlockMissing
          ? "Connector verification reported missing shell routing blocks."
          : "No enabled connector has reported shell-block verification yet.",
    },
    {
      label: "Codex provider block",
      status: codexProviderVerified
        ? "Verified"
        : codexProviderMissing
          ? "Missing"
          : codexRoutingConnector?.enabled
            ? "No proof"
            : "Direct",
      detail: codexProviderVerified
        ? "Connector verification found the Headroom-managed provider block in ~/.codex/config.toml."
        : codexProviderMissing
          ? "Connector verification reported the Codex provider block is missing."
          : codexRoutingConnector?.enabled
            ? "Codex is enabled, but provider-block verification has not reported proof yet."
            : "Codex provider routing is repair ready. Use the Codex routing repair-ready row to re-apply the managed provider block.",
    },
    {
      label: "Shell export",
      status: runtimeStatus?.rtk.pathConfigured ? "Configured" : "Not configured",
      detail: runtimeStatus?.rtk.pathConfigured
        ? "Managed RTK PATH export is present."
        : "Managed RTK PATH export is not active.",
    },
    {
      label: "RTK shell hook",
      status: runtimeStatus?.rtk.hookConfigured ? "Configured" : "Not configured",
      detail: runtimeStatus?.rtk.hookConfigured
        ? "Managed RTK command-rewrite hook is present."
        : runtimeStatus?.rtk.installed
          ? "RTK is installed, but the managed shell hook is not active."
          : "RTK shell hook is not installed.",
    },
    {
      label: "Proxy session auth",
      status: describeProxySessionAuthStatus(
        runtimeStatus?.proxyAuthStatus
          ? {
              available: true,
              enforce:
                runtimeStatus.proxyAuthStatus === "session_token_enforced",
              fingerprint: "",
              status: runtimeStatus.proxyAuthStatus,
              detail: runtimeStatus.proxyAuthDetail ?? "",
              validatedRequestCount: 0,
              rejectedRequestCount: 0,
            }
          : null,
      ).label,
      detail:
        runtimeStatus?.proxyAuthDetail ??
        "Proxy session auth status is unavailable.",
      actionLabel: "Open Settings",
      onAction: () => openSettingsFocus("proxy-session-auth"),
    },
    {
      label: "Headroom MCP",
      status:
        runtimeStatus?.mcpConfigured === true
          ? "Configured"
          : runtimeStatus?.mcpConfigured === false
            ? "Not configured"
            : "Unknown",
      detail:
        runtimeStatus?.mcpConfigured === true
          ? "Claude MCP config includes the local Headroom server."
          : runtimeStatus?.mcpConfigured === false
            ? (runtimeStatus.mcpError ??
              "Claude MCP config does not include the local Headroom server.")
            : "Headroom MCP configuration has not been checked yet.",
    },
    {
      ...repoMemoryMcpInspectorRow({
        configured: runtimeStatus?.repoMemoryMcpConfigured,
        error: runtimeStatus?.repoMemoryMcpError,
        active: runtimeStatus?.repoMemoryMcpActive,
        lastStartedAt: runtimeStatus?.repoMemoryMcpLastStartedAt,
        lastCheckedAt: runtimeStatus?.repoMemoryMcpLastCheckedAt,
        supervisionStatus: runtimeStatus?.repoMemoryMcpSupervisionStatus,
        relaunchSurvivalStatus: runtimeStatus?.repoMemoryMcpRelaunchSurvivalStatus,
        supervisionScope: runtimeStatus?.repoMemoryMcpSupervisionScope,
        service: runtimeStatus?.repoMemoryMcpService,
      }),
      actionLabel:
        repoMemoryLifecycle.state === "active"
          ? "Stop MCP"
          : runtimeStatus?.repoMemoryMcpConfigured === true
            ? "Start MCP"
            : "Prepare MCP",
      actionBusyLabel:
        addonBusyId === "repo-memory" ? (addonBusyLabel ?? "Working") : undefined,
      actionDisabled: addonBusyId !== null,
      onAction:
        repoMemoryLifecycle.state === "active"
          ? () => void setRepoMemoryMcpActive(false)
          : runtimeStatus?.repoMemoryMcpConfigured === true
            ? () => void setRepoMemoryMcpActive(true)
            : () => void prepareRepoMemoryMcp(),
    },
    {
      label: "Launch at login",
      status:
        legacyLaunchAgentInstalled || legacyLaunchAgentLoaded
          ? "Legacy found"
          : launchAgentLoaded
            ? "Loaded"
            : launchAgentInstalled
              ? "Installed"
              : "Not installed",
      detail: launchAgentDetail,
    },
  ];
}
