import { describe, expect, it, vi } from "vitest";

import {
  copyManagedDiffPreview,
  copyManagedRollbackExecutionPreview,
  copyManagedRollbackInventory,
  copyManagedRollbackPlan,
  copyManagedRollbackUndoAllPreview,
  type RollbackCopyOptions,
} from "./rollbackCopyController";
import type { ManagedChangeRecord } from "./managedChanges";

const record: ManagedChangeRecord = {
  id: "codex",
  kind: "client_config",
  owner: "Codex",
  text: "Route Codex through Switchboard.",
  paths: ["/Users/example/.codex/config.toml"],
  markerId: "codex",
  backupPath: "/Users/example/.codex/config.toml.headroom.bak",
  lastVerifiedLabel: "Verified by Doctor",
  rollback: "Restore the sibling backup.",
};

const recordWithoutBackup: ManagedChangeRecord = {
  ...record,
  id: "runtime",
  owner: "Runtime",
  backupPath: null,
};

function options(overrides: Partial<RollbackCopyOptions> = {}) {
  return {
    records: [record],
    setNotice: vi.fn(),
    setTimeout: vi.fn((handler: () => void) => handler()),
    writeText: vi.fn(async () => undefined),
    ...overrides,
  } satisfies RollbackCopyOptions;
}

describe("rollbackCopyController", () => {
  it("copies the Rollback Center inventory", async () => {
    const setup = options();

    await copyManagedRollbackInventory(setup);

    expect(setup.writeText).toHaveBeenCalledWith(
      expect.stringContaining("AI Switchboard Rollback Center inventory"),
    );
    expect(setup.setNotice).toHaveBeenNthCalledWith(
      1,
      "Rollback inventory copied.",
    );
    expect(setup.setNotice).toHaveBeenLastCalledWith(null);
  });

  it("copies the undo-all preview", async () => {
    const setup = options();

    await copyManagedRollbackUndoAllPreview(setup);

    expect(setup.writeText).toHaveBeenCalledWith(
      expect.stringContaining("Switchboard undo-all rollback preview"),
    );
    expect(setup.setNotice).toHaveBeenNthCalledWith(
      1,
      "Undo-all preview copied.",
    );
  });

  it("copies a rollback plan for a single record", async () => {
    const setup = options();

    await copyManagedRollbackPlan(record, setup);

    expect(setup.writeText).toHaveBeenCalledWith(
      expect.stringContaining("Codex"),
    );
    expect(setup.setNotice).toHaveBeenNthCalledWith(
      1,
      "Codex rollback plan copied.",
    );
  });

  it("copies a rollback execution preview for a single record", async () => {
    const setup = options();

    await copyManagedRollbackExecutionPreview(record, 0, setup);

    expect(setup.writeText).toHaveBeenCalledWith(
      expect.stringContaining("Codex"),
    );
    expect(setup.setNotice).toHaveBeenNthCalledWith(
      1,
      "Codex execution preview copied.",
    );
  });

  it("copies a managed config diff preview when a backup exists", async () => {
    const setup = options();

    await copyManagedDiffPreview(
      record,
      "Diff preview for /Users/example/.codex/config.toml",
      setup,
    );

    expect(setup.writeText).toHaveBeenCalledWith(
      expect.stringContaining("/Users/example/.codex/config.toml"),
    );
    expect(setup.setNotice).toHaveBeenNthCalledWith(
      1,
      "Codex dry-run copied.",
    );
  });

  it("shows a notice without copying when no diff is required", async () => {
    const setup = options();

    await copyManagedDiffPreview(recordWithoutBackup, null, setup);

    expect(setup.writeText).not.toHaveBeenCalled();
    expect(setup.setNotice).toHaveBeenNthCalledWith(
      1,
      "No config diff required for that record.",
    );
    expect(setup.setNotice).toHaveBeenLastCalledWith(null);
  });

  it("keeps the rollback row visible when clipboard copy fails", async () => {
    const setup = options({
      writeText: vi.fn(async () => {
        throw new Error("denied");
      }),
    });

    await copyManagedRollbackPlan(record, setup);

    expect(setup.setNotice).toHaveBeenNthCalledWith(
      1,
      "Copy failed. Rollback row remains visible.",
    );
    expect(setup.setTimeout).toHaveBeenCalledWith(expect.any(Function), 3000);
  });

  it("shows the inventory failure copy for multi-row actions", async () => {
    const setup = options({ writeText: undefined });

    await copyManagedRollbackInventory(setup);

    expect(setup.setNotice).toHaveBeenNthCalledWith(
      1,
      "Copy failed. Rollback rows remain visible.",
    );
  });
});
