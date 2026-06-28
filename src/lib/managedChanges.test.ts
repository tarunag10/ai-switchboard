import { describe, expect, it } from "vitest";
import {
  applyManagedConfigBlock,
  buildManagedConfigDiffPreview,
  formatManagedConfigDiffPreview,
  formatManagedRollbackInventory,
  managedChangeRecords,
  removeManagedConfigBlock,
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
      applyBlockedReason:
        "Apply is blocked until the user confirms this exact target, backup path, marker, rollback plan, and Off-mode cleanup boundary.",
      writePathStatus: "blocked",
      offModeCleanupBoundary:
        "Off mode cleanup may remove only the marked Switchboard block and must preserve unrelated user config.",
      unmanagedConfigPolicy:
        "Preserve unmanaged user config outside the marked Switchboard block.",
    });
    expect(preview.currentManagedBlock).toContain("old");
    expect(preview.proposedManagedBlock).toContain("new");
    expect(preview.safetyNotes.join(" ")).toContain("does not modify files");
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
    expect(text).toContain("Write path status: blocked");
    expect(text).toContain(
      "Apply blocked: Apply is blocked until the user confirms this exact target, backup path, marker, rollback plan, and Off-mode cleanup boundary.",
    );
    expect(text).toContain(
      "Off-mode cleanup boundary: Off mode cleanup may remove only the marked Switchboard block and must preserve unrelated user config.",
    );
    expect(text).toContain("Current managed block:\n(none detected)");
    expect(text).toContain("Proposed managed block:");
    expect(text).toContain("Unmanaged user config:");
    expect(text).toContain("This dry-run report does not modify files.");
    expect(text).toContain(
      "Preserve unmanaged user config outside the marked Switchboard block.",
    );
    expect(text).toContain("Backup: next to edited shell profile as *.headroom.bak");
    expect(text).toContain("Off mode must remove only Switchboard-owned");
  });

  it("can produce dry-run previews for every managed config write path", () => {
    const configRecords = managedChangeRecords.filter(
      (record) => record.backupPath !== null,
    );

    expect(configRecords.map((record) => record.id)).toEqual([
      "claude-code-routing",
      "codex-routing",
      "managed-hooks",
      "plugins-backups",
    ]);

    for (const record of configRecords) {
      const targetPath = record.paths[0] ?? "~/.config/mac-ai-switchboard";
      const preview = buildManagedConfigDiffPreview({
        record,
        targetPath,
        currentManagedBlock: null,
        proposedManagedBlock: [
          `# >>> ${record.markerId} >>>`,
          "managed = true",
          `# <<< ${record.markerId} <<<`,
        ].join("\n"),
      });
      const text = formatManagedConfigDiffPreview(preview);

      expect(preview.targetPath).toBe(targetPath);
      expect(preview.backupPath).toBe(record.backupPath);
      expect(preview.writePathStatus).toBe("blocked");
      expect(preview.requiresExplicitConfirmation).toBe(true);
      expect(text).toContain(`Marker: ${record.markerId}`);
      expect(text).toContain(`Backup: ${record.backupPath}`);
      expect(text).toContain("Write path status: blocked");
      expect(text).toContain("Rollback:");
      expect(text).toContain(record.rollback);
      expect(text).toContain("Off-mode cleanup boundary:");
      expect(text).toContain("Unmanaged user config:");
    }
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

  it("applies managed config blocks without touching unmanaged user config", () => {
    const existing = [
      "export PATH=/usr/local/bin:$PATH",
      "alias ll='ls -la'",
      "",
    ].join("\n");
    const proposed = [
      "# >>> headroom:claude_code >>>",
      "export ANTHROPIC_BASE_URL=http://127.0.0.1:6767",
      "# <<< headroom:claude_code <<<",
    ].join("\n");

    const result = applyManagedConfigBlock(
      existing,
      "headroom:claude_code",
      proposed,
    );

    expect(result.changed).toBe(true);
    expect(result.text).toContain("export PATH=/usr/local/bin:$PATH");
    expect(result.text).toContain("alias ll='ls -la'");
    expect(result.text).toContain("export ANTHROPIC_BASE_URL");
    expect(result.text.match(/# >>> headroom:claude_code >>>/g)).toHaveLength(1);
  });

  it("replaces only the managed block during repair", () => {
    const existing = [
      "export EDITOR=vim",
      "# >>> headroom:codex_cli >>>",
      "old = true",
      "# <<< headroom:codex_cli <<<",
      "export VISUAL=code",
      "",
    ].join("\n");
    const proposed = [
      "# >>> headroom:codex_cli >>>",
      "new = true",
      "# <<< headroom:codex_cli <<<",
    ].join("\n");

    const result = applyManagedConfigBlock(existing, "headroom:codex_cli", proposed);

    expect(result.changed).toBe(true);
    expect(result.text).toContain("export EDITOR=vim");
    expect(result.text).toContain("export VISUAL=code");
    expect(result.text).toContain("new = true");
    expect(result.text).not.toContain("old = true");
    expect(result.text.match(/# >>> headroom:codex_cli >>>/g)).toHaveLength(1);
  });

  it("removes only the managed block for Off cleanup", () => {
    const existing = [
      "export PATH=/usr/bin:$PATH",
      "# >>> headroom:rtk >>>",
      "source ~/.headroom/rtk.sh",
      "# <<< headroom:rtk <<<",
      "export EDITOR=vim",
      "",
    ].join("\n");

    const result = removeManagedConfigBlock(existing, "headroom:rtk");

    expect(result.changed).toBe(true);
    expect(result.text).toContain("export PATH=/usr/bin:$PATH");
    expect(result.text).toContain("export EDITOR=vim");
    expect(result.text).not.toContain("source ~/.headroom/rtk.sh");
    expect(result.text).not.toContain("# >>> headroom:rtk >>>");
  });

  it("rejects broken marker blocks before apply or cleanup", () => {
    const broken = [
      "export PATH=/usr/bin:$PATH",
      "# >>> headroom:codex_cli >>>",
      "managed = true",
    ].join("\n");

    expect(() =>
      applyManagedConfigBlock(
        broken,
        "headroom:codex_cli",
        "# >>> headroom:codex_cli >>>\nnew = true\n# <<< headroom:codex_cli <<<",
      ),
    ).toThrow("missing an end marker");
    expect(() =>
      removeManagedConfigBlock(broken, "headroom:codex_cli"),
    ).toThrow("missing an end marker");
  });
});
