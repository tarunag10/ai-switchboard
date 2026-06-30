import { describe, expect, it } from "vitest";
import {
  applyManagedConfigBlock,
  buildManagedConfigApplyPlan,
  buildManagedConfigDiffPreview,
  buildManagedRollbackExecutionPreview,
  buildManagedRollbackExecutionPreviews,
  buildManagedRollbackUndoAllPreview,
  canExecuteNativeManagedRollbackPreview,
  buildManagedRollbackPlan,
  buildManagedRollbackPlans,
  formatManagedConfigApplyPlan,
  formatManagedConfigDiffPreview,
  formatManagedFootprintReport,
  formatManagedRollbackExecutionPreview,
  formatManagedRollbackInventory,
  formatManagedRollbackPlan,
  formatManagedRollbackUndoAllPreview,
  managedChangeRecords,
  removeManagedConfigBlock,
  supportsDedicatedCleanupRollbackRecord,
  supportsNativeManagedRollbackRecord,
} from "./managedChanges";

describe("managedChangeRecords", () => {
  it("keeps rollback-center inventory stable and app-owned", () => {
    expect(managedChangeRecords.map((record) => record.id)).toEqual([
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
    ]);
    expect(
      managedChangeRecords.every((record) => record.rollback.length > 0),
    ).toBe(true);
    expect(supportsNativeManagedRollbackRecord("repo-intelligence")).toBe(false);
    expect(supportsDedicatedCleanupRollbackRecord("managed-storage")).toBe(true);
    expect(supportsDedicatedCleanupRollbackRecord("repo-intelligence")).toBe(true);
    expect(supportsDedicatedCleanupRollbackRecord("login-item")).toBe(true);
    expect(supportsDedicatedCleanupRollbackRecord("app-state")).toBe(true);
    expect(supportsDedicatedCleanupRollbackRecord("plugins-backups")).toBe(true);
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
    expect(allCopy).toContain("Aider");
    expect(allCopy).toContain("Amazon Q");
    expect(allCopy).toContain("AWS credentials, SSO cache, and profiles are not modified");
    expect(allCopy).toContain("headroom:claude_code");
    expect(allCopy).toContain("headroom:codex_cli");
    expect(allCopy).toContain("headroom:zed_ai");
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
    expect(text).toContain("Mode: backup_restore");
    expect(text).toContain("Status: ready_for_review");
    expect(text).toContain("Targets: ~/.zshrc, ~/.zprofile");
    expect(text).toContain("Marker: headroom:codex_cli");
    expect(text).toContain("Backup: next to edited shell profile as *.headroom.bak");
    expect(text).toContain("Verified by: Verified by Doctor connector checks");
    expect(text).toContain("Rollback: Remove managed Codex shell routing");
    expect(text).toContain("Evidence required: Confirm the target file still contains");
    expect(text).toContain("Repo Intelligence");
    expect(text).toContain("Launch at login");
    expect(text).toContain("Dry-run reports do not modify files");
    expect(text).toContain("every apply requires explicit user confirmation");
    expect(text).toContain("Off mode must remove only Switchboard-owned");
  });

  it("classifies rollback plans by restore mode and required evidence", () => {
    const plans = buildManagedRollbackPlans();
    const codex = plans.find((plan) => plan.recordId === "codex-routing");
    const repoIndex = plans.find((plan) => plan.recordId === "repo-intelligence");
    const plugins = plans.find((plan) => plan.recordId === "plugins-backups");

    expect(plans).toHaveLength(managedChangeRecords.length);
    expect(codex).toMatchObject({
      owner: "Codex routing",
      mode: "backup_restore",
      status: "ready_for_review",
      backupPath: "next to edited client config as *.headroom.bak",
    });
    expect(codex?.evidenceRequired.join(" ")).toContain(
      "timestamped backup exists",
    );
    expect(repoIndex).toMatchObject({
      mode: "cleanup_inventory",
      status: "cleanup_only",
      backupPath: null,
    });
    expect(repoIndex?.evidenceRequired.join(" ")).toContain(
      "user repositories",
    );
    expect(plugins).toMatchObject({
      mode: "managed_block_removal",
      status: "ready_for_review",
    });
  });

  it("formats per-change rollback plans for copy and support review", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "codex-routing",
    )!;
    const text = formatManagedRollbackPlan(buildManagedRollbackPlan(record));

    expect(text).toContain("Mac AI Switchboard rollback plan: Codex routing");
    expect(text).toContain("Kind: client_config");
    expect(text).toContain("Mode: backup_restore");
    expect(text).toContain("Status: ready_for_review");
    expect(text).toContain("Targets: ~/.codex/config.toml, ~/.zshrc, ~/.zprofile");
    expect(text).toContain("Marker: headroom:codex_cli");
    expect(text).toContain("Backup: next to edited client config as *.headroom.bak");
    expect(text).toContain("Evidence required:");
    expect(text).toContain("Confirm the target file still contains");
    expect(text).toContain("Unmanaged user config:");
    expect(text).toContain("Off-mode cleanup boundary:");
    expect(text).toContain("This rollback plan does not modify files.");
    expect(text).toContain("fixture-home tests");
  });

  it("builds guarded rollback execution previews without native writes", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "opencode-routing",
    )!;

    const preview = buildManagedRollbackExecutionPreview(record, 3);

    expect(preview).toMatchObject({
      executionStatus: "blocked_until_confirmed",
      confirmationPhrase: "Restore headroom:opencode for OpenCode routing",
      undoAllOrder: 4,
      backendAction: "restore_backup_or_remove_marker",
      nativeWriteStatus: "not_executed",
    });
    expect(preview.blockedReason).toContain("Native restore is blocked");
    expect(preview.orderedSteps.join(" ")).toContain(
      "Require the exact confirmation phrase",
    );
    expect(preview.orderedSteps.join(" ")).toContain(
      "remove only the marked Switchboard block",
    );
    expect(preview.plan.rollback).toContain("OpenCode");
  });

  it("keeps cleanup-only rollback execution previews out of config writes", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "repo-intelligence",
    )!;

    const preview = buildManagedRollbackExecutionPreview(record);

    expect(preview.executionStatus).toBe("cleanup_inventory_only");
    expect(preview.backendAction).toBe("cleanup_inventory_review");
    expect(preview.nativeWriteStatus).toBe("not_executed");
    expect(preview.blockedReason).toContain("dedicated uninstall or Doctor repair");
    expect(preview.orderedSteps.join(" ")).toContain("managed ownership boundary");
    expect(preview.orderedSteps.join(" ")).toContain("unmanaged configs");
  });

  it("formats execution previews with confirmation and ordered restore steps", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "codex-routing",
    )!;
    const text = formatManagedRollbackExecutionPreview(
      buildManagedRollbackExecutionPreview(record, 1),
    );

    expect(text).toContain(
      "Mac AI Switchboard rollback execution preview: Codex routing",
    );
    expect(text).toContain("Native write status: not_executed");
    expect(text).toContain("Execution status: blocked_until_confirmed");
    expect(text).toContain("Undo-all order: 2");
    expect(text).toContain("Backend action: restore_backup_or_remove_marker");
    expect(text).toContain(
      "Confirmation phrase: Restore headroom:codex_cli for Codex routing",
    );
    expect(text).toContain("Mac AI Switchboard rollback plan: Codex routing");
    expect(text).toContain("Ordered restore steps:");
    expect(text).toContain("1. Load the target file");
  });

  it("builds undo-all execution previews in stable inventory order", () => {
    const previews = buildManagedRollbackExecutionPreviews();

    expect(previews).toHaveLength(managedChangeRecords.length);
    expect(previews[0]).toMatchObject({
      undoAllOrder: 1,
      confirmationPhrase: "Restore headroom:claude_code for Claude Code routing",
    });
    expect(previews[previews.length - 1]).toMatchObject({
      undoAllOrder: managedChangeRecords.length,
      confirmationPhrase: "Restore headroom:addon for Add-ons",
    });
  });

  it("builds undo-all preview with backend-allowlisted native and sidecar rows executable", () => {
    const preview = buildManagedRollbackUndoAllPreview();

    expect(preview.executable.map((item) => item.plan.recordId)).toEqual([
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
    ]);
    expect(preview.manual.map((item) => item.plan.recordId)).not.toContain(
      "cursor-routing",
    );
    expect(preview.manual).toHaveLength(
      managedChangeRecords.length - preview.executable.length,
    );
    expect(preview.blockedReason).toContain("Native undo-all can execute");
    expect(preview.blockedReason).toContain(
      "dedicated cleanup rows use their own exact-confirmation cleanup actions",
    );
    expect(preview.safetyNotes.join(" ")).toContain("native undo-all control");
  });

  it("formats undo-all preview without claiming native writes ran", () => {
    const text = formatManagedRollbackUndoAllPreview();

    expect(text).toContain("Mac AI Switchboard undo-all rollback preview");
    expect(text).toContain("Native write status: not_executed");
    expect(text).toContain("Executable native rows: 12");
    expect(text).toContain("Codex routing (codex-routing)");
    expect(text).toContain("Gemini CLI routing (gemini-routing)");
    expect(text).toContain("OpenCode routing (opencode-routing)");
    expect(text).toContain("Cursor routing (cursor-routing)");
    expect(text).toContain("Amazon Q Developer CLI routing (amazon-q-routing)");
    expect(text).toContain("Manual or cleanup rows:");
    expect(text).toContain("Native undo-all can execute only backend-allowlisted ready rows");
    expect(text).toContain(
      "dedicated cleanup rows use their own exact-confirmation cleanup actions",
    );
    expect(text).toContain(
      "This copyable undo-all preview does not modify files; use the native undo-all control to execute ready rows.",
    );
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

  it("builds a confirmed apply plan from the reviewed dry-run preview", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "codex-routing",
    )!;
    const preview = buildManagedConfigDiffPreview({
      record,
      targetPath: "~/.codex/config.toml",
      currentManagedBlock: null,
      proposedManagedBlock: [
        "# >>> headroom:codex_cli >>>",
        '[model_providers.headroom]',
        'base_url = "http://127.0.0.1:6767/v1"',
        "# <<< headroom:codex_cli <<<",
      ].join("\n"),
    });
    const existing = [
      'model = "gpt-5"',
      "[profiles.default]",
      'approval_policy = "never"',
      "",
    ].join("\n");

    const plan = buildManagedConfigApplyPlan({
      preview,
      existingText: existing,
      confirmationPhrase: "Apply headroom:codex_cli to ~/.codex/config.toml",
    });

    expect(plan).toMatchObject({
      recordId: "codex-routing",
      owner: "Codex routing",
      targetPath: "~/.codex/config.toml",
      markerId: "headroom:codex_cli",
      confirmed: true,
      writePathStatus: "ready",
      changed: true,
    });
    expect(plan.nextText).toContain('model = "gpt-5"');
    expect(plan.nextText).toContain("[profiles.default]");
    expect(plan.nextText).toContain("[model_providers.headroom]");
    expect(plan.nextText.match(/# >>> headroom:codex_cli >>>/g)).toHaveLength(1);
    expect(plan.safetyNotes.join(" ")).toContain("Create the backup");
    expect(plan.safetyNotes.join(" ")).toContain("Off mode");
  });

  it("rejects apply plans without the exact confirmation phrase", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "managed-hooks",
    )!;
    const preview = buildManagedConfigDiffPreview({
      record,
      targetPath: "~/.zshrc",
      currentManagedBlock: null,
      proposedManagedBlock:
        "# >>> headroom:rtk >>>\nsource ~/.headroom/rtk.sh\n# <<< headroom:rtk <<<",
    });

    expect(() =>
      buildManagedConfigApplyPlan({
        preview,
        existingText: "export PATH=/usr/bin:$PATH\n",
        confirmationPhrase: "Apply RTK",
      }),
    ).toThrow("confirmation phrase does not match");
  });

  it("formats confirmed apply plans with backup rollback and cleanup evidence", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "claude-code-routing",
    )!;
    const preview = buildManagedConfigDiffPreview({
      record,
      targetPath: "~/.zshrc",
      currentManagedBlock: null,
      proposedManagedBlock: [
        "# >>> headroom:claude_code >>>",
        "export ANTHROPIC_BASE_URL=http://127.0.0.1:6767",
        "# <<< headroom:claude_code <<<",
      ].join("\n"),
    });
    const text = formatManagedConfigApplyPlan(
      buildManagedConfigApplyPlan({
        preview,
        existingText: "alias ll='ls -la'\n",
        confirmationPhrase: "Apply headroom:claude_code to ~/.zshrc",
      }),
    );

    expect(text).toContain("Managed config apply plan: Claude Code routing");
    expect(text).toContain("Target: ~/.zshrc");
    expect(text).toContain("Confirmed: yes");
    expect(text).toContain("Write path status: ready");
    expect(text).toContain("Backup: next to edited client config as *.headroom.bak");
    expect(text).toContain("Rollback:\nRemove managed Claude Code shell routing");
    expect(text).toContain("Off-mode cleanup boundary:");
    expect(text).toContain("Unmanaged user config:");
    expect(text).toContain("Exact user confirmation phrase matched");
    expect(text).toContain("Create the backup before writing nextText");
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

  it("allows native rollback execution for ready no-backup cleanup previews", () => {
    expect(
      canExecuteNativeManagedRollbackPreview({
        preview: {
          recordId: "gemini-routing",
          owner: "Gemini CLI routing",
          targetPath: "~/.gemini/mac-ai-switchboard-routing.md",
          marker: "headroom:gemini_cli",
          backupPath: null,
          markerPresent: true,
          backupExists: true,
          status: "ready",
          confirmationPhrase: "Restore headroom:gemini_cli for Gemini CLI routing",
          proposedAction:
            "Remove only the Switchboard-owned Gemini shell routing and sidecar blocks.",
          blockedReason: null,
          evidence: [],
        },
        confirmation: "Restore headroom:gemini_cli for Gemini CLI routing",
        busy: false,
      }),
    ).toBe(true);

    expect(
      canExecuteNativeManagedRollbackPreview({
        preview: {
          recordId: "cursor-routing",
          owner: "Cursor routing",
          targetPath:
            "~/Library/Application Support/Cursor/mac-ai-switchboard-routing.md",
          marker: "headroom:cursor",
          backupPath: null,
          markerPresent: true,
          backupExists: true,
          status: "ready",
          confirmationPhrase: "Restore headroom:cursor for Cursor routing",
          proposedAction:
            "Remove only the Switchboard-owned Cursor sidecar block.",
          blockedReason: null,
          evidence: [],
        },
        confirmation: "Restore headroom:cursor for Cursor routing",
        busy: false,
      }),
    ).toBe(true);
  });

  it("blocks native rollback execution when ready previews are missing backup evidence", () => {
    expect(
      canExecuteNativeManagedRollbackPreview({
        preview: {
          recordId: "codex-routing",
          owner: "Codex routing",
          targetPath: "~/.codex/config.toml",
          marker: "headroom:codex_cli",
          backupPath: null,
          markerPresent: true,
          backupExists: false,
          status: "ready",
          confirmationPhrase: "Restore headroom:codex_cli for Codex routing",
          proposedAction: "Restore the Codex config from backup.",
          blockedReason: null,
          evidence: [],
        },
        confirmation: "Restore headroom:codex_cli for Codex routing",
        busy: false,
      }),
    ).toBe(false);
  });

  it("formats managed footprint reports without secret values", () => {
    const text = formatManagedFootprintReport({
      generatedAt: "2026-06-29T00:00:00Z",
      items: [
        {
          id: "codex-config",
          category: "client_config",
          path: "~/.codex/config.toml",
          exists: true,
          managed: true,
          action: "Codex config may contain managed provider blocks.",
          reversible: true,
          backupPaths: ["*.headroom.bak next to edited config"],
          notes: ["Report does not include provider values."],
        },
        {
          id: "keychain-mac-ai-switchboard",
          category: "keychain",
          path: "Keychain service: mac-ai-switchboard",
          exists: false,
          managed: true,
          action: "Values are never reported.",
          reversible: true,
          backupPaths: [],
          notes: ["Existence is not probed."],
        },
      ],
    });

    expect(text).toContain("Mac AI Switchboard managed footprint");
    expect(text).toContain("## client_config");
    expect(text).toContain("Path: ~/.codex/config.toml");
    expect(text).toContain("Keychain service: mac-ai-switchboard");
    expect(text).toContain("No file contents, secret values, or keychain values");
    expect(text).not.toContain("sk-");
  });
});
