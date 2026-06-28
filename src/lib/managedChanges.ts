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
  dryRunOnly: true;
  requiresExplicitConfirmation: true;
  confirmationPhrase: string;
  applyBlockedReason: string;
  writePathStatus: "blocked";
  offModeCleanupBoundary: string;
  currentManagedBlock: string | null;
  proposedManagedBlock: string;
  rollback: string;
  unmanagedConfigPolicy: string;
  safetyNotes: string[];
}

export interface ManagedConfigTextResult {
  text: string;
  changed: boolean;
}

export interface ManagedConfigApplyPlan {
  recordId: ManagedChangeRecord["id"];
  owner: string;
  targetPath: string;
  markerId: string;
  backupPath: string;
  confirmationPhrase: string;
  confirmed: true;
  writePathStatus: "ready";
  nextText: string;
  changed: boolean;
  rollback: string;
  offModeCleanupBoundary: string;
  unmanagedConfigPolicy: string;
  safetyNotes: string[];
}

export type ManagedRollbackMode =
  | "backup_restore"
  | "managed_block_removal"
  | "cleanup_inventory";

export interface ManagedRollbackPlan {
  recordId: ManagedChangeRecord["id"];
  owner: string;
  kind: ManagedChangeKind;
  managedChange: string;
  mode: ManagedRollbackMode;
  status: "ready_for_review" | "cleanup_only";
  targetSummary: string;
  markerId: string;
  backupPath: string | null;
  rollback: string;
  verification: string;
  evidenceRequired: string[];
  unmanagedConfigPolicy: string;
  offModeCleanupBoundary: string;
  safetyNotes: string[];
}

export interface ManagedRollbackExecutionPreview {
  plan: ManagedRollbackPlan;
  executionStatus: "blocked_until_confirmed" | "cleanup_inventory_only";
  confirmationPhrase: string;
  orderedSteps: string[];
  undoAllOrder: number;
  backendAction: "restore_backup_or_remove_marker" | "cleanup_inventory_review";
  nativeWriteStatus: "not_executed";
  blockedReason: string;
}

