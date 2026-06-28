import { describe, expect, it } from "vitest";

import {
  formatUninstallDryRunReport,
  uninstallDisclosureFooter,
  uninstallDisclosureItems,
  uninstallDisclosureTitle,
} from "./uninstallDisclosure";
import { managedChangeRecords } from "./managedChanges";

describe("uninstallDisclosure", () => {
  it("uses Mac AI Switchboard product naming", () => {
    expect(uninstallDisclosureTitle).toBe("Uninstall Mac AI Switchboard?");
    expect(uninstallDisclosureFooter).toContain("Mac AI Switchboard");
    expect(uninstallDisclosureFooter).toContain("Off mode");
  });

  it("lists the reversible local footprint removed by uninstall", () => {
    const allCopy = uninstallDisclosureItems
      .flatMap((item) => [item.text, ...item.paths])
      .join(" ");

    expect(allCopy).toContain("Claude Code");
    expect(allCopy).toContain("Codex");
    expect(allCopy).toContain("~/Library/Application Support/Headroom");
    expect(allCopy).toContain("Repo Intelligence");
    expect(allCopy).toContain("repo-intelligence-latest.json");
    expect(allCopy).toContain("User repositories are not modified");
    expect(allCopy).toContain("~/.headroom");
    expect(allCopy).toContain("~/Library/LaunchAgents/");
    expect(allCopy).toContain("Keychain");
    expect(allCopy).toContain("Ponytail");
    expect(allCopy).toContain("backup files");
  });

  it("keeps stable ids for modal rendering", () => {
    expect(uninstallDisclosureItems.map((item) => item.id)).toEqual([
      "claude-code-routing",
      "codex-routing",
      "gemini-routing",
      "opencode-routing",
      "managed-hooks",
      "managed-storage",
      "repo-intelligence",
      "login-item",
      "app-state",
      "plugins-backups",
    ]);
  });

  it("derives uninstall footprint from the rollback center inventory", () => {
    expect(uninstallDisclosureItems.map((item) => item.id)).toEqual(
      managedChangeRecords.map((record) => record.id),
    );
    expect(uninstallDisclosureItems).toHaveLength(managedChangeRecords.length);
  });

  it("formats a copyable uninstall dry-run report from the managed footprint", () => {
    const report = formatUninstallDryRunReport();

    expect(report).toContain("Mac AI Switchboard uninstall dry-run");
    expect(report).toContain("No files are changed by this report.");
    expect(report).toContain("Managed footprint source: Rollback Center inventory.");
    expect(report).toContain(`Items: ${managedChangeRecords.length}`);
    expect(report).toContain("Remove managed Claude Code shell routing");
    expect(report).toContain("Remove managed Codex shell routing");
    expect(report).toContain("~/.codex/config.toml");
    expect(report).toContain("Marker: headroom:codex_cli");
    expect(report).toContain("Backup: next to edited client config as *.headroom.bak");
    expect(report).toContain("Marker: repo-intelligence-latest.json");
    expect(report).toContain("Backup: not required");
    expect(report).toContain("~/Library/Application Support/Headroom");
    expect(report).toContain(uninstallDisclosureFooter);
  });
});
