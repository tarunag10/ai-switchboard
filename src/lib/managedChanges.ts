export type ManagedChangeKind =
  | "client_config"
  | "shell_hook"
  | "managed_storage"
  | "repo_index"
  | "launch_agent"
  | "app_state"
  | "plugin";

export interface ManagedChangeRecord {
  id: string;
  kind: ManagedChangeKind;
  owner: string;
  text: string;
  paths: string[];
  markerId: string;
  backupPath: string | null;
  lastVerifiedLabel: string;
  rollback: string;
}

export interface ManagedConfigDiffPreview {
  recordId: ManagedChangeRecord["id"];
  owner: string;
  targetPath: string;
  markerId: string;
  backupPath: string;
  currentManagedBlock: string | null;
  proposedManagedBlock: string;
  rollback: string;
  safetyNotes: string[];
}

export const managedChangeRecords: ManagedChangeRecord[] = [
  {
    id: "claude-code-routing",
    kind: "client_config",
    owner: "Claude Code routing",
    text: "Managed ANTHROPIC_BASE_URL shell block and Claude Code settings hook.",
    paths: [
      "~/.zshrc",
      "~/.zprofile",
      "~/.claude/settings.json",
      "~/.claude/settings.local.json",
    ],
    markerId: "headroom:claude_code",
    backupPath: "next to edited client config as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback:
      "Remove managed Claude Code shell routing, settings env, and Switchboard hook entries.",
  },
  {
    id: "codex-routing",
    kind: "client_config",
    owner: "Codex routing",
    text: "Managed OPENAI_BASE_URL shell block and Codex provider config.",
    paths: ["~/.codex/config.toml", "~/.zshrc", "~/.zprofile"],
    markerId: "headroom:codex_cli",
    backupPath: "next to edited client config as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback:
      "Remove managed Codex shell routing, provider config, AGENTS.md nudge, and thread provider tags.",
  },
  {
    id: "managed-hooks",
    kind: "shell_hook",
    owner: "RTK shell compression",
    text: "Managed hook scripts and shell-profile blocks created by the app.",
    paths: [
      "~/.claude/hooks/headroom-rtk-rewrite.sh",
      "~/.zshrc",
      "~/.zprofile",
    ],
    markerId: "headroom:rtk",
    backupPath: "next to edited shell profile as *.headroom.bak",
    lastVerifiedLabel: "Verified by RTK path and hook checks",
    rollback: "Delete managed hook scripts and shell-profile blocks created by the app.",
  },
  {
    id: "managed-storage",
    kind: "managed_storage",
    owner: "Mac AI Switchboard runtime",
    text: "Managed runtime storage, logs, caches, receipts, setup state, and legacy runtime folders.",
    paths: ["~/Library/Application Support/Headroom", "~/.headroom"],
    markerId: "managed storage path",
    backupPath: null,
    lastVerifiedLabel: "Verified by release and uninstall checks",
    rollback:
      "Delete managed runtime storage, logs, caches, receipts, setup state, Repo Intelligence summaries, and legacy runtime folders.",
  },
  {
    id: "repo-intelligence",
    kind: "repo_index",
    owner: "Repo Intelligence",
    text: "Saved Repo Intelligence local index metadata. User repositories are not modified.",
    paths: [
      "~/Library/Application Support/Headroom/config/repo-intelligence-latest.json",
    ],
    markerId: "repo-intelligence-latest.json",
    backupPath: null,
    lastVerifiedLabel: "Verified when latest summary loads",
    rollback:
      "Remove saved Repo Intelligence local index metadata. User repositories are not modified.",
  },
  {
    id: "login-item",
    kind: "launch_agent",
    owner: "Launch at login",
    text: "App-managed LaunchAgent files and launch-at-login state.",
    paths: ["~/Library/LaunchAgents/"],
    markerId: "com.extraheadroom.headroom",
    backupPath: null,
    lastVerifiedLabel: "Verified by autostart status check",
    rollback: "Disable launch-at-login and remove app-managed LaunchAgent files.",
  },
  {
    id: "app-state",
    kind: "app_state",
    owner: "Mac AI Switchboard app state",
    text: "App preferences, caches, logs, and known Keychain entries.",
    paths: [
      "~/Library/Preferences/com.extraheadroom.headroom*",
      "~/Library/Caches/com.extraheadroom.headroom",
    ],
    markerId: "com.extraheadroom.headroom",
    backupPath: null,
    lastVerifiedLabel: "Verified by uninstall disclosure",
    rollback: "Delete app preferences, caches, logs, and known Keychain entries.",
  },
  {
    id: "plugins-backups",
    kind: "plugin",
    owner: "Add-ons",
    text: "Ponytail plugin registration and managed backup files created next to edited configs.",
    paths: [],
    markerId: "headroom:addon",
    backupPath: "next to edited add-on config as *.headroom.bak",
    lastVerifiedLabel: "Verified by add-on health checks",
    rollback:
      "Remove Ponytail plugin registration and sweep managed backup files created next to edited configs.",
  },
];

