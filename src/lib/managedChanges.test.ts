import { describe, expect, it } from "vitest";
import {
  buildManagedConfigDiffPreview,
  formatManagedConfigDiffPreview,
  formatManagedRollbackInventory,
  managedChangeRecords,
} from "./managedChanges";

describe("managedChangeRecords", () => {
  it("keeps rollback-center inventory stable and app-owned", () => {
    expect(managedChangeRecords.map((record) => record.id)).toEqual([
      "claude-code-routing",
      "codex-routing",
      "managed-hooks",
      "managed-storage",
      "repo-intelligence",
      "login-item",
      "app-state",
      "plugins-backups",
    ]);
    expect(
      managedChangeRecords.every((record) => record.rollback.length > 0),
    ).toBe(true);
  });

  it("tracks marker, backup, and verification evidence for every managed change", () => {
    expect(managedChangeRecords.every((record) => record.markerId.length > 0)).toBe(
      true,
    );
    expect(
      managedChangeRecords.every((record) => record.lastVerifiedLabel.length > 0),
    ).toBe(true);
    expect(
      managedChangeRecords
        .filter((record) => record.kind === "client_config" || record.kind === "shell_hook")
        .every((record) => record.backupPath?.includes("*.headroom.bak")),
    ).toBe(true);
  });

  it("covers reversible config storage footprint", () => {
    const allCopy = managedChangeRecords
      .flatMap((record) => [
        record.owner,
        record.text,
        record.rollback,
        record.markerId,
        record.backupPath ?? "",
        record.lastVerifiedLabel,
        ...record.paths,
      ])
      .join(" ");

    expect(allCopy).toContain("Claude Code");
    expect(allCopy).toContain("Codex");
    expect(allCopy).toContain("headroom:claude_code");
    expect(allCopy).toContain("headroom:codex_cli");
    expect(allCopy).toContain("~/Library/Application Support/Headroom");
    expect(allCopy).toContain("Repo Intelligence");
    expect(allCopy).toContain("User repositories are not modified");
    expect(allCopy).toContain("~/Library/LaunchAgents/");
    expect(allCopy).toContain("Keychain");
    expect(allCopy).toContain("Ponytail");
    expect(allCopy).toContain("*.headroom.bak");
  });

  it("builds a safe dry-run diff preview for managed config edits", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "codex-routing",
    );
    expect(record).toBeDefined();

    const preview = buildManagedConfigDiffPreview({
      record: record!,
      targetPath: " ~/.codex/config.toml ",
      currentManagedBlock: " # >>> headroom:codex_cli >>>\nold\n# <<< headroom:codex_cli <<< ",
      proposedManagedBlock:
        "# >>> headroom:codex_cli >>>\nnew\n# <<< headroom:codex_cli <<<",
    });

    expect(preview).toMatchObject({
      recordId: "codex-routing",
      owner: "Codex routing",
      targetPath: "~/.codex/config.toml",
      markerId: "headroom:codex_cli",
      backupPath: "next to edited client config as *.headroom.bak",
      dryRunOnly: true,
      requiresExplicitConfirmation: true,
      confirmationPhrase: "Apply headroom:codex_cli to ~/.codex/config.toml",
      unmanagedConfigPolicy:
        "Preserve unmanaged user config outside the marked Switchboard block.",
    });
    expect(preview.currentManagedBlock).toContain("old");
    expect(preview.proposedManagedBlock).toContain("new");
    expect(preview.safetyNotes.join(" ")).toContain("dry-run diff");
    expect(preview.safetyNotes.join(" ")).toContain("Off mode");
    expect(preview.safetyNotes.join(" ")).toContain("Unmanaged user config");
  });

  it("formats managed config diff previews for review before writes", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "managed-hooks",
    )!;
    const text = formatManagedConfigDiffPreview(
      buildManagedConfigDiffPreview({
        record,
        targetPath: "~/.zshrc",
        currentManagedBlock: null,
        proposedManagedBlock:
          "# >>> headroom:rtk >>>\nsource ~/.headroom/rtk.sh\n# <<< headroom:rtk <<<",
      }),
    );

    expect(text).toContain("Managed config diff: RTK shell compression");
    expect(text).toContain("Target: ~/.zshrc");
    expect(text).toContain("Dry run only: yes");
    expect(text).toContain("Requires explicit confirmation: yes");
    expect(text).toContain("Confirmation phrase: Apply headroom:rtk to ~/.zshrc");
    expect(text).toContain("Current managed block:\n(none detected)");
    expect(text).toContain("Proposed managed block:");
    expect(text).toContain("Unmanaged user config:");
    expect(text).toContain(
      "Preserve unmanaged user config outside the marked Switchboard block.",
    );
    expect(text).toContain("Backup: next to edited shell profile as *.headroom.bak");
    expect(text).toContain("Off mode must remove only Switchboard-owned");
  });

  it("formats a complete rollback-center inventory for support handoff", () => {
    const text = formatManagedRollbackInventory();

    expect(text).toContain("Mac AI Switchboard Rollback Center inventory");
    expect(text).toContain("No files are changed by this report.");
    expect(text).toContain("## Claude Code routing");
    expect(text).toContain("Paths: ~/.zshrc, ~/.zprofile");
    expect(text).toContain("Marker: headroom:codex_cli");
    expect(text).toContain("Backup: next to edited shell profile as *.headroom.bak");
    expect(text).toContain("Verified by: Verified by Doctor connector checks");
    expect(text).toContain("Rollback: Remove managed Codex shell routing");
    expect(text).toContain("Repo Intelligence");
    expect(text).toContain("Launch at login");
    expect(text).toContain("Dry-run reports do not modify files");
    expect(text).toContain("every apply requires explicit user confirmation");
    expect(text).toContain("Off mode must remove only Switchboard-owned");
  });

  it("rejects diff previews for non-config records or missing inputs", () => {
    const storage = managedChangeRecords.find(
      (candidate) => candidate.id === "managed-storage",
    )!;
    expect(() =>
      buildManagedConfigDiffPreview({
        record: storage,
        targetPath: "~/Library/Application Support/Headroom",
        proposedManagedBlock: "managed",
      }),
    ).toThrow("does not require a config backup");

    const clientHooks = managedChangeRecords.find(
      (candidate) => candidate.id === "codex-routing",
    )!;
    expect(() =>
      buildManagedConfigDiffPreview({
        record: clientHooks,
        targetPath: "",
        proposedManagedBlock: "managed",
      }),
    ).toThrow("targetPath is required");
    expect(() =>
      buildManagedConfigDiffPreview({
        record: clientHooks,
        targetPath: "~/.codex/config.toml",
        proposedManagedBlock: " ",
      }),
    ).toThrow("proposedManagedBlock is required");
  });
});