export interface ManagedRollbackUndoAllPreview {
  executable: ManagedRollbackExecutionPreview[];
  manual: ManagedRollbackExecutionPreview[];
  blockedReason: string;
  orderedSteps: string[];
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
    id: "gemini-routing",
    kind: "client_config",
    owner: "Gemini CLI routing",
    text: "Managed Gemini CLI shell routing exports and rollback dossier.",
    paths: [
      "~/.zshrc",
      "~/.zprofile",
      "~/.gemini/mac-ai-switchboard-routing.md",
    ],
    markerId: "headroom:gemini_cli",
    backupPath: "next to edited shell profile or sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback:
      "Remove managed Gemini CLI shell routing exports and Switchboard sidecar dossier.",
  },
  {
    id: "opencode-routing",
    kind: "client_config",
    owner: "OpenCode routing",
    text: "Managed OpenCode provider config and rollback dossier.",
    paths: [
      "~/.config/opencode/opencode.json",
      "~/.config/opencode/mac-ai-switchboard-routing.md",
    ],
    markerId: "headroom:opencode",
    backupPath: "next to edited OpenCode config or sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback:
      "Remove only the managed OpenCode headroom provider and Switchboard sidecar dossier.",
  },
  {
    id: "cursor-routing",
    kind: "client_config",
    owner: "Cursor routing",
    text: "Managed Cursor Switchboard sidecar dossier.",
    paths: [
      "~/Library/Application Support/Cursor/mac-ai-switchboard-routing.md",
    ],
    markerId: "headroom:cursor",
    backupPath: "next to edited Cursor sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback: "Remove only the managed Cursor Switchboard sidecar dossier.",
  },
  {
    id: "grok-routing",
    kind: "client_config",
    owner: "Grok / xAI CLI routing",
    text: "Managed Grok / xAI CLI Switchboard sidecar dossier.",
    paths: ["~/.config/xai/mac-ai-switchboard-routing.md"],
    markerId: "headroom:grok_cli",
    backupPath: "next to edited Grok / xAI sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback:
      "Remove only the managed Grok / xAI CLI Switchboard sidecar dossier.",
  },
  {
    id: "aider-routing",
    kind: "client_config",
    owner: "Aider routing",
    text: "Managed Aider Switchboard sidecar dossier.",
    paths: ["~/.config/aider/mac-ai-switchboard-routing.md"],
    markerId: "headroom:aider",
    backupPath: "next to edited Aider sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback: "Remove only the managed Aider Switchboard sidecar dossier.",
  },
  {
    id: "continue-routing",
    kind: "client_config",
    owner: "Continue routing",
    text: "Managed Continue Switchboard sidecar dossier.",
    paths: ["~/.continue/mac-ai-switchboard-routing.md"],
    markerId: "headroom:continue",
    backupPath: "next to edited Continue sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback: "Remove only the managed Continue Switchboard sidecar dossier.",
  },
  {
    id: "goose-routing",
    kind: "client_config",
    owner: "Goose routing",
    text: "Managed Goose Switchboard sidecar dossier.",
    paths: ["~/.config/goose/mac-ai-switchboard-routing.md"],
    markerId: "headroom:goose",
    backupPath: "next to edited Goose sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback: "Remove only the managed Goose Switchboard sidecar dossier.",
  },
  {
    id: "qwen-code-routing",
    kind: "client_config",
    owner: "Qwen Code routing",
    text: "Managed Qwen Code Switchboard sidecar dossier.",
    paths: ["~/.qwen/mac-ai-switchboard-routing.md"],
    markerId: "headroom:qwen_code",
    backupPath: "next to edited Qwen Code sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback: "Remove only the managed Qwen Code Switchboard sidecar dossier.",
  },
  {
    id: "amazon-q-routing",
    kind: "client_config",
    owner: "Amazon Q Developer CLI routing",
    text: "Managed Amazon Q Developer CLI Switchboard sidecar dossier.",
    paths: ["~/.aws/amazonq/mac-ai-switchboard-routing.md"],
    markerId: "headroom:amazon_q",
    backupPath: "next to edited Amazon Q sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback:
      "Remove only the managed Amazon Q sidecar; AWS credentials, SSO cache, and profiles are not modified.",
  },
  {
    id: "windsurf-routing",
    kind: "client_config",
    owner: "Windsurf routing",
    text: "Managed Windsurf Switchboard sidecar dossier.",
    paths: [
      "~/Library/Application Support/Windsurf/mac-ai-switchboard-routing.md",
    ],
    markerId: "headroom:windsurf",
    backupPath: "next to edited Windsurf sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback: "Remove only the managed Windsurf Switchboard sidecar dossier.",
  },
  {
    id: "zed-ai-routing",
    kind: "client_config",
    owner: "Zed AI routing",
    text: "Managed Zed AI Switchboard sidecar dossier.",
    paths: ["~/.config/zed/mac-ai-switchboard-routing.md"],
    markerId: "headroom:zed_ai",
    backupPath: "next to edited Zed AI sidecar as *.headroom.bak",
    lastVerifiedLabel: "Verified by Doctor connector checks",
    rollback: "Remove only the managed Zed AI Switchboard sidecar dossier.",
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
    dryRunOnly: true,
    requiresExplicitConfirmation: true,
    confirmationPhrase: `Apply ${record.markerId} to ${trimmedPath}`,
    applyBlockedReason:
      "Apply is blocked until the user confirms this exact target, backup path, marker, rollback plan, and Off-mode cleanup boundary.",
    writePathStatus: "blocked",
    offModeCleanupBoundary:
      "Off mode cleanup may remove only the marked Switchboard block and must preserve unrelated user config.",
    currentManagedBlock: currentManagedBlock?.trim() || null,
    proposedManagedBlock: trimmedProposedBlock,
    rollback: record.rollback,
    unmanagedConfigPolicy:
      "Preserve unmanaged user config outside the marked Switchboard block.",
    safetyNotes: [
      "This dry-run report does not modify files.",
      "Review this dry-run diff before applying changes.",
      "Back up the target config before writing the proposed managed block.",
      "Off mode must remove only Switchboard-owned marked changes.",
      "Unmanaged user config outside the marker must remain untouched.",
    ],
  };
}

function markerBounds(markerId: string) {
  return {
    start: `# >>> ${markerId} >>>`,
    end: `# <<< ${markerId} <<<`,
  };
}

export function applyManagedConfigBlock(
  existingText: string,
  markerId: string,
  proposedManagedBlock: string,
): ManagedConfigTextResult {
  const trimmedBlock = proposedManagedBlock.trim();
  if (!trimmedBlock) {
    throw new Error("proposedManagedBlock is required.");
  }

  const { start, end } = markerBounds(markerId);
  const normalizedBlock = `${trimmedBlock}\n`;
  const startIndex = existingText.indexOf(start);
  if (startIndex === -1) {
    const separator = existingText.length > 0 && !existingText.endsWith("\n") ? "\n" : "";
    return {
      text: `${existingText}${separator}${normalizedBlock}`,
      changed: true,
    };
  }

  const endIndex = existingText.indexOf(end, startIndex);
  if (endIndex === -1) {
    throw new Error(`managed marker ${markerId} is missing an end marker.`);
  }
  const endWithMarker = endIndex + end.length;
  const suffix = existingText.slice(endWithMarker).replace(/^\n+/, "\n");
  const nextText = `${existingText.slice(0, startIndex)}${normalizedBlock}${suffix}`;

  return {
    text: nextText,
    changed: nextText !== existingText,
  };
}

