export interface PlannedConnector {
  id: string;
  name: string;
  category: "cli" | "editor" | "agent";
  statusLabel: "Planned";
  integrationTarget: string;
  notes: string;
}

export const plannedConnectors: PlannedConnector[] = [
  {
    id: "gemini_cli",
    name: "Gemini CLI",
    category: "cli",
    statusLabel: "Planned",
    integrationTarget: "Reversible local config and base-url routing adapter.",
    notes:
      "Detect installed CLI first, then add Headroom routing only when its provider configuration supports a local proxy.",
  },
  {
    id: "opencode",
    name: "OpenCode",
    category: "cli",
    statusLabel: "Planned",
    integrationTarget: "Reversible provider config adapter plus RTK shell-output support.",
    notes:
      "Keep off-mode cleanup symmetric with Claude Code and Codex before enabling automatic setup.",
  },
  {
    id: "cursor",
    name: "Cursor",
    category: "editor",
    statusLabel: "Planned",
    integrationTarget: "Editor settings/profile detection with opt-in local proxy routing where supported.",
    notes:
      "Treat as a guided setup first because Cursor settings and extension behavior can vary by account and release channel.",
  },
  {
    id: "grok_cli",
    name: "Grok / xAI CLI",
    category: "cli",
    statusLabel: "Planned",
    integrationTarget: "Provider/base-url adapter after a stable local CLI surface is identified.",
    notes:
      "Track separately from generic OpenAI-compatible clients so account/model constraints are visible in Doctor.",
  },
  {
    id: "aider",
    name: "Aider",
    category: "agent",
    statusLabel: "Planned",
    integrationTarget: "Local environment/provider wrapper plus repo-intelligence context packs.",
    notes:
      "Good fit for RTK and future repo graph context because it is frequently used inside long coding sessions.",
  },
  {
    id: "continue",
    name: "Continue",
    category: "editor",
    statusLabel: "Planned",
    integrationTarget: "Local config adapter with explicit backup and restore.",
    notes:
      "Start with read-only detection and guided setup because Continue configs often contain multiple providers.",
  },
  {
    id: "goose",
    name: "Goose",
    category: "agent",
    statusLabel: "Planned",
    integrationTarget: "Local provider adapter and MCP/repo-intelligence handoff.",
    notes:
      "Useful target once the switchboard has a stable connector capability model for agent-style tools.",
  },
];

export function getPlannedConnector(id: string) {
  return plannedConnectors.find((connector) => connector.id === id) ?? null;
}
