import type { ClientConnectorStatus } from "./types";

export const launcherConnectorFallback: ClientConnectorStatus[] = [
  {
    clientId: "claude_code",
    name: "Claude Code",
    installed: false,
    enabled: false,
    verified: false,
  },
  {
    clientId: "codex",
    name: "Codex",
    installed: false,
    enabled: false,
    verified: false,
  },
];