export function buildManagedConfigApplyPlan({
  preview,
  existingText,
  confirmationPhrase,
}: {
  preview: ManagedConfigDiffPreview;
  existingText: string;
  confirmationPhrase: string;
}): ManagedConfigApplyPlan {
  if (confirmationPhrase !== preview.confirmationPhrase) {
    throw new Error("confirmation phrase does not match managed config preview.");
  }

  const result = applyManagedConfigBlock(
    existingText,
    preview.markerId,
    preview.proposedManagedBlock,
  );

  return {
    recordId: preview.recordId,
    owner: preview.owner,
    targetPath: preview.targetPath,
    markerId: preview.markerId,
    backupPath: preview.backupPath,
    confirmationPhrase: preview.confirmationPhrase,
    confirmed: true,
    writePathStatus: "ready",
    nextText: result.text,
    changed: result.changed,
    rollback: preview.rollback,
    offModeCleanupBoundary: preview.offModeCleanupBoundary,
    unmanagedConfigPolicy: preview.unmanagedConfigPolicy,
    safetyNotes: [
      "Exact user confirmation phrase matched this preview.",
      "Create the backup before writing nextText to the target path.",
      "Write only the computed nextText for this target path.",
      ...preview.safetyNotes.filter((note) => !/dry-run report/i.test(note)),
    ],
  };
}

export function removeManagedConfigBlock(
  existingText: string,
  markerId: string,
): ManagedConfigTextResult {
  const { start, end } = markerBounds(markerId);
  const startIndex = existingText.indexOf(start);
  if (startIndex === -1) {
    return { text: existingText, changed: false };
  }

  const endIndex = existingText.indexOf(end, startIndex);
  if (endIndex === -1) {
    throw new Error(`managed marker ${markerId} is missing an end marker.`);
  }
  const endWithMarker = endIndex + end.length;
  const suffix = existingText.slice(endWithMarker).replace(/^\n+/, "\n");
  const text = `${existingText.slice(0, startIndex)}${suffix}`;

  return {
    text,
    changed: text !== existingText,
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
    `Dry run only: ${preview.dryRunOnly ? "yes" : "no"}`,
    `Requires explicit confirmation: ${
      preview.requiresExplicitConfirmation ? "yes" : "no"
    }`,
    `Confirmation phrase: ${preview.confirmationPhrase}`,
    `Write path status: ${preview.writePathStatus}`,
    `Apply blocked: ${preview.applyBlockedReason}`,
    `Off-mode cleanup boundary: ${preview.offModeCleanupBoundary}`,
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
    "Unmanaged user config:",
    preview.unmanagedConfigPolicy,
    "",
    "Safety:",
    ...preview.safetyNotes.map((note) => `- ${note}`),
  ].join("\n");
}

export function formatManagedConfigApplyPlan(plan: ManagedConfigApplyPlan): string {
  return [
    `Managed config apply plan: ${plan.owner}`,
    `Target: ${plan.targetPath}`,
    `Marker: ${plan.markerId}`,
    `Backup: ${plan.backupPath}`,
    `Confirmed: ${plan.confirmed ? "yes" : "no"}`,
    `Confirmation phrase: ${plan.confirmationPhrase}`,
    `Write path status: ${plan.writePathStatus}`,
    `Changed: ${plan.changed ? "yes" : "no"}`,
    `Off-mode cleanup boundary: ${plan.offModeCleanupBoundary}`,
    "",
    "Rollback:",
    plan.rollback,
    "",
    "Unmanaged user config:",
    plan.unmanagedConfigPolicy,
    "",
    "Safety:",
    ...plan.safetyNotes.map((note) => `- ${note}`),
  ].join("\n");
}

function managedRollbackMode(record: ManagedChangeRecord): ManagedRollbackMode {
  if (record.backupPath) {
    return record.kind === "plugin" ? "managed_block_removal" : "backup_restore";
  }
  return "cleanup_inventory";
}

