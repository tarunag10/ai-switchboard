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
}

export interface PlannedConnectorCapability {
  label: string;
  state: "Available now" | "Manual today" | "Planned";
  detail: string;
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
    capabilityBadges: ["CLI detection", "RTK-safe today", "Provider routing pending"],
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can surface the installed Gemini CLI without editing files.",
      },
      {
        label: "Token-saving shell output",
        state: "Available now",
        detail: "RTK-only mode can be used around noisy Gemini commands today.",
      },
      {
        label: "Provider routing",
        state: "Planned",
        detail: "Automatic base-url routing waits for backed-up Gemini config support.",
      },
    ],
  },
  {
    id: "opencode",
    name: "OpenCode",
    category: "cli",
    statusLabel: "Planned",
    setupPhase: "Adapt",
    integrationTarget: "Reversible provider config adapter plus RTK shell-output support.",
    notes:
      "Keep off-mode cleanup symmetric with Claude Code and Codex before enabling automatic setup.",
    capabilityBadges: ["CLI detection", "RTK-safe today", "Backup/restore pending"],
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can identify a local OpenCode binary.",
      },
      {
        label: "Token-saving shell output",
        state: "Available now",
        detail: "RTK can compact command output while OpenCode adapter work continues.",
      },
      {
        label: "Config edits",
        state: "Planned",
        detail: "Automatic setup is gated on backup, restore, and Off mode cleanup.",
      },
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
    capabilityBadges: ["App detection", "Guided setup", "Settings backup pending"],
    capabilityRows: [
      {
        label: "App detection",
        state: "Available now",
        detail: "Switchboard can show Cursor as a planned editor connector.",
      },
      {
        label: "Manual setup",
        state: "Manual today",
        detail: "Open Cursor settings and review model/provider routing manually.",
      },
      {
        label: "Settings adapter",
        state: "Planned",
        detail: "Automatic edits wait for profile-aware backups and restore tests.",
      },
    ],
  },
  {
    id: "grok_cli",
    name: "Grok / xAI CLI",
    category: "cli",
    statusLabel: "Planned",
    setupPhase: "Detect",
    integrationTarget: "Provider/base-url adapter once a stable local CLI surface is identified.",
    notes:
      "Track separately from generic OpenAI-compatible clients so account/model constraints stay visible in Doctor.",
    capabilityBadges: ["CLI detection", "Model guardrails pending", "Provider routing pending"],
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check for local grok or xai commands.",
      },
      {
        label: "Model guardrails",
        state: "Planned",
        detail: "Doctor should prevent unsupported model/account combinations before routing.",
      },
      {
        label: "Provider routing",
        state: "Planned",
        detail: "Automatic setup waits for a stable OpenAI-compatible local config surface.",
      },
    ],
  },
  {
    id: "aider",
    name: "Aider",
    category: "agent",
    statusLabel: "Planned",
    setupPhase: "Adapt",
    integrationTarget: "Local environment/provider wrapper plus Repo Intelligence context packs.",
    notes:
      "Good fit for RTK and future repo graph context because it is frequently used inside long coding sessions.",
    capabilityBadges: ["CLI detection", "RTK-safe today", "Repo packs planned"],
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can surface a local Aider install.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail: "Use Repo Intelligence context packs alongside Aider prompts today.",
      },
      {
        label: "Provider wrapper",
        state: "Planned",
        detail: "Automatic provider environment wrapping waits for reversible setup state.",
      },
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
    capabilityBadges: ["Config detection", "Guided setup", "Backup/restore pending"],
    capabilityRows: [
      {
        label: "Config detection",
        state: "Available now",
        detail: "Switchboard can guide users to the Continue config folder.",
      },
      {
        label: "Manual setup",
        state: "Manual today",
        detail: "Review configured providers before choosing any local proxy route.",
      },
      {
        label: "Config adapter",
        state: "Planned",
        detail: "Automatic edits wait for multi-provider backup and restore coverage.",
      },
    ],
  },
  {
    id: "goose",
    name: "Goose",
    category: "agent",
    statusLabel: "Planned",
    setupPhase: "Adapt",
    integrationTarget: "Local provider adapter and MCP/Repo Intelligence handoff.",
    notes:
      "Useful target once Switchboard has a stable connector capability model for agent-style tools.",
    capabilityBadges: ["CLI detection", "MCP handoff planned", "Repo packs planned"],
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check for a local Goose command.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail: "Copy Repo Intelligence packs into Goose sessions while adapter work lands.",
      },
      {
        label: "MCP handoff",
        state: "Planned",
        detail: "Automatic MCP and provider handoff waits for tested connector state.",
      },
    ],
  },
];

export function getPlannedConnector(id: string) {
  return plannedConnectors.find((connector) => connector.id === id) ?? null;
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
    default:
      return null;
  }
}
