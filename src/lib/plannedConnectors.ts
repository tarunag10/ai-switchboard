export interface PlannedConnector {
  id: string;
  name: string;
  category: "cli" | "editor" | "agent";
  statusLabel: "Planned";
  setupPhase: "Detect" | "Guide" | "Adapt";
  integrationTarget: string;
  notes: string;
  capabilityBadges: string[];
  capabilityRows: PlannedConnectorCapability[];
  supportedModes: string[];
  safeToday: string;
  firstAutomation: string;
  configSurfaces: string[];
  automationGates: string[];
  manualWorkflow: string[];
}

export interface PlannedConnectorCapability {
  label: string;
  state: "Available now" | "Manual today" | "Planned";
  detail: string;
}

export interface PlannedConnectorSupportSummary {
  connectorCount: number;
  safeTodayCount: number;
  manualTodayCount: number;
  plannedCount: number;
  automationGateCount: number;
  safeTodayLabels: string[];
  plannedLabels: string[];
}

export interface PlannedConnectorSetupGuide {
  label: string;
  command: string;
  notes: string;
}

export const plannedConnectors: PlannedConnector[] = [
  {
    id: "gemini_cli",
    name: "Gemini CLI",
    category: "cli",
    statusLabel: "Planned",
    setupPhase: "Guide",
    integrationTarget: "Reversible local config base-url routing adapter.",
    notes:
      "Detect installed CLI first, then add Headroom routing only when provider configuration supports local proxy.",
    capabilityBadges: [
      "CLI detection",
      "RTK-safe today",
      "Provider routing pending",
    ],
    supportedModes: ["RTK only", "Off"],
    safeToday:
      "Detect binary and use RTK around verbose Gemini shell runs; provider routing remains manual.",
    firstAutomation:
      "Add a read-only config probe that reports detected provider surface and model/account compatibility.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail:
          "Switchboard can surface the installed Gemini CLI without editing files.",
      },
      {
        label: "Token-saving shell output",
        state: "Available now",
        detail: "RTK-only mode can be used around noisy Gemini commands today.",
      },
      {
        label: "Provider routing",
        state: "Planned",
        detail:
          "Automatic base-url routing waits for backed-up Gemini config support.",
      },
    ],
    configSurfaces: [
      "Gemini CLI binary",
      "provider settings",
      "shell environment",
    ],
    automationGates: [
      "Detect a stable Gemini config file or documented provider flag.",
      "Back up and restore provider settings before enabling setup.",
      "Verify Off mode removes local proxy routing without changing account state.",
    ],
    manualWorkflow: [
      "Confirm the Gemini CLI binary is installed.",
      "Use RTK-only mode around noisy Gemini shell commands.",
      "Keep provider routing manual until the Doctor can verify account and model compatibility.",
    ],
  },
  {
    id: "opencode",
    name: "OpenCode",
    category: "cli",
    statusLabel: "Planned",
    setupPhase: "Adapt",
    integrationTarget:
      "Reversible provider config adapter plus RTK shell-output support.",
    notes:
      "Keep off-mode cleanup symmetric with Claude Code and Codex before enabling automatic setup.",
    capabilityBadges: [
      "CLI detection",
      "RTK-safe today",
      "Backup/restore pending",
    ],
    supportedModes: ["RTK only", "Off"],
    safeToday:
      "Detect binary and compact command output while provider config handling stays untouched.",
    firstAutomation:
      "Ship backup/restore for the active provider config path before enabling Headroom routing.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can identify a local OpenCode binary.",
      },
      {
        label: "Token-saving shell output",
        state: "Available now",
        detail:
          "RTK can compact command output while OpenCode adapter work continues.",
      },
      {
        label: "Config edits",
        state: "Planned",
        detail:
          "Automatic setup is gated on backup, restore, and Off mode cleanup.",
      },
    ],
    configSurfaces: ["OpenCode binary", "provider config", "shell environment"],
    automationGates: [
      "Identify the active provider config path without guessing.",
      "Create timestamped backups before any provider edits.",
      "Prove Off mode restores the exact previous provider config.",
    ],
    manualWorkflow: [
      "Confirm OpenCode is installed.",
      "Run OpenCode commands through RTK when output is noisy.",
      "Leave provider config edits manual until backup and restore checks ship.",
    ],
  },
  {
    id: "cursor",
    name: "Cursor",
    category: "editor",
    statusLabel: "Planned",
    setupPhase: "Guide",
    integrationTarget:
      "Editor settings/profile detection with opt-in local proxy routing where supported.",
    notes:
      "Treat as guided setup first because Cursor settings and extension behavior can vary by account release channel.",
    capabilityBadges: [
      "App detection",
      "Guided setup",
      "Settings backup pending",
    ],
    supportedModes: ["Repo packs", "Guided setup", "Off"],
    safeToday:
      "Show Cursor as a guided editor target and let users copy Repo Intelligence packs into sessions.",
    firstAutomation:
      "Add profile-aware settings discovery with a dry-run diff before any settings write.",
    capabilityRows: [
      {
        label: "App detection",
        state: "Available now",
        detail: "Switchboard can show Cursor as a planned editor connector.",
      },
      {
        label: "Manual setup",
        state: "Manual today",
        detail:
          "Open Cursor settings and review model/provider routing manually.",
      },
      {
        label: "Settings adapter",
        state: "Planned",
        detail:
          "Automatic edits wait for profile-aware backups and restore tests.",
      },
    ],
    configSurfaces: ["Cursor app bundle", "user settings", "profile settings"],
    automationGates: [
      "Detect the active Cursor profile before reading settings.",
      "Back up settings without touching extension-managed secrets.",
      "Keep account-specific model choices visible before routing.",
    ],
    manualWorkflow: [
      "Open Cursor settings from the setup guide.",
      "Review provider/model settings manually.",
      "Use Repo Intelligence packs as copyable context until editor handoff is stable.",
    ],
  },
  {
    id: "grok_cli",
    name: "Grok / xAI CLI",
    category: "cli",
    statusLabel: "Planned",
    setupPhase: "Detect",
    integrationTarget:
      "Provider/base-url adapter once a stable local CLI surface is identified.",
    notes:
      "Track separately from generic OpenAI-compatible clients so account/model constraints stay visible in Doctor.",
    capabilityBadges: [
      "CLI detection",
      "Model guardrails pending",
      "Provider routing pending",
    ],
    supportedModes: ["RTK only", "Off"],
    safeToday:
      "Detect grok or xai commands and keep model/provider choices visible instead of auto-routing.",
    firstAutomation:
      "Add Doctor model/account guardrails before a local provider adapter is offered.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check for local grok or xai commands.",
      },
      {
        label: "Model guardrails",
        state: "Planned",
        detail:
          "Doctor should prevent unsupported model/account combinations before routing.",
      },
      {
        label: "Provider routing",
        state: "Planned",
        detail:
          "Automatic setup waits for a stable OpenAI-compatible local config surface.",
      },
    ],
    configSurfaces: [
      "grok or xai binary",
      "provider/model flags",
      "shell environment",
    ],
    automationGates: [
      "Detect a stable xAI CLI surface.",
      "Add Doctor guardrails for unsupported model/account combinations.",
      "Keep API key and account state outside managed app storage.",
    ],
    manualWorkflow: [
      "Confirm whether grok or xai exists locally.",
      "Use RTK-only mode for command output savings.",
      "Keep model selection manual until compatibility checks are explicit.",
    ],
  },
  {
    id: "aider",
    name: "Aider",
    category: "agent",
    statusLabel: "Planned",
    setupPhase: "Adapt",
    integrationTarget:
      "Local environment/provider wrapper plus Repo Intelligence context packs.",
    notes:
      "Good fit for RTK and future repo graph context because it is frequently used inside long coding sessions.",
    capabilityBadges: ["CLI detection", "RTK-safe today", "Repo packs planned"],
    supportedModes: ["RTK only", "Repo packs", "Off"],
    safeToday:
      "Use RTK for noisy verification commands and copy implementation or handoff packs into Aider.",
    firstAutomation:
      "Add a reversible environment wrapper that points one Aider launch at local routing without editing saved secrets.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can surface a local Aider install.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Use Repo Intelligence context packs alongside Aider prompts today.",
      },
      {
        label: "Provider wrapper",
        state: "Planned",
        detail:
          "Automatic provider environment wrapping waits for reversible setup state.",
      },
    ],
    configSurfaces: [
      "Aider binary",
      "provider environment",
      "repo context files",
    ],
    automationGates: [
      "Detect provider configuration without exposing secrets.",
      "Route through a reversible environment wrapper first.",
      "Expose Repo Intelligence packs without writing into the repo by default.",
    ],
    manualWorkflow: [
      "Confirm Aider is installed.",
      "Copy implementation or handoff packs into long Aider sessions.",
      "Use RTK-only mode for noisy verification commands.",
    ],
  },
  {
    id: "continue",
    name: "Continue",
    category: "editor",
    statusLabel: "Planned",
    setupPhase: "Guide",
    integrationTarget: "Local config adapter with explicit backup and restore.",
    notes:
      "Start with read-only detection and guided setup because Continue configs often contain multiple providers.",
    capabilityBadges: [
      "Config detection",
      "Guided setup",
      "Backup/restore pending",
    ],
    supportedModes: ["Repo packs", "Guided setup", "Off"],
    safeToday:
      "Guide users to review Continue provider config and copy Repo Intelligence packs manually.",
    firstAutomation:
      "Parse the provider list losslessly, back it up, and show an exact restore plan before writes.",
    capabilityRows: [
      {
        label: "Config detection",
        state: "Available now",
        detail: "Switchboard can guide users to the Continue config folder.",
      },
      {
        label: "Manual setup",
        state: "Manual today",
        detail:
          "Review configured providers before choosing any local proxy route.",
      },
      {
        label: "Config adapter",
        state: "Planned",
        detail:
          "Automatic edits wait for multi-provider backup and restore coverage.",
      },
    ],
    configSurfaces: [
      "Continue config folder",
      "provider list",
      "editor integration",
    ],
    automationGates: [
      "Parse multi-provider configs without dropping unknown fields.",
      "Back up the exact config before provider routing changes.",
      "Offer guided setup before automatic edits.",
    ],
    manualWorkflow: [
      "Open the Continue config folder.",
      "Review configured providers manually.",
      "Use Repo Intelligence packs beside Continue until the adapter can preserve every provider entry.",
    ],
  },
  {
    id: "goose",
    name: "Goose",
    category: "agent",
    statusLabel: "Planned",
    setupPhase: "Adapt",
    integrationTarget:
      "Local provider adapter and MCP/Repo Intelligence handoff.",
    notes:
      "Useful target once Switchboard has a stable connector capability model for agent-style tools.",
    capabilityBadges: [
      "CLI detection",
      "MCP handoff planned",
      "Repo packs planned",
    ],
    supportedModes: ["RTK only", "Repo packs", "Off"],
    safeToday:
      "Detect Goose and copy Repo Intelligence packs into sessions while MCP handoff remains planned.",
    firstAutomation:
      "Prototype a read-only MCP handoff manifest before managing provider configuration.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check for a local Goose command.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Copy Repo Intelligence packs into Goose sessions while adapter work lands.",
      },
      {
        label: "MCP handoff",
        state: "Planned",
        detail:
          "Automatic MCP and provider handoff waits for tested connector state.",
      },
    ],
    configSurfaces: ["Goose binary", "provider config", "MCP handoff"],
    automationGates: [
      "Detect Goose provider configuration safely.",
      "Confirm MCP handoff shape before adding managed setup.",
      "Verify Off mode removes local provider routing and leaves MCP config intact.",
    ],
    manualWorkflow: [
      "Confirm Goose is installed.",
      "Copy Repo Intelligence packs into Goose sessions today.",
      "Wait for managed MCP handoff before enabling automatic provider setup.",
    ],
  },
  {
    id: "qwen_code",
    name: "Qwen Code",
    category: "cli",
    statusLabel: "Planned",
    setupPhase: "Guide",
    integrationTarget:
      "CLI detection plus read-only Repo Intelligence handoff before reversible provider routing.",
    notes:
      "Treat as provider/account-sensitive coding CLI until model account compatibility can be checked without editing config.",
    capabilityBadges: [
      "CLI detection",
      "Repo packs today",
      "Provider routing pending",
    ],
    supportedModes: ["RTK only", "Repo packs", "Off"],
    safeToday:
      "Detect local Qwen Code command copy bounded Repo Intelligence packs into sessions.",
    firstAutomation:
      "Add read-only provider/model probe reversible environment wrapper before routing through Headroom.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check qwen-code or qwen commands.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Use Repo Intelligence implementation packs with Qwen Code today.",
      },
      {
        label: "Provider routing",
        state: "Planned",
        detail:
          "Automatic routing waits model/account guardrails restore coverage.",
      },
    ],
    configSurfaces: [
      "Qwen Code binary",
      "provider/model settings",
      "shell environment",
    ],
    automationGates: [
      "Detect stable Qwen Code command provider surface.",
      "Validate model/account compatibility before routing.",
      "Verify Off mode removes local proxy routing without touching account state.",
    ],
    manualWorkflow: [
      "Confirm Qwen Code installed.",
      "Copy Repo Intelligence implementation packs into Qwen Code sessions.",
      "Keep provider routing manual until Doctor can verify compatibility.",
    ],
  },
  {
    id: "amazon_q",
    name: "Amazon Q Developer CLI",
    category: "cli",
    statusLabel: "Planned",
    setupPhase: "Detect",
    integrationTarget:
      "Local CLI detection verification-pack handoff without changing AWS or provider state.",
    notes:
      "Keep AWS account credentials profile state outside managed switchboard storage.",
    capabilityBadges: [
      "CLI detection",
      "Repo packs today",
      "Credential-safe pending",
    ],
    supportedModes: ["RTK only", "Repo packs", "Off"],
    safeToday:
      "Detect Amazon Q CLI use verification packs build/test questions.",
    firstAutomation:
      "Add read-only provider status detection that never reads or stores AWS secrets.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check whether q present on PATH.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Use Repo Intelligence verification packs Amazon Q Developer CLI.",
      },
      {
        label: "Credential guardrails",
        state: "Planned",
        detail:
          "Automatic setup waits AWS profile-safe detection restore policy.",
      },
    ],
    configSurfaces: [
      "Amazon Q CLI binary",
      "AWS profile state",
      "shell environment",
    ],
    automationGates: [
      "Detect q command without reading credentials.",
      "Keep AWS profile SSO state outside app storage.",
      "Prove Off mode does not alter AWS config or credentials.",
    ],
    manualWorkflow: [
      "Confirm Amazon Q Developer CLI is installed.",
      "Use Repo Intelligence verification packs build test work.",
      "Do not route provider traffic automatically until credential guardrails ship.",
    ],
  },
  {
    id: "windsurf",
    name: "Windsurf",
    category: "editor",
    statusLabel: "Planned",
    setupPhase: "Guide",
    integrationTarget:
      "Editor detection copyable Repo Intelligence handoff before settings adapter.",
    notes:
      "Start guided editor settings account surfaces vary by release channel.",
    capabilityBadges: [
      "App detection",
      "Repo packs today",
      "Settings backup pending",
    ],
    supportedModes: ["Repo packs", "Guided setup", "Off"],
    safeToday:
      "Open Windsurf paste Repo Intelligence handoff packs into assistant manually.",
    firstAutomation:
      "Add settings discovery dry-run profile-aware backup before any provider edits.",
    capabilityRows: [
      {
        label: "App detection",
        state: "Available now",
        detail: "Switchboard can guide users toward Windsurf app surface.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Use handoff packs in Windsurf assistant without writing settings.",
      },
      {
        label: "Settings adapter",
        state: "Planned",
        detail:
          "Automatic edits wait settings backup restore Off mode cleanup.",
      },
    ],
    configSurfaces: [
      "Windsurf app bundle",
      "user settings",
      "profile settings",
    ],
    automationGates: [
      "Detect active Windsurf settings location before reading.",
      "Back up settings without touching account secrets.",
      "Verify Off mode restores exact prior settings.",
    ],
    manualWorkflow: [
      "Open Windsurf manually.",
      "Paste Repo Intelligence handoff packs into assistant.",
      "Wait settings backup restore coverage before automatic routing.",
    ],
  },
  {
    id: "zed_ai",
    name: "Zed AI",
    category: "editor",
    statusLabel: "Planned",
    setupPhase: "Guide",
    integrationTarget:
      "Editor detection read-only context handoff before provider settings support.",
    notes:
      "Keep provider/account selection manual until Zed settings parsing restore lossless.",
    capabilityBadges: [
      "App detection",
      "Repo packs today",
      "Settings backup pending",
    ],
    supportedModes: ["Repo packs", "Guided setup", "Off"],
    safeToday:
      "Open Zed paste bounded Repo Intelligence handoffs into assistant manually.",
    firstAutomation:
      "Parse Zed assistant settings read-only show dry-run diff before edits.",
    capabilityRows: [
      {
        label: "App detection",
        state: "Available now",
        detail: "Switchboard can guide users toward Zed app surface.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Use Repo Intelligence handoff packs in Zed AI without config writes.",
      },
      {
        label: "Settings adapter",
        state: "Planned",
        detail:
          "Automatic routing waits lossless settings parse restore coverage.",
      },
    ],
    configSurfaces: [
      "Zed app bundle",
      "assistant settings",
      "provider settings",
    ],
    automationGates: [
      "Detect Zed assistant settings without guessing paths.",
      "Back up provider settings before any local proxy route.",
      "Verify Off mode restores exact previous assistant settings.",
    ],
    manualWorkflow: [
      "Open Zed manually.",
      "Paste Repo Intelligence handoff packs into Zed AI.",
      "Keep model/provider settings manual until restore checks ship.",
    ],
  },
];