export function buildManagedRollbackPlan(
  record: ManagedChangeRecord,
): ManagedRollbackPlan {
  const mode = managedRollbackMode(record);
  const targetSummary =
    record.paths.length > 0
      ? record.paths.join(", ")
      : "Managed footprint discovered by add-on health checks";

  return {
    recordId: record.id,
    owner: record.owner,
    kind: record.kind,
    managedChange: record.text,
    mode,
    status: mode === "cleanup_inventory" ? "cleanup_only" : "ready_for_review",
    targetSummary,
    markerId: record.markerId,
    backupPath: record.backupPath,
    rollback: record.rollback,
    verification: record.lastVerifiedLabel,
    evidenceRequired:
      mode === "backup_restore"
        ? [
            "Confirm the target file still contains the Switchboard marker.",
            "Confirm the timestamped backup exists before restoring.",
            "Verify the post-restore diff leaves unmanaged user config unchanged.",
          ]
        : mode === "managed_block_removal"
          ? [
              "Confirm the managed add-on marker or registration exists.",
              "Remove only Switchboard-owned plugin or backup artifacts.",
              "Verify add-on health checks no longer report the managed footprint.",
            ]
          : [
              "Confirm this footprint is Switchboard-owned managed state.",
              "Use cleanup inventory instead of a config-file restore.",
              "Verify user repositories and unmanaged configs are not modified.",
            ],
    unmanagedConfigPolicy:
      "Rollback may touch only Switchboard-owned markers, backups, or managed storage listed for this record.",
    offModeCleanupBoundary:
      "Off mode cleanup must remove only Switchboard-owned changes and must not recreate routing, hooks, or agent config.",
    safetyNotes: [
      "This rollback plan does not modify files.",
      "Per-change restore stays manual until backend restore actions and fixture-home tests exist.",
      "Copy this plan before restoring so target, backup, marker, and verification evidence stay visible.",
    ],
  };
}

export function buildManagedRollbackPlans(
  records: ManagedChangeRecord[] = managedChangeRecords,
): ManagedRollbackPlan[] {
  return records.map(buildManagedRollbackPlan);
}

export function formatManagedRollbackPlan(plan: ManagedRollbackPlan): string {
  return [
    `Mac AI Switchboard rollback plan: ${plan.owner}`,
    `Kind: ${plan.kind}`,
    `Mode: ${plan.mode}`,
    `Status: ${plan.status}`,
    `Targets: ${plan.targetSummary}`,
    `Marker: ${plan.markerId}`,
    `Backup: ${plan.backupPath ?? "not required"}`,
    `Verified by: ${plan.verification}`,
    "",
    "Rollback:",
    plan.rollback,
    "",
    "Evidence required:",
    ...plan.evidenceRequired.map((item) => `- ${item}`),
    "",
    "Unmanaged user config:",
    plan.unmanagedConfigPolicy,
    "",
    "Off-mode cleanup boundary:",
    plan.offModeCleanupBoundary,
    "",
    "Safety:",
    ...plan.safetyNotes.map((note) => `- ${note}`),
  ].join("\n");
}

export function buildManagedRollbackExecutionPreview(
  record: ManagedChangeRecord,
  index = 0,
): ManagedRollbackExecutionPreview {
  const plan = buildManagedRollbackPlan(record);
  const cleanupOnly = plan.mode === "cleanup_inventory";
  const confirmationPhrase = `Restore ${record.markerId} for ${record.owner}`;

  return {
    plan,
    executionStatus: cleanupOnly
      ? "cleanup_inventory_only"
      : "blocked_until_confirmed",
    confirmationPhrase,
    undoAllOrder: index + 1,
    backendAction: cleanupOnly
      ? "cleanup_inventory_review"
      : "restore_backup_or_remove_marker",
    nativeWriteStatus: "not_executed",
    blockedReason: cleanupOnly
      ? "This record represents managed storage or app state; native cleanup must run through the dedicated uninstall or Doctor repair path."
      : "Native restore is blocked until the user confirms the exact marker, target summary, rollback mode, and verification evidence.",
    orderedSteps: cleanupOnly
      ? [
          "Show the cleanup inventory and managed ownership boundary.",
          "Require the user to choose the dedicated cleanup or uninstall flow.",
          "Verify user repositories and unmanaged configs were not modified.",
        ]
      : [
          "Load the target file and confirm the Switchboard marker is still present.",
          "Locate the timestamped backup or compute managed-marker removal.",
          "Require the exact confirmation phrase before any write.",
          "Restore from backup or remove only the marked Switchboard block.",
          "Run connector verification and show the post-restore evidence.",
        ],
  };
}

export function buildManagedRollbackExecutionPreviews(
  records: ManagedChangeRecord[] = managedChangeRecords,
): ManagedRollbackExecutionPreview[] {
  return records.map((record, index) =>
    buildManagedRollbackExecutionPreview(record, index),
  );
}