export function buildManagedConfigDiffPreview({
  record,
  targetPath,
  currentManagedBlock,
  proposedManagedBlock,
}: {
  record: ManagedChangeRecord;
  targetPath: string;
  currentManagedBlock?: string | null;
  proposedManagedBlock: string;
}): ManagedConfigDiffPreview {
  if (!record.backupPath) {
    throw new Error(`${record.id} does not require a config backup.`);
  }
  const trimmedPath = targetPath.trim();
  if (!trimmedPath) {
    throw new Error("targetPath is required for managed config diff preview.");
  }
  const trimmedProposedBlock = proposedManagedBlock.trim();
  if (!trimmedProposedBlock) {
    throw new Error("proposedManagedBlock is required.");
  }

  return {
    recordId: record.id,
    owner: record.owner,
    targetPath: trimmedPath,
    markerId: record.markerId,
    backupPath: record.backupPath,
    currentManagedBlock: currentManagedBlock?.trim() || null,
    proposedManagedBlock: trimmedProposedBlock,
    rollback: record.rollback,
    safetyNotes: [
      "Review this dry-run diff before applying changes.",
      "Back up the target config before writing the proposed managed block.",
      "Off mode must remove only Switchboard-owned marked changes.",
    ],
  };
}

export function formatManagedConfigDiffPreview(
  preview: ManagedConfigDiffPreview,
): string {
  return [
    `Managed config diff: ${preview.owner}`,
    `Target: ${preview.targetPath}`,
    `Marker: ${preview.markerId}`,
    `Backup: ${preview.backupPath}`,
    "",
    "Current managed block:",
    preview.currentManagedBlock ?? "(none detected)",
    "",
    "Proposed managed block:",
    preview.proposedManagedBlock,
    "",
    "Rollback:",
    preview.rollback,
    "",
    "Safety:",
    ...preview.safetyNotes.map((note) => `- ${note}`),
  ].join("\n");
}

export function formatManagedRollbackInventory(
  records: ManagedChangeRecord[] = managedChangeRecords,
): string {
  return [
    "Mac AI Switchboard Rollback Center inventory",
    "No files are changed by this report.",
    "",
    ...records.flatMap((record) => [
      `## ${record.owner}`,
      `Kind: ${record.kind}`,
      `Managed change: ${record.text}`,
      `Paths: ${record.paths.length > 0 ? record.paths.join(", ") : "none recorded"}`,
      `Marker: ${record.markerId}`,
      `Backup: ${record.backupPath ?? "not required"}`,
      `Verified by: ${record.lastVerifiedLabel}`,
      `Rollback: ${record.rollback}`,
      "",
    ]),
    "Review dry-run diffs before applying config changes. Off mode must remove only Switchboard-owned marked changes.",
  ]
    .join("\n")
    .trimEnd();
}
