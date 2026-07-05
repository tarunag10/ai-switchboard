import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

import { describeInvokeError } from "../lib/appHelpers";
import { sampleManagedBlock } from "../lib/appSupport";
import {
  copyManagedDiffPreview as copyManagedDiffPreviewController,
  copyManagedRollbackExecutionPreview as copyManagedRollbackExecutionPreviewController,
  copyManagedRollbackInventory as copyManagedRollbackInventoryController,
  copyManagedRollbackPlan as copyManagedRollbackPlanController,
  copyManagedRollbackUndoAllPreview as copyManagedRollbackUndoAllPreviewController,
  type RollbackCopyOptions,
} from "../lib/rollbackCopyController";
import {
  buildManagedConfigDiffPreview,
  buildManagedRollbackExecutionPreview,
  buildManagedRollbackPlan,
  buildManagedRollbackUndoAllPreview,
  canExecuteNativeManagedRollbackPreview,
  formatManagedConfigDiffPreview,
  formatManagedRollbackUndoAllPreview,
  managedChangeRecords,
  supportsDedicatedCleanupRollbackRecord,
  type ManagedChangeRecord,
} from "../lib/managedChanges";
import {
  firstManagedConfigTarget,
  supportsNativeConfigApply,
  supportsNativeManagedRollback,
} from "../lib/settingsConnectorCopy";
import type {
  ManagedConfigApplyPreview,
  ManagedConfigApplyResult,
  ManagedRollbackExecutionResult,
  ManagedRollbackPreview,
  ManagedRollbackUndoAllExecutionResult,
  ManagedRollbackUndoAllPreview,
} from "../lib/types";