const nativeRollbackRecordIds = new Set(["codex-routing", "opencode-routing"]);

export function buildManagedRollbackUndoAllPreview(
  records: ManagedChangeRecord[] = managedChangeRecords,
): ManagedRollbackUndoAllPreview {
  const previews = buildManagedRollbackExecutionPreviews(records);
  const executable = previews.filter((preview) =>
    nativeRollbackRecordIds.has(preview.plan.recordId),
  );
  const manual = previews.filter(
    (preview) => !nativeRollbackRecordIds.has(preview.plan.recordId),
  );

  return {
    executable,
    manual,
    blockedReason:
      "Undo all is preview-only until every managed rollback row has backend execution, relaunch-survival evidence, and explicit per-row confirmation.",
    orderedSteps: [
      "Preview every managed rollback row in stable inventory order.",
      "Execute only allowlisted native rows one at a time after their exact confirmation phrases match.",
      "Leave unsupported rows in manual review or dedicated cleanup flows.",
      "Refresh Doctor and connector verification after each native restore.",
      "Stop before any row whose marker, backup, or ownership evidence is missing.",
    ],
    safetyNotes: [
      "This undo-all preview does not modify files.",
      "Executable rows are limited to Codex and OpenCode native restore paths.",
      "Cleanup-only app state, storage, launch agents, repo indexes, and plugin footprints must use their dedicated flows.",
      "Unmanaged user config outside Switchboard markers remains out of scope.",
    ],
  };
}

export function formatManagedRollbackExecutionPreview(
  preview: ManagedRollbackExecutionPreview,
): string {
  return [
    `Mac AI Switchboard rollback execution preview: ${preview.plan.owner}`,
    `Native write status: ${preview.nativeWriteStatus}`,
    `Execution status: ${preview.executionStatus}`,
    `Undo-all order: ${preview.undoAllOrder}`,
    `Backend action: ${preview.backendAction}`,
    `Confirmation phrase: ${preview.confirmationPhrase}`,
    `Blocked reason: ${preview.blockedReason}`,
    "",
    formatManagedRollbackPlan(preview.plan),
    "",
    "Ordered restore steps:",
    ...preview.orderedSteps.map((step, index) => `${index + 1}. ${step}`),
  ].join("\n");
}

export function formatManagedRollbackUndoAllPreview(
  preview: ManagedRollbackUndoAllPreview = buildManagedRollbackUndoAllPreview(),
): string {
  return [
    "Mac AI Switchboard undo-all rollback preview",
    "Native write status: not_executed",
    `Executable native rows: ${preview.executable.length}`,
    `Manual or cleanup rows: ${preview.manual.length}`,
    `Blocked reason: ${preview.blockedReason}`,
    "",
    "Executable native order:",
    ...preview.executable.map(
      (item, index) =>
        `${index + 1}. ${item.plan.owner} (${item.plan.recordId}) — ${item.confirmationPhrase}`,
    ),
    "",
    "Manual or cleanup rows:",
    ...preview.manual.map(
      (item) =>
        `- ${item.plan.owner} (${item.plan.recordId}) — ${item.executionStatus}`,
    ),
    "",
    "Ordered undo-all steps:",
    ...preview.orderedSteps.map((step, index) => `${index + 1}. ${step}`),
    "",
    "Safety:",
    ...preview.safetyNotes.map((note) => `- ${note}`),
  ].join("\n");
}

export function formatManagedRollbackInventory(
  records: ManagedChangeRecord[] = managedChangeRecords,
): string {
  const plans = buildManagedRollbackPlans(records);
  return [
    "Mac AI Switchboard Rollback Center inventory",
    "No files are changed by this report.",
    "",
    ...plans.flatMap((plan) => [
      `## ${plan.owner}`,
      `Kind: ${plan.kind}`,
      `Mode: ${plan.mode}`,
      `Status: ${plan.status}`,
      `Managed change: ${plan.managedChange}`,
      `Targets: ${plan.targetSummary}`,
      `Marker: ${plan.markerId}`,
      `Backup: ${plan.backupPath ?? "not required"}`,
      `Verified by: ${plan.verification}`,
      `Rollback: ${plan.rollback}`,
      `Evidence required: ${plan.evidenceRequired.join(" | ")}`,
      "",
    ]),
    "Review dry-run diffs before applying config changes. Dry-run reports do not modify files and every apply requires explicit user confirmation. Off mode must remove only Switchboard-owned marked changes.",
  ]
    .join("\n")
    .trimEnd();
}
