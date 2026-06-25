export interface PlannedConnector {
  id: string;
  name: string;
  category: "cli" | "editor" | "agent";
  statusLabel: "Planned";
  setupPhase: "Detect" | "Guide" | "Adapt";
  integrationTarget: string;
  notes: string;
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
      "Detect the installed CLI first, then add Headroom routing only when the provider configuration supports a local proxy.",
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
  },
  {
    id: "grok_cli",
    name: "Grok / xAI CLI",
    category: "cli",
    statusLabel: "Planned",
    setupPhase: "Detect",
    integrationTarget:
      "Provider/base-url adapter after a stable local CLI surface is identified.",
    notes:
      "Track separately from generic OpenAI-compatible clients so account/model constraints are visible in Doctor.",
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
  },
];

export function getPlannedConnector(id: string) {
  return plannedConnectors.find((connector) => connector.id === id) ?? null;
}

export function getPlannedConnectorSetupGuide(
  id: string
): PlannedConnectorSetupGuide | null {
  switch (id) {
    case "gemini_cli":
      return {
        label: "Check Gemini CLI",
        command: "command -v gemini && gemini --help",
        notes:
          "Use this only to confirm the CLI is present. Mac AI Switchboard will add provider routing after reversible Gemini config support lands.",
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
          "Review configured providers manually. Mac AI Switchboard will only edit Continue after backup and restore coverage exists.",
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