export function getPlannedConnector(id: string) {
  return plannedConnectors.find((connector) => connector.id === id) ?? null;
}

export function summarizePlannedConnectorSupport(
  connectors: PlannedConnector[] = plannedConnectors,
): PlannedConnectorSupportSummary {
  const capabilityRows = connectors.flatMap((connector) =>
    connector.capabilityRows.map((capability) => ({
      connectorName: connector.name,
      ...capability,
    })),
  );
  const safeToday = capabilityRows.filter(
    (capability) => capability.state === "Available now",
  );
  const manualToday = capabilityRows.filter(
    (capability) => capability.state === "Manual today",
  );
  const planned = capabilityRows.filter(
    (capability) => capability.state === "Planned",
  );

  return {
    connectorCount: connectors.length,
    safeTodayCount: safeToday.length,
    manualTodayCount: manualToday.length,
    plannedCount: planned.length,
    automationGateCount: connectors.reduce(
      (total, connector) => total + connector.automationGates.length,
      0,
    ),
    safeTodayLabels: safeToday.map(
      (capability) => `${capability.connectorName}: ${capability.label}`,
    ),
    plannedLabels: planned.map(
      (capability) => `${capability.connectorName}: ${capability.label}`,
    ),
  };
}

export function getPlannedConnectorSetupGuide(
  id: string,
): PlannedConnectorSetupGuide | null {
  switch (id) {
    case "gemini_cli":
      return {
        label: "Check Gemini CLI",
        command: "command -v gemini && gemini --help",
        notes:
          "Use this only to confirm the CLI is present. Mac AI Switchboard will add provider routing once reversible Gemini config support lands.",
      };
    case "opencode":
      return {
        label: "Check OpenCode",
        command: "command -v opencode && opencode --help",
        notes:
          "Confirms the OpenCode binary before the app offers backed-up provider config edits.",
      };
    case "cursor":
      return {
        label: "Open Cursor settings",
        command: "open -a Cursor",
        notes:
          "Open Cursor and review model/provider settings manually. Automatic routing waits for account-safe settings detection.",
      };
    case "grok_cli":
      return {
        label: "Check xAI CLI",
        command: "command -v grok || command -v xai",
        notes:
          "Confirms whether a local xAI/Grok CLI exists. Provider/model compatibility remains a Doctor check before routing.",
      };
    case "aider":
      return {
        label: "Check Aider",
        command: "command -v aider && aider --help",
        notes:
          "Confirms Aider is available. RTK-only mode can already reduce noisy shell output while provider wrapping is built.",
      };
    case "continue":
      return {
        label: "Inspect Continue config",
        command: "open ~/.continue",
        notes:
          "Review configured providers manually. Mac AI Switchboard will only edit Continue once backup and restore coverage exists.",
      };
    case "goose":
      return {
        label: "Check Goose",
        command: "command -v goose && goose --help",
        notes:
          "Confirms Goose is present before local provider and MCP handoff support is enabled.",
      };
    case "qwen_code":
      return {
        label: "Check Qwen Code",
        command: "command -v qwen-code || command -v qwen",
        notes:
          "Confirms Qwen Code is present. Keep provider routing manual until model and account guardrails are explicit.",
      };
    case "amazon_q":
      return {
        label: "Check Amazon Q Developer CLI",
        command: "command -v q && q --version",
        notes:
          "Confirms the CLI is present without reading AWS credentials or changing profile state.",
      };
    case "windsurf":
      return {
        label: "Open Windsurf",
        command: "open -a Windsurf",
        notes:
          "Open Windsurf and paste Repo Intelligence handoffs manually. Automatic settings edits wait backup and restore coverage.",
      };
    case "zed_ai":
      return {
        label: "Open Zed",
        command: "open -a Zed",
        notes:
          "Open Zed and paste Repo Intelligence handoffs manually. Provider settings remain manual until restore checks ship.",
      };
    default:
      return null;
  }
}

export function getPlannedConnectorSetupChecklistScript() {
  const lines = [
    "# Mac AI Switchboard planned-tool detection checks",
    "# Read-only: these commands only inspect local app/CLI availability.",
    ...plannedConnectors.flatMap((connector) => {
      const guide = getPlannedConnectorSetupGuide(connector.id);
      if (!guide) {
        return [];
      }
      return ["", `echo "== ${connector.name} =="`, `${guide.command} || true`];
    }),
  ];

  return lines.join("\n");
}
