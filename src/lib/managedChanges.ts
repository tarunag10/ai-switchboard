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
  rollback: string;
}

export const managedChangeRecords: ManagedChangeRecord[] = [
  {
    id: "client-hooks",
    kind: "client_config",
    owner: "Headroom engine routing",
    text: "Managed routing hooks and environment changes in Claude Code and Codex config.",
    paths: [
      "~/.claude/settings.json",
      "~/.claude/settings.local.json",
      "~/.codex/config.toml",
    ],
    rollback: "Remove managed routing hooks and environment changes from Claude Code and Codex config.",
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
    rollback: "Delete managed hook scripts and shell-profile blocks created by the app.",
  },
  {
    id: "managed-storage",
    kind: "managed_storage",
    owner: "Mac AI Switchboard runtime",
    text: "Managed runtime storage, logs, caches, receipts, setup state, and legacy runtime folders.",
    paths: ["~/Library/Application Support/Headroom", "~/.headroom"],
    rollback: "Delete managed runtime storage, logs, caches, receipts, setup state, Repo Intelligence summaries, and legacy runtime folders.",
  },
  {
    id: "repo-intelligence",
    kind: "repo_index",
    owner: "Repo Intelligence",
    text: "Saved Repo Intelligence local index metadata. User repositories are not modified.",
    paths: [
      "~/Library/Application Support/Headroom/config/repo-intelligence-latest.json",
    ],
    rollback: "Remove saved Repo Intelligence local index metadata. User repositories are not modified.",
  },
  {
    id: "login-item",
    kind: "launch_agent",
    owner: "Launch at login",
    text: "App-managed LaunchAgent files and launch-at-login state.",
    paths: ["~/Library/LaunchAgents/"],
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
    rollback: "Delete app preferences, caches, logs, and known Keychain entries.",
  },
  {
    id: "plugins-backups",
    kind: "plugin",
    owner: "Add-ons",
    text: "Ponytail plugin registration and managed backup files created next to edited configs.",
    paths: [],
    rollback: "Remove Ponytail plugin registration and sweep managed backup files created next to edited configs.",
  },
];
