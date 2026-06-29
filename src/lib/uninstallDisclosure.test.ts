import { describe, expect, it } from "vitest";

import {
  formatBackendUninstallDryRunReport,
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
    expect(allCopy).toContain("Amazon Q");
    expect(allCopy).toContain("AWS credentials, SSO cache, and profiles are not modified");
    expect(allCopy).toContain("~/Library/Application Support/Headroom");
    expect(allCopy).toContain("com.tarunagarwal.mac-ai-switchboard");
    expect(allCopy).toContain("com.extraheadroom.headroom");
    expect(allCopy).toContain("~/Library/WebKit/com.tarunagarwal.mac-ai-switchboard");
    expect(allCopy).toContain("~/Library/HTTPStorages/com.extraheadroom.headroom");
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
      "cursor-routing",
      "grok-routing",
      "aider-routing",
      "continue-routing",
      "goose-routing",
      "qwen-code-routing",
      "amazon-q-routing",
      "windsurf-routing",
      "zed-ai-routing",
      "managed-hooks",
      "managed-storage",
      "repo-intelligence",
      "login-item",
      "app-state",
      "plugins-backups",
      "macos-current-bundle-data",
      "macos-legacy-bundle-data",
      "keychain-entries",
    ]);
  });

  it("starts with rollback center inventory and appends uninstall-only cleanup targets", () => {
    expect(
      uninstallDisclosureItems
        .slice(0, managedChangeRecords.length)
        .map((item) => item.id),
    ).toEqual(managedChangeRecords.map((record) => record.id));
    expect(uninstallDisclosureItems.length).toBeGreaterThan(
      managedChangeRecords.length,
    );
  });

  it("formats a copyable uninstall dry-run report from the managed footprint", () => {
    const report = formatUninstallDryRunReport();

    expect(report).toContain("Mac AI Switchboard uninstall dry-run");
    expect(report).toContain("No files are changed by this report.");
    expect(report).toContain("Managed footprint source: Rollback Center inventory.");
    expect(report).toContain(`Items: ${uninstallDisclosureItems.length}`);
    expect(report).toContain("Remove managed Claude Code shell routing");
    expect(report).toContain("Remove managed Codex shell routing");
    expect(report).toContain("~/.codex/config.toml");
    expect(report).toContain("Marker: headroom:codex_cli");
    expect(report).toContain("Backup: next to edited client config as *.headroom.bak");
    expect(report).toContain("Marker: repo-intelligence-latest.json");
    expect(report).toContain("Backup: not required");
    expect(report).toContain("~/Library/Application Support/Headroom");
    expect(report).toContain("bundle-id:com.tarunagarwal.mac-ai-switchboard");
    expect(report).toContain(uninstallDisclosureFooter);
  });

  it("formats backend uninstall dry-run reports with exact target evidence", () => {
    const report = formatBackendUninstallDryRunReport({
      generatedAt: "2026-06-29T00:00:00Z",
      removedOnUninstall: [
        "/Users/test/Library/Application Support/Mac AI Switchboard",
      ],
      preserved: ["User repositories and source files are never deleted."],
      targets: [
        {
          id: "app-support-current",
          category: "app-storage",
          path: "/Users/test/Library/Application Support/Mac AI Switchboard",
          exists: true,
          managed: true,
          action: "Delete Mac AI Switchboard app support storage after explicit uninstall confirmation.",
          requiresConfirmation: true,
          notes: ["Contains local runtime state."],
        },
      ],
    });

    expect(report).toContain("Generated: 2026-06-29T00:00:00Z");
    expect(report).toContain("Targets: 1");
    expect(report).toContain("Category: app-storage");
    expect(report).toContain("Exists now: yes");
    expect(report).toContain("Requires confirmation: yes");
    expect(report).toContain("User repositories and source files are never deleted.");
  });
});
