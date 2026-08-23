export const SELECTIVE_ACTIVATION_LIMIT = 5;

export type ActivationToolId =
  | "headroom"
  | "rtk"
  | "repo-intelligence"
  | "token-xray"
  | "ponytail"
  | "caveman"
  | "markitdown"
  | "response-cache"
  | "chonkify"
  | "leanctx";

export type ActivationToolKind =
  | "mode"
  | "addon"
  | "refresh"
  | "local-preference";

export interface ActivationToolDefinition {
  id: ActivationToolId;
  label: string;
  description: string;
  kind: ActivationToolKind;
  actionLabel: string;
  scope: string;
}

export const SELECTIVE_ACTIVATION_TOOLS: readonly ActivationToolDefinition[] = [
  { id: "headroom", label: "Headroom Engine", description: "Enable the full local optimization mode.", kind: "mode", actionLabel: "Full mode", scope: "Local proxy mode; provider routing remains governed by the existing policy." },
  { id: "rtk", label: "RTK", description: "Compress shell command output before it reaches coding agents.", kind: "addon", actionLabel: "Enable RTK", scope: "Managed shell integration with reversible hooks." },
  { id: "repo-intelligence", label: "Repo Intelligence", description: "Load the latest local repository graph and context summary.", kind: "refresh", actionLabel: "Load latest summary", scope: "Local, read-only repository metadata and bounded packs; indexing a path remains an explicit action." },
  { id: "token-xray", label: "Token X-Ray", description: "Refresh local context-pressure and savings evidence.", kind: "refresh", actionLabel: "Refresh evidence", scope: "Local analytics; no prompt content is exported by this action." },
  { id: "ponytail", label: "Ponytail", description: "Enable the app-bundled Ponytail guidance profile.", kind: "addon", actionLabel: "Enable Ponytail", scope: "Pinned MIT text resources; no marketplace install, runtime download, Node.js hook, or auto-update." },
  { id: "caveman", label: "Caveman", description: "Enable the selected Caveman output style.", kind: "addon", actionLabel: "Enable Caveman", scope: "Managed local instruction blocks; current Caveman level is preserved." },
  { id: "markitdown", label: "MarkItDown", description: "Enable local document conversion for supported workflows.", kind: "addon", actionLabel: "Enable MarkItDown", scope: "Managed local converter, hook, permission, and cache lifecycle." },
  { id: "response-cache", label: "Exact Response Cache", description: "Enable the local exact-response cache.", kind: "addon", actionLabel: "Enable cache", scope: "Requires Full or Headroom mode; no semantic or provider routing is enabled." },
  { id: "chonkify", label: "Switchboard Pack Compaction", description: "Enable AI Switchboard's built-in deterministic compaction for local Repo Intelligence packs.", kind: "local-preference", actionLabel: "Enable pack compaction", scope: "Switchboard-native, read-only local packs; original repository files remain authoritative. The chonkify value is a legacy preference ID only." },
  { id: "leanctx", label: "Leanctx Shadow", description: "Install and enable the loopback-only shadow sidecar when available.", kind: "addon", actionLabel: "Enable shadow", scope: "Shadow preparation only; live provider routing stays disabled until its promotion gate passes." },
] as const;

const knownToolIds = new Set<ActivationToolId>(SELECTIVE_ACTIVATION_TOOLS.map((tool) => tool.id));

export function normalizeActivationSelection(value: unknown): ActivationToolId[] {
  if (!Array.isArray(value)) return [];
  const normalized: ActivationToolId[] = [];
  for (const item of value) {
    if (typeof item !== "string" || !knownToolIds.has(item as ActivationToolId)) continue;
    const id = item as ActivationToolId;
    if (!normalized.includes(id)) normalized.push(id);
  }
  return normalized;
}

export function validateActivationSelection(value: unknown): string | null {
  if (!Array.isArray(value)) return "Choose exactly five tools.";
  const normalized = normalizeActivationSelection(value);
  if (normalized.length !== value.length) return "Selection contains an unknown or duplicate tool.";
  if (normalized.length !== SELECTIVE_ACTIVATION_LIMIT) return "Choose exactly five tools.";
  return null;
}

export function getActivationTool(id: ActivationToolId): ActivationToolDefinition {
  return SELECTIVE_ACTIVATION_TOOLS.find((tool) => tool.id === id)!;
}