export function RollbackCenter() {
  const [rollbackCopyNotice, setRollbackCopyNotice] = useState<string | null>(
    null,
  );
  const [rollbackPreviewByRecord, setRollbackPreviewByRecord] = useState<
    Record<string, ManagedRollbackPreview>
  >({});
  const [rollbackResultByRecord, setRollbackResultByRecord] = useState<
    Record<string, ManagedRollbackExecutionResult>
  >({});
  const [rollbackConfirmationByRecord, setRollbackConfirmationByRecord] =
    useState<Record<string, string>>({});
  const [rollbackBusyRecord, setRollbackBusyRecord] = useState<string | null>(
    null,
  );
  const [rollbackErrorByRecord, setRollbackErrorByRecord] = useState<
    Record<string, string>
  >({});
  const [configApplyPreviewByRecord, setConfigApplyPreviewByRecord] = useState<
    Record<string, ManagedConfigApplyPreview>
  >({});
  const [configApplyResultByRecord, setConfigApplyResultByRecord] = useState<
    Record<string, ManagedConfigApplyResult>
  >({});
  const [configApplyConfirmationByRecord, setConfigApplyConfirmationByRecord] =
    useState<Record<string, string>>({});
  const [configApplyBusyRecord, setConfigApplyBusyRecord] = useState<
    string | null
  >(null);
  const [configApplyErrorByRecord, setConfigApplyErrorByRecord] = useState<
    Record<string, string>
  >({});
  const [rollbackUndoAllPreview, setRollbackUndoAllPreview] =
    useState<ManagedRollbackUndoAllPreview | null>(null);
  const [rollbackUndoAllResult, setRollbackUndoAllResult] =
    useState<ManagedRollbackUndoAllExecutionResult | null>(null);
  const [rollbackUndoAllConfirmation, setRollbackUndoAllConfirmation] =
    useState("");
  const [rollbackUndoAllBusy, setRollbackUndoAllBusy] = useState(false);
  const [rollbackUndoAllError, setRollbackUndoAllError] = useState<string | null>(
    null,
  );

  function rollbackCopyOptions(): RollbackCopyOptions {
    return {
      records: managedChangeRecords,
      setNotice: setRollbackCopyNotice,
      setTimeout: window.setTimeout.bind(window),
      writeText: navigator.clipboard?.writeText.bind(navigator.clipboard),
    };
  }

  async function copyManagedDiffPreview(record: ManagedChangeRecord) {
    const previewText = record.backupPath
      ? formatManagedConfigDiffPreview(
          buildManagedConfigDiffPreview({
            record,
            targetPath: firstManagedConfigTarget(record),
            currentManagedBlock: null,
            proposedManagedBlock: sampleManagedBlock(record),
          }),
        )
      : null;
    await copyManagedDiffPreviewController(
      record,
      previewText,
      rollbackCopyOptions(),
    );
  }

  async function copyManagedRollbackInventory() {
    await copyManagedRollbackInventoryController(rollbackCopyOptions());
  }

  async function copyManagedRollbackUndoAllPreview() {
    await copyManagedRollbackUndoAllPreviewController(rollbackCopyOptions());
  }

  async function previewNativeRollbackUndoAll() {
    setRollbackUndoAllBusy(true);
    setRollbackUndoAllError(null);
    try {
      const preview = await invoke<ManagedRollbackUndoAllPreview>(
        "preview_managed_rollback_undo_all",
      );
      setRollbackUndoAllPreview(preview);
      setRollbackUndoAllResult(null);
      setRollbackUndoAllConfirmation("");
    } catch (error) {
      setRollbackUndoAllError(
        describeInvokeError(error, "Could not preview native undo-all."),
      );
    } finally {
      setRollbackUndoAllBusy(false);
    }
  }

  async function executeNativeRollbackUndoAll() {
    if (
      !rollbackUndoAllPreview ||
      rollbackUndoAllPreview.status !== "ready" ||
      rollbackUndoAllConfirmation !== rollbackUndoAllPreview.confirmationPhrase
    ) {
      return;
    }
    setRollbackUndoAllBusy(true);
    setRollbackUndoAllError(null);
    try {
      const result = await invoke<ManagedRollbackUndoAllExecutionResult>(
        "execute_managed_rollback_undo_all",
        { confirmationPhrase: rollbackUndoAllConfirmation },
      );
      setRollbackUndoAllResult(result);
      setRollbackCopyNotice(
        `Undo-all executed ${result.executed.length} native row${
          result.executed.length === 1 ? "" : "s"
        }.`,
      );
      window.setTimeout(() => setRollbackCopyNotice(null), 3000);
    } catch (error) {
      setRollbackUndoAllError(
        describeInvokeError(error, "Could not execute native undo-all."),
      );
    } finally {
      setRollbackUndoAllBusy(false);
    }
  }

  async function copyManagedRollbackPlan(record: ManagedChangeRecord) {
    await copyManagedRollbackPlanController(record, rollbackCopyOptions());
  }

  async function copyManagedRollbackExecutionPreview(
    record: ManagedChangeRecord,
    index: number,
  ) {
    await copyManagedRollbackExecutionPreviewController(
      record,
      index,
      rollbackCopyOptions(),
    );
  }

  async function previewManagedConfigApply(record: ManagedChangeRecord) {
    if (!supportsNativeConfigApply(record)) {
      return;
    }
    setConfigApplyBusyRecord(record.id);
    setConfigApplyErrorByRecord((current) => {
      const next = { ...current };
      delete next[record.id];
      return next;
    });
    try {
      const preview = await invoke<ManagedConfigApplyPreview>(
        "preview_managed_config_apply",
        { recordId: record.id },
      );
      setConfigApplyPreviewByRecord((current) => ({
        ...current,
        [record.id]: preview,
      }));
      setConfigApplyResultByRecord((current) => {
        const next = { ...current };
        delete next[record.id];
        return next;
      });
      setConfigApplyConfirmationByRecord((current) => ({
        ...current,
        [record.id]: "",
      }));
    } catch (error) {
      setConfigApplyErrorByRecord((current) => ({
        ...current,
        [record.id]: describeInvokeError(
          error,
          "Could not preview safe config apply.",
        ),
      }));
    } finally {
      setConfigApplyBusyRecord(null);
    }
  }

  async function executeManagedConfigApply(record: ManagedChangeRecord) {
    const preview = configApplyPreviewByRecord[record.id];
    const confirmation = configApplyConfirmationByRecord[record.id] ?? "";
    if (
      !preview ||
      preview.status !== "ready" ||
      confirmation !== preview.confirmationPhrase ||
      configApplyBusyRecord === record.id
    ) {
      return;
    }
    setConfigApplyBusyRecord(record.id);
    setConfigApplyErrorByRecord((current) => {
      const next = { ...current };
      delete next[record.id];
      return next;
    });
    try {
      const result = await invoke<ManagedConfigApplyResult>(
        "execute_managed_config_apply",
        {
          recordId: record.id,
          confirmationPhrase: confirmation,
        },
      );
      setConfigApplyResultByRecord((current) => ({
        ...current,
        [record.id]: result,
      }));
      setRollbackCopyNotice(`${record.owner} config apply executed.`);
      window.setTimeout(() => setRollbackCopyNotice(null), 2500);
      void previewManagedRollback(record);
    } catch (error) {
      setConfigApplyErrorByRecord((current) => ({
        ...current,
        [record.id]: describeInvokeError(
          error,
          "Could not apply managed config.",
        ),
      }));
    } finally {
      setConfigApplyBusyRecord(null);
    }
  }

  async function previewManagedRollback(record: ManagedChangeRecord) {
    if (!supportsNativeManagedRollback(record)) {
      return;
    }
    setRollbackBusyRecord(record.id);
    setRollbackErrorByRecord((current) => {
      const next = { ...current };
      delete next[record.id];
      return next;
    });
    try {
      const preview = await invoke<ManagedRollbackPreview>(
        supportsDedicatedCleanupRollbackRecord(record.id)
          ? "preview_dedicated_cleanup_rollback"
          : "preview_managed_rollback",
        { recordId: record.id },
      );
      setRollbackPreviewByRecord((current) => ({
        ...current,
        [record.id]: preview,
      }));
      setRollbackResultByRecord((current) => {
        const next = { ...current };
        delete next[record.id];
        return next;
      });
    } catch (error) {
      setRollbackErrorByRecord((current) => ({
        ...current,
        [record.id]: describeInvokeError(
          error,
          "Could not preview native rollback.",
        ),
      }));
    } finally {
      setRollbackBusyRecord(null);
    }
  }

  async function executeManagedRollback(record: ManagedChangeRecord) {
    const preview = rollbackPreviewByRecord[record.id];
    if (
      !canExecuteNativeManagedRollbackPreview({
        preview,
        confirmation: rollbackConfirmationByRecord[record.id] ?? "",
        busy: rollbackBusyRecord === record.id,
      })
    ) {
      return;
    }
    setRollbackBusyRecord(record.id);
    setRollbackErrorByRecord((current) => {
      const next = { ...current };
      delete next[record.id];
      return next;
    });
    try {
      const result = await invoke<ManagedRollbackExecutionResult>(
        supportsDedicatedCleanupRollbackRecord(record.id)
          ? "execute_dedicated_cleanup_rollback"
          : "execute_managed_rollback",
        supportsDedicatedCleanupRollbackRecord(record.id)
          ? {
              recordId: record.id,
              confirmationPhrase: rollbackConfirmationByRecord[record.id] ?? "",
            }
          : {
              recordId: record.id,
              backupPath: preview.backupPath ?? "",
              confirmationPhrase: rollbackConfirmationByRecord[record.id] ?? "",
            },
      );
      setRollbackResultByRecord((current) => ({
        ...current,
        [record.id]: result,
      }));
      setRollbackCopyNotice(`${record.owner} rollback executed.`);
      window.setTimeout(() => setRollbackCopyNotice(null), 2500);
    } catch (error) {
      setRollbackErrorByRecord((current) => ({
        ...current,
        [record.id]: describeInvokeError(error, "Could not restore from backup."),
      }));
    } finally {
      setRollbackBusyRecord(null);
    }
  }

  return (
            <article
              className="soft-card panel-card rollback-center-card"
              id="rollback-center"
            >
              <div className="panel-card__header">
                <div>
                  <h3>Rollback Center</h3>
                  <p>
                    Managed local changes Switchboard can disclose or
                    undo with guarded restore or cleanup previews.
                  </p>
                </div>
                <div className="rollback-center-card__actions">
                  <button
                    className="secondary-button secondary-button--small"
                    disabled={rollbackUndoAllBusy}
                    onClick={() => void previewNativeRollbackUndoAll()}
                    type="button"
                  >
                    Preview native undo-all
                  </button>
                  <button
                    className="secondary-button secondary-button--small"
                    onClick={() => void copyManagedRollbackUndoAllPreview()}
                    type="button"
                  >
                    Copy undo-all preview
                  </button>
                  <button
                    className="secondary-button secondary-button--small"
                    onClick={() => void copyManagedRollbackInventory()}
                    type="button"
                  >
                    Copy inventory
                  </button>
                </div>
              </div>
              {rollbackUndoAllPreview ? (
                <div className="rollback-center-card__native">
                  <div className="rollback-center-card__native-row">
                    <span>
                      Native undo-all:{" "}
                      {rollbackUndoAllPreview.ready.length} ready,{" "}
                      {rollbackUndoAllPreview.blocked.length} blocked
                    </span>
                    {rollbackUndoAllResult ? (
                      <span>
                        Executed {rollbackUndoAllResult.executed.length}; left{" "}
                        {rollbackUndoAllResult.blocked.length} blocked
                      </span>
                    ) : null}
                  </div>
                  <label className="rollback-center-card__confirm">
                    <span>Exact undo-all confirmation</span>
                    <input
                      type="text"
                      value={rollbackUndoAllConfirmation}
                      placeholder={rollbackUndoAllPreview.confirmationPhrase}
                      onChange={(event) =>
                        setRollbackUndoAllConfirmation(event.target.value)
                      }
                    />
                  </label>
                  <button
                    className="secondary-button secondary-button--small rollback-center-card__restore-button"
                    disabled={
                      rollbackUndoAllBusy ||
                      rollbackUndoAllPreview.status !== "ready" ||
                      rollbackUndoAllConfirmation !==
                        rollbackUndoAllPreview.confirmationPhrase
                    }
                    onClick={() => void executeNativeRollbackUndoAll()}
                    type="button"
                  >
                    Execute native undo-all
                  </button>
                </div>
              ) : null}
              {rollbackUndoAllError ? (
                <p className="rollback-center-card__notice">
                  {rollbackUndoAllError}
                </p>
              ) : null}
              <div className="rollback-center-card__list">
                {managedChangeRecords.map((record, index) => {
                  const plan = buildManagedRollbackPlan(record);
                  const executionPreview = buildManagedRollbackExecutionPreview(
                    record,
                    index,
                  );
                  const nativePreview = rollbackPreviewByRecord[record.id];
                  const nativeResult = rollbackResultByRecord[record.id];
                  const rollbackError = rollbackErrorByRecord[record.id];
                  const applyPreview = configApplyPreviewByRecord[record.id];
                  const applyResult = configApplyResultByRecord[record.id];
                  const applyError = configApplyErrorByRecord[record.id];
                  const applyConfirmation =
                    configApplyConfirmationByRecord[record.id] ?? "";
                  const nativeApplySupported = supportsNativeConfigApply(record);
                  const canExecuteNativeApply =
                    applyPreview?.status === "ready" &&
                    applyConfirmation === applyPreview.confirmationPhrase &&
                    configApplyBusyRecord !== record.id;
                  const confirmation =
                    rollbackConfirmationByRecord[record.id] ?? "";
                  const nativeRollbackSupported =
                    supportsNativeManagedRollback(record);
                  const canExecuteNativeRollback =
                    canExecuteNativeManagedRollbackPreview({
                      preview: nativePreview,
                      confirmation,
                      busy: rollbackBusyRecord === record.id,
                    });
                  return (
                    <div className="rollback-center-card__item" key={record.id}>
                      <div>
                        <strong>{record.owner}</strong>
                        <span>{record.rollback}</span>
                        <span>Marker: {record.markerId}</span>
                        <span>Backup: {record.backupPath ?? "not required"}</span>
                        <span>{record.lastVerifiedLabel}</span>
                        <div className="rollback-center-card__evidence">
                          <span>Mode: {plan.mode.replace(/_/g, " ")}</span>
                          <span>Status: {plan.status.replace(/_/g, " ")}</span>
                          <span>
                            Evidence: {plan.evidenceRequired[0]}
                          </span>
                          <span>
                            Native restore:{" "}
                            {executionPreview.executionStatus.replace(
                              /_/g,
                              " ",
                            )}
                          </span>
                          <span>
                            Confirm: {executionPreview.confirmationPhrase}
                          </span>
                        </div>
                        <div className="rollback-center-card__diff">
                          {record.backupPath ? (
                            <>
                              <span>
                                Dry-run target: {firstManagedConfigTarget(record)}
                              </span>
                              <button
                                className="secondary-button secondary-button--small"
                                onClick={() => void copyManagedDiffPreview(record)}
                                type="button"
                              >
                                Copy dry-run diff
                              </button>
                            </>
                          ) : null}
                          <button
                            className="secondary-button secondary-button--small"
                            onClick={() => void copyManagedRollbackPlan(record)}
                            type="button"
                          >
                            Copy rollback plan
                          </button>
                          <button
                            className="secondary-button secondary-button--small"
                            onClick={() =>
                              void copyManagedRollbackExecutionPreview(
                                record,
                                index,
                              )
                            }
                            type="button"
                          >
                            Copy execution preview
                          </button>
                        </div>
                        {nativeApplySupported ? (
                          <div className="rollback-center-card__native">
                            <div className="rollback-center-card__native-row">
                              <button
                                className="secondary-button secondary-button--small"
                                disabled={configApplyBusyRecord === record.id}
                                onClick={() =>
                                  void previewManagedConfigApply(record)
                                }
                                type="button"
                              >
                                Preview safe apply
                              </button>
                              {applyPreview ? (
                                <span>
                                  Apply status:{" "}
                                  {applyPreview.status.replace(/_/g, " ")}
                                </span>
                              ) : null}
                            </div>
                            {applyPreview ? (
                              <>
                                <span>Target: {applyPreview.targetPath}</span>
                                <span>Backup: {applyPreview.backupPath}</span>
                                <span>{applyPreview.rollbackPreview}</span>
                                {applyPreview.blockedReason ? (
                                  <span>{applyPreview.blockedReason}</span>
                                ) : null}
                                <label className="rollback-center-card__confirm">
                                  <span>Exact apply confirmation</span>
                                  <input
                                    type="text"
                                    value={applyConfirmation}
                                    placeholder={applyPreview.confirmationPhrase}
                                    onChange={(event) =>
                                      setConfigApplyConfirmationByRecord(
                                        (current) => ({
                                          ...current,
                                          [record.id]: event.target.value,
                                        }),
                                      )
                                    }
                                  />
                                </label>
                                <button
                                  className="secondary-button secondary-button--small rollback-center-card__restore-button"
                                  disabled={!canExecuteNativeApply}
                                  onClick={() =>
                                    void executeManagedConfigApply(record)
                                  }
                                  type="button"
                                >
                                  Apply {record.owner}
                                </button>
                              </>
                            ) : null}
                            {applyResult ? (
                              <span>
                                Applied: {applyResult.changed ? "changed" : "already current"};
                                backup: {applyResult.backupPath ?? "not created"}
                              </span>
                            ) : null}
                            {applyError ? <span>{applyError}</span> : null}
                          </div>
                        ) : null}
                        {nativeRollbackSupported ? (
                          <div className="rollback-center-card__native">
                            <div className="rollback-center-card__native-row">
                              <button
                                className="secondary-button secondary-button--small"
                                disabled={rollbackBusyRecord === record.id}
                                onClick={() => void previewManagedRollback(record)}
                                type="button"
                              >
                                Preview native rollback
                              </button>
                              {nativePreview ? (
                                <span>
                                  Native status:{" "}
                                  {nativePreview.status.replace(/_/g, " ")}
                                </span>
                              ) : null}
                            </div>
                            {nativePreview ? (
                              <>
                                <span>Target: {nativePreview.targetPath}</span>
                                <span>
                                  Backup:{" "}
                                  {nativePreview.backupPath ?? "not found"}
                                </span>
                                <span>
                                  Marker present:{" "}
                                  {nativePreview.markerPresent ? "yes" : "no"}
                                </span>
                                {nativePreview.blockedReason ? (
                                  <span>{nativePreview.blockedReason}</span>
                                ) : null}
                                <label className="rollback-center-card__confirm">
                                  <span>Exact confirmation</span>
                                  <input
                                    type="text"
                                    value={confirmation}
                                    placeholder={nativePreview.confirmationPhrase}
                                    onChange={(event) =>
                                      setRollbackConfirmationByRecord(
                                        (current) => ({
                                          ...current,
                                          [record.id]: event.target.value,
                                        }),
                                      )
                                    }
                                  />
                                </label>
                                <button
                                  className="secondary-button secondary-button--small rollback-center-card__restore-button"
                                  disabled={!canExecuteNativeRollback}
                                  onClick={() =>
                                    void executeManagedRollback(record)
                                  }
                                  type="button"
                                >
                                  Execute rollback for {record.owner}
                                </button>
                              </>
                            ) : null}
                            {nativeResult ? (
                              <span>
                                Restored from {nativeResult.restoredFrom};
                                safety backup:{" "}
                                {nativeResult.safetyBackupPath ?? "not created"}
                              </span>
                            ) : null}
                            {rollbackError ? <span>{rollbackError}</span> : null}
                          </div>
                        ) : null}
                      </div>
                      <span className="rollback-center-card__kind">
                        {record.kind.replace(/_/g, " ")}
                      </span>
                    </div>
                  );
                })}
              </div>
              {rollbackCopyNotice ? (
                <p className="rollback-center-card__notice">
                  {rollbackCopyNotice}
                </p>
              ) : null}
            </article>
  );
}
