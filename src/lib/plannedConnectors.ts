export interface PlannedConnector {
  id: string;
  name: string;
  category: "cli" | "editor" | "agent";
  statusLabel: "Planned";
  setupPhase: "Detect" | "Guide" | "Adapt";
  integrationTarget: string;
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
      "Good fit for RTK future repo graph context because it is frequently used inside long coding sessions.",
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
