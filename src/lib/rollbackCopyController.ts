import {
  buildManagedRollbackExecutionPreview,
  buildManagedRollbackPlan,
  buildManagedRollbackUndoAllPreview,
  formatManagedRollbackExecutionPreview,
  formatManagedRollbackInventory,
  formatManagedRollbackPlan,
  formatManagedRollbackUndoAllPreview,
  type ManagedChangeRecord,
} from "./managedChanges";

export type RollbackClipboardWriter = (text: string) => Promise<void>;

export interface RollbackCopyOptions {
  records: ManagedChangeRecord[];
  setNotice: (notice: string | null) => void;
  setTimeout: (handler: () => void, timeout: number) => unknown;
  writeText?: RollbackClipboardWriter;
}

function clearNoticeLater(
  setNotice: RollbackCopyOptions["setNotice"],
  setTimeout: RollbackCopyOptions["setTimeout"],
  delay: number,
) {
  setTimeout(() => setNotice(null), delay);
}

async function writeRollbackText(
  options: RollbackCopyOptions,
  text: string,
  successNotice: string,
  failureNotice: string,
) {
  try {
    if (!options.writeText) {
      throw new Error("Clipboard API unavailable");
    }
    await options.writeText(text);
    options.setNotice(successNotice);
    clearNoticeLater(options.setNotice, options.setTimeout, 2500);
  } catch {
    options.setNotice(failureNotice);
    clearNoticeLater(options.setNotice, options.setTimeout, 3000);
  }
}

export async function copyManagedDiffPreview(
  record: ManagedChangeRecord,
  previewText: string | null,
  options: RollbackCopyOptions,
) {
  if (!previewText) {
    options.setNotice("No config diff required for that record.");
    clearNoticeLater(options.setNotice, options.setTimeout, 2500);
    return;
  }

  await writeRollbackText(
    options,
    previewText,
    `${record.owner} dry-run copied.`,
    "Copy failed. Rollback row remains visible.",
  );
}

export async function copyManagedRollbackInventory(options: RollbackCopyOptions) {
  await writeRollbackText(
    options,
    formatManagedRollbackInventory(options.records),
    "Rollback inventory copied.",
    "Copy failed. Rollback rows remain visible.",
  );
}

export async function copyManagedRollbackUndoAllPreview(
  options: RollbackCopyOptions,
) {
  await writeRollbackText(
    options,
    formatManagedRollbackUndoAllPreview(
      buildManagedRollbackUndoAllPreview(options.records),
    ),
    "Undo-all preview copied.",
    "Copy failed. Rollback rows remain visible.",
  );
}

export async function copyManagedRollbackPlan(
  record: ManagedChangeRecord,
  options: RollbackCopyOptions,
) {
  await writeRollbackText(
    options,
    formatManagedRollbackPlan(buildManagedRollbackPlan(record)),
    `${record.owner} rollback plan copied.`,
    "Copy failed. Rollback row remains visible.",
  );
}

export async function copyManagedRollbackExecutionPreview(
  record: ManagedChangeRecord,
  index: number,
  options: RollbackCopyOptions,
) {
  await writeRollbackText(
    options,
    formatManagedRollbackExecutionPreview(
      buildManagedRollbackExecutionPreview(record, index),
    ),
    `${record.owner} execution preview copied.`,
    "Copy failed. Rollback row remains visible.",
  );
}
