import { managedChangeRecords } from "./managedChanges";
import type { UninstallDryRunReport } from "./types";

export interface UninstallDisclosureItem {
  id: string;
  text: string;
  paths: string[];
  markerId: string;
  backupPath: string | null;
}

export const uninstallDisclosureTitle = "Uninstall AI Switchboard for Mac?";

export const uninstallCleanupDisclosureItems: UninstallDisclosureItem[] = [
  {
    id: "macos-current-bundle-data",
    text: "Remove current AI Switchboard for Mac preferences, caches, WebKit data, HTTP storage, saved state, logs, and launch agent.",
    paths: [
      "~/Library/Preferences/com.tarunagarwal.mac-ai-switchboard.plist",
      "~/Library/Caches/com.tarunagarwal.mac-ai-switchboard",
      "~/Library/WebKit/com.tarunagarwal.mac-ai-switchboard",
      "~/Library/HTTPStorages/com.tarunagarwal.mac-ai-switchboard",
      "~/Library/Saved Application State/com.tarunagarwal.mac-ai-switchboard.savedState",
      "~/Library/Logs/Mac AI Switchboard",
      "~/Library/LaunchAgents/com.tarunagarwal.mac-ai-switchboard.plist",
    ],
    markerId: "bundle-id:com.tarunagarwal.mac-ai-switchboard",
    backupPath: null,
  },
  {
    id: "keychain-entries",
    text: "Remove Switchboard-owned Keychain entries without exposing or exporting secret values.",
    paths: [
      "keychain://com.tarunagarwal.mac-ai-switchboard.account/session-token",
      "keychain://com.tarunagarwal.mac-ai-switchboard.device/machine-id-digest",
    ],
    markerId: "keychain-services",
    backupPath: null,
  },
];

export const uninstallDisclosureItems: UninstallDisclosureItem[] =
  managedChangeRecords
    .map((record) => ({
      id: record.id,
      text: record.rollback,
      paths: record.paths,
      markerId: record.markerId,
      backupPath: record.backupPath,
    }))
    .concat(uninstallCleanupDisclosureItems);

export const uninstallDisclosureFooter =
  "You can reinstall later by launching AI Switchboard for Mac again. Use Off mode instead if you only want to stop routing without deleting runtime files.";

export function formatUninstallDryRunReport(
  items: UninstallDisclosureItem[] = uninstallDisclosureItems,
) {
  return [
    "AI Switchboard for Mac uninstall dry-run",
    "No files are changed by this report.",
    "Managed footprint source: Rollback Center inventory.",
    `Items: ${items.length}`,
    "",
    ...items.flatMap((item, index) => [
      `${index + 1}. ${item.text}`,
      `Paths: ${item.paths.length > 0 ? item.paths.join(", ") : "not required"}`,
      `Marker: ${item.markerId}`,
      `Backup: ${item.backupPath ?? "not required"}`,
      "",
    ]),
    uninstallDisclosureFooter,
  ]
    .join("\n")
    .trimEnd();
}

export function formatBackendUninstallDryRunReport(
  report: UninstallDryRunReport,
) {
  return [
    "AI Switchboard for Mac uninstall dry-run",
    "No files are changed by this report.",
    `Generated: ${report.generatedAt}`,
    `Targets: ${report.targets.length}`,
    "",
    ...report.targets.flatMap((target, index) =>
      [
        `${index + 1}. ${target.action}`,
        `Category: ${target.category}`,
        `Path: ${target.path}`,
        `Exists now: ${target.exists ? "yes" : "no"}`,
        `Managed: ${target.managed ? "yes" : "no"}`,
        `Requires confirmation: ${target.requiresConfirmation ? "yes" : "no"}`,
        target.notes.length > 0 ? `Notes: ${target.notes.join("; ")}` : null,
        "",
      ].filter((line): line is string => line !== null),
    ),
    "Preserved:",
    ...report.preserved.map((item) => `- ${item}`),
    "",
    uninstallDisclosureFooter,
  ]
    .join("\n")
    .trimEnd();
}
