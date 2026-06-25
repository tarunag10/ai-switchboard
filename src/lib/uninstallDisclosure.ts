export interface UninstallDisclosureItem {
  id: string;
  text: string;
  paths: string[];
}

export const uninstallDisclosureTitle = "Uninstall Mac AI Switchboard?";

export const uninstallDisclosureItems: UninstallDisclosureItem[] = [
  {
    id: "client-hooks",
    text: "Remove managed routing hooks and environment changes from Claude Code and Codex config.",
    paths: [
      "~/.claude/settings.json",
      "~/.claude/settings.local.json",
      "~/.codex/config.toml",
    ],
  },
  {
    id: "managed-hooks",
    text: "Delete managed hook scripts and shell-profile blocks created by the app.",
    paths: [
      "~/.claude/hooks/headroom-rtk-rewrite.sh",
      "~/.zshrc",
      "~/.zprofile",
    ],
  },
  {
    id: "managed-storage",
    text: "Delete managed runtime storage, logs, caches, receipts, setup state, and legacy runtime folders.",
    paths: ["~/Library/Application Support/Headroom", "~/.headroom"],
  },
  {
    id: "login-item",
    text: "Disable launch-at-login and remove app-managed LaunchAgent files.",
    paths: ["~/Library/LaunchAgents/"],
  },
  {
    id: "app-state",
    text: "Delete app preferences, caches, logs, and known Keychain entries.",
    paths: [
      "~/Library/Preferences/com.extraheadroom.headroom*",
      "~/Library/Caches/com.extraheadroom.headroom",
    ],
  },
  {
    id: "plugins-backups",
    text: "Remove Ponytail plugin registration and sweep managed backup files created next to edited configs.",
    paths: [],
  },
];

export const uninstallDisclosureFooter =
  "You can reinstall later by launching Mac AI Switchboard again. Use Off mode instead if you only want to stop routing without deleting runtime files.";
