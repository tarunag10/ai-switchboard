import { managedChangeRecords } from "./managedChanges";

export interface UninstallDisclosureItem {
  id: string;
  text: string;
  paths: string[];
  markerId: string;
  backupPath: string | null;
}

export const uninstallDisclosureTitle = "Uninstall Mac AI Switchboard?";

export const uninstallDisclosureItems: UninstallDisclosureItem[] =
  managedChangeRecords.map((record) => ({
    id: record.id,
    text: record.rollback,
    paths: record.paths,
    markerId: record.markerId,
    backupPath: record.backupPath,
  }));

export const uninstallDisclosureFooter =
  "You can reinstall later by launching Mac AI Switchboard again. Use Off mode instead if you only want to stop routing without deleting runtime files.";

export function formatUninstallDryRunReport(
  items: UninstallDisclosureItem[] = uninstallDisclosureItems,
) {
  return [
    "Mac AI Switchboard uninstall dry-run",
    "No files are changed by this report.",
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
