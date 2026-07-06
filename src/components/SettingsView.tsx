import { useState } from "react";
import { type RefObject } from "react";
import { ArrowClockwise, Copy, Terminal } from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import {
  buildManagedRollbackExecutionPreview,
  buildManagedRollbackPlan,
  canExecuteNativeManagedRollbackPreview,
  managedChangeRecords,
} from "../lib/managedChanges";
import {
  formatLocalReleaseEvidenceSequenceCopy,
  formatReleaseReadinessNextAction,
  formatReleaseReadinessSourceLabel,
  releaseReadinessCommand,
  releaseReadinessGroups,
  releaseReadinessItemCount,
  releaseShareableGates,
} from "../lib/releaseReadiness";
import {
  firstManagedConfigTarget,
  supportsNativeConfigApply,
  supportsNativeManagedRollback,
} from "../lib/settingsConnectorCopy";
import { SettingsLegalPanel } from "./SettingsLegalPanel";
import { SettingsOpenLoginCard } from "./SettingsOpenLoginCard";
import { SettingsReleaseReadinessCard } from "./SettingsReleaseReadinessCard";
import { SettingsRuntimeStatusCard } from "./SettingsRuntimeStatusCard";
import { SettingsUninstallCard } from "./SettingsUninstallCard";
import { SettingsConnectorPanel } from "./SettingsConnectorPanel";
import { SettingsFooterActions } from "./SettingsFooterActions";
import type {
  AppUpdateConfiguration,
  ClientConnectorStatus,
  DashboardState,
  ManagedConfigApplyPreview,
  ManagedConfigApplyResult,
  ManagedRollbackExecutionResult,
  ManagedRollbackPreview,
  ManagedRollbackUndoAllExecutionResult,
  ManagedRollbackUndoAllPreview,
  RuntimeStatus,
  SavingsMode,
  SwitchboardMode,
} from "../lib/types";
import type { ReleaseReadinessReportSnapshot } from "../lib/releaseReadiness";
import type { ManagedChangeRecord } from "../lib/managedChanges";

interface ReleaseEvidenceResult {
  commandId: string;
  label: string;
  command: string;
  summaryPath: string | null;
  stdout: string;
  stderr: string;
}

interface ReleaseReadinessReportPayload {
  reportPath: string;
  report: ReleaseReadinessReportSnapshot | null;
}

interface RollbackPreviewEntry {
  status: string;
  targetPath: string;
  backupPath: string | null;
  markerPresent: boolean;
  blockedReason: string | null;
  confirmationPhrase: string;
}

interface RollbackResultEntry {
  restoredFrom: string;
  safetyBackupPath: string | null;
}

interface ConfigApplyPreviewEntry {
  status: string;
  targetPath: string;
  backupPath: string;
  rollbackPreview: string;
  blockedReason: string | null;
  confirmationPhrase: string;
}

interface ConfigApplyResultEntry {
  changed: boolean;
  backupPath: string | null;
}

interface ReleaseReadinessRow {
  id: string;
  label: string;
  detail: string;
  statusLabel: string;
  tone: string;
  source: string;
}

interface ReleaseLocalEvidenceRow {
  id: string;
  label: string;
  detail: string;
  statusLabel: string;
  passed: boolean;
  command: string;
  summaryPath: string;
}

interface ReleaseReadinessGroup {
  id: string;
  title: string;
  items: {
    id: string;
    label: string;
    detail: string;
    command?: string;
    executable?: boolean;
  }[];
}

interface ReleaseShareableGate {
  id: string;
  label: string;
  detail: string;
}

interface ReleaseReadinessCounts {
  ready: number;
  blocked: number;
  "local-only": number;
}

interface PlannedConnectorReadinessSummary {
  headline: string;
  detail: string;
  detectedCount: number;
  manualOnlyCount: number;
  notDetectedCount: number;
  safeTodayCount: number;
  automationGateCount: number;
}

interface ConnectorReadinessCopyNotice {
  id: string;
  label: string;
  detail: string;
}

interface ReleaseEvidenceBusy {
  commandId: string;
  label: string;
}

export interface SettingsViewProps {
  dashboard: DashboardState;
  switchboardMode: SwitchboardMode;
  savingsMode: SavingsMode;
  connectors: ClientConnectorStatus[];
  appSemver: string;

  settingsTransferNotice: string | null;
  setSettingsImportText: (value: string) => void;
  setSettingsImportPreview: (value: null) => void;
  setSettingsTransferNotice: (value: string | null) => void;
  settingsImportText: string;
  settingsImportPreview: {
    valid: boolean;
    title: string;
    detail: string;
    errors: string[];
    safePreferences: Record<string, string>;
    migrationActions: {
      id: string;
      label: string;
      status: string;
      detail: string;
    }[];
    manualItems: string[];
  } | null;
  settingsImportBusy: boolean;
  copySettingsExport: () => Promise<void>;
  previewSettingsImport: () => void;
  applySettingsImport: () => Promise<void>;

  plannedConnectorReadiness: PlannedConnectorReadinessSummary;
  plannedConnectorCopyNotice: string | null;

  connectorsBusy: boolean;
  connectorsError: string | null;
  openConnectorHelpId: string | null;
  setOpenConnectorHelpId: React.Dispatch<React.SetStateAction<string | null>>;
  toggleConnector: (
    connector: ClientConnectorStatus,
    enabled: boolean,
  ) => Promise<void>;
  copyPlannedConnectorCommand: (
    command: string,
    connectorName: string,
  ) => Promise<void>;

  autostartEnabled: boolean | null;
  autostartBusy: boolean;
  handleAutostartToggle: (enabled: boolean) => Promise<void>;

  showHeadroomDetails: boolean;
  setShowHeadroomDetails: (value: boolean) => void;
  headroomLogLines: string[];
  headroomLogRef: RefObject<HTMLPreElement | null>;
  headroomVersion: string;
  headroomLifetimeSavingsPct: number | null;

  runtimeStatus: RuntimeStatus | null;
  kompressWarming: boolean;

  appUpdateConfig: AppUpdateConfiguration | null;
  appUpdateBusy: boolean;
  appUpdateInstallBusy: boolean;
  appUpdateStatusCopy: string | null;
  checkForAppUpdate: () => Promise<void>;

  releaseReadinessRefreshing: boolean;
  releaseEvidenceBusyId: string | null;
  releaseReadinessCommand: string;
  releaseReadinessReport: ReleaseReadinessReportPayload | null;
  releaseReadinessEvidence: { copy: string };
  releaseReadinessAction: string;
  releaseReadinessError: string | null;
  releaseReadinessCounts: ReleaseReadinessCounts;
  releaseReadinessRows: ReleaseReadinessRow[];
  releaseLocalEvidenceRows: ReleaseLocalEvidenceRow[];
  releaseShareableGates: ReleaseShareableGate[];
  releaseReadinessGroups: ReleaseReadinessGroup[];
  releaseReadinessCopyNotice: string | null;
  copyReleaseReadinessReport: () => Promise<void>;
  refreshReleaseReadinessReport: () => Promise<void>;
  runReleaseEvidenceCommand: (commandId: string) => Promise<void>;
  runLocalReleaseEvidenceSequence: () => Promise<void>;
  formatLocalReleaseEvidenceSequenceCopy: () => string;

  rollbackUndoAllBusy: boolean;
  rollbackUndoAllPreview: ManagedRollbackUndoAllPreview | null;
  rollbackUndoAllResult: ManagedRollbackUndoAllExecutionResult | null;
  rollbackUndoAllConfirmation: string;
  setRollbackUndoAllConfirmation: (value: string) => void;
  rollbackUndoAllError: string | null;
  previewNativeRollbackUndoAll: () => Promise<void>;
  executeNativeRollbackUndoAll: () => Promise<void>;
  copyManagedRollbackUndoAllPreview: () => Promise<void>;
  copyManagedRollbackInventory: () => Promise<void>;
  rollbackCopyNotice: string | null;

  rollbackPreviewByRecord: Record<string, ManagedRollbackPreview | null>;
  rollbackResultByRecord: Record<string, RollbackResultEntry | null>;
  rollbackErrorByRecord: Record<string, string | null>;
  rollbackConfirmationByRecord: Record<string, string>;
  setRollbackConfirmationByRecord: React.Dispatch<
    React.SetStateAction<Record<string, string>>
  >;
  rollbackBusyRecord: string | null;
  previewManagedRollback: (record: ManagedChangeRecord) => Promise<void>;
  executeManagedRollback: (record: ManagedChangeRecord) => Promise<void>;
  copyManagedDiffPreview: (record: ManagedChangeRecord) => Promise<void>;
  copyManagedRollbackPlan: (record: ManagedChangeRecord) => Promise<void>;
  copyManagedRollbackExecutionPreview: (
    record: ManagedChangeRecord,
    index: number,
  ) => Promise<void>;

  configApplyPreviewByRecord: Record<string, ConfigApplyPreviewEntry | null>;
  configApplyResultByRecord: Record<string, ConfigApplyResultEntry | null>;
  configApplyErrorByRecord: Record<string, string | null>;
  configApplyConfirmationByRecord: Record<string, string>;
  setConfigApplyConfirmationByRecord: React.Dispatch<
    React.SetStateAction<Record<string, string>>
  >;
  configApplyBusyRecord: string | null;
  previewManagedConfigApply: (record: ManagedChangeRecord) => Promise<void>;
  executeManagedConfigApply: (record: ManagedChangeRecord) => Promise<void>;

  uninstallBusy: boolean;
  uninstallError: string | null;
  showUninstallDialog: boolean;
  setUninstallError: (value: string | null) => void;
  setShowUninstallDialog: (value: boolean) => void;
  handleUninstall: () => Promise<void>;
  copyUninstallDryRunReport: () => Promise<void>;

  SUPPORT_ISSUES_URL: string;
}

export function SettingsView({
  dashboard,
  switchboardMode,
  savingsMode,
  connectors,
  appSemver,
  settingsTransferNotice,
  setSettingsImportText,
  setSettingsImportPreview,
  setSettingsTransferNotice,
  settingsImportText,
  settingsImportPreview,
  settingsImportBusy,
  copySettingsExport,
  previewSettingsImport,
  applySettingsImport,
  plannedConnectorReadiness,
  plannedConnectorCopyNotice,
  connectorsBusy,
  connectorsError,
  openConnectorHelpId,
  setOpenConnectorHelpId,
  toggleConnector,
  copyPlannedConnectorCommand,
  autostartEnabled,
  autostartBusy,
  handleAutostartToggle,
  showHeadroomDetails,
  setShowHeadroomDetails,
  headroomLogLines,
  headroomLogRef,
  headroomVersion,
  headroomLifetimeSavingsPct,
  runtimeStatus,
  kompressWarming,
  appUpdateConfig,
  appUpdateBusy,
  appUpdateInstallBusy,
  appUpdateStatusCopy,
  checkForAppUpdate,
  releaseReadinessRefreshing,
  releaseEvidenceBusyId,
  releaseReadinessCommand: releaseReadinessCommandProp,
  releaseReadinessReport,
  releaseReadinessEvidence,
  releaseReadinessAction,
  releaseReadinessError,
  releaseReadinessCounts,
  releaseReadinessRows,
  releaseLocalEvidenceRows,
  releaseShareableGates,
  releaseReadinessGroups,
  releaseReadinessCopyNotice,
  copyReleaseReadinessReport,
  refreshReleaseReadinessReport,
  runReleaseEvidenceCommand,
  runLocalReleaseEvidenceSequence,
  formatLocalReleaseEvidenceSequenceCopy,
  rollbackUndoAllBusy,
  rollbackUndoAllPreview,
  rollbackUndoAllResult,
  rollbackUndoAllConfirmation,
  setRollbackUndoAllConfirmation,
  rollbackUndoAllError,
  previewNativeRollbackUndoAll,
  executeNativeRollbackUndoAll,
  copyManagedRollbackUndoAllPreview,
  copyManagedRollbackInventory,
  rollbackCopyNotice,
  rollbackPreviewByRecord,
  rollbackResultByRecord,
  rollbackErrorByRecord,
  rollbackConfirmationByRecord,
  setRollbackConfirmationByRecord,
  rollbackBusyRecord,
  previewManagedRollback,
  executeManagedRollback,
  copyManagedDiffPreview,
  copyManagedRollbackPlan,
  copyManagedRollbackExecutionPreview,
  configApplyPreviewByRecord,
  configApplyResultByRecord,
  configApplyErrorByRecord,
  configApplyConfirmationByRecord,
  setConfigApplyConfirmationByRecord,
  configApplyBusyRecord,
  previewManagedConfigApply,
  executeManagedConfigApply,
  uninstallBusy,
  uninstallError,
  showUninstallDialog,
  setUninstallError,
  setShowUninstallDialog,
  handleUninstall,
  copyUninstallDryRunReport,
  SUPPORT_ISSUES_URL,
}: SettingsViewProps) {
  const [runtimeActionError, setRuntimeActionError] = useState<string | null>(
    null,
  );

  async function openHeadroomDashboard() {
    setRuntimeActionError(null);
    try {
      await invoke("open_headroom_dashboard");
    } catch (err) {
      setRuntimeActionError(
        err instanceof Error
          ? err.message
          : "Could not open the Switchboard dashboard.",
      );
    }
  }

  return (
    <div className="tray-content">
      <section className="panel-stack">
        <article className="soft-card panel-card settings-account-card">
          <div className="settings-account-row">
            <p className="settings-account-copy">
              Account and paid APIs: <em>not included</em>
            </p>
            <span className="settings-account-badge">Local-free</span>
          </div>
          <p className="settings-account-notice">
            AI Switchboard does not include remote account, billing, checkout,
            or paid pricing APIs. Provider model calls still use the accounts
            you configure in Claude, Codex, or other tools.
          </p>
        </article>

        <SettingsLegalPanel
          requiredTermsVersion={dashboard.requiredTermsVersion}
        />

        <article className="soft-card panel-card settings-transfer-card">
          <div className="panel-card__header">
            <div>
              <h3>Settings import/export</h3>
              <p>
                Move safe AI Switchboard for Mac preferences without carrying
                secrets, local paths, message logs, billing state, or token
                history.
              </p>
            </div>
          </div>
          <div className="settings-transfer__summary">
            <span>
              Mode <strong>{switchboardMode}</strong>
            </span>
            <span>
              Savings <strong>{savingsMode}</strong>
            </span>
            <span>
              Connectors <strong>{connectors.length}</strong>
            </span>
            <span>
              Add-ons{" "}
              <strong>
                {dashboard.tools.filter((tool) => !tool.required).length}
              </strong>
            </span>
          </div>
          <p className="settings-transfer__note">
            Import applies only safe app preferences. Connector and add-on
            entries are shown as approval-review items so config writes still go
            through Doctor, Addons, and connector gates.
          </p>
          <div className="settings-transfer__actions">
            <button
              className="secondary-button secondary-button--small"
              onClick={() => void copySettingsExport()}
              type="button"
            >
              Copy settings export
            </button>
            {settingsTransferNotice ? (
              <span>{settingsTransferNotice}</span>
            ) : null}
          </div>
          <textarea
            className="settings-transfer__textarea"
            onChange={(event) => {
              setSettingsImportText(event.target.value);
              setSettingsImportPreview(null);
              setSettingsTransferNotice(null);
            }}
            placeholder="Paste settings export JSON to preview safe preferences"
            rows={5}
            value={settingsImportText}
          />
          <div className="settings-transfer__actions">
            <button
              className="secondary-button secondary-button--small"
              disabled={settingsImportText.trim().length === 0}
              onClick={previewSettingsImport}
              type="button"
            >
              Preview import
            </button>
            <button
              className="secondary-button secondary-button--small"
              disabled={
                settingsImportBusy ||
                settingsImportText.trim().length === 0 ||
                settingsImportPreview?.valid !== true
              }
              onClick={() => void applySettingsImport()}
              type="button"
            >
              {settingsImportBusy ? "Applying..." : "Apply safe preferences"}
            </button>
          </div>
          {settingsImportPreview ? (
            <div
              className={`settings-transfer__preview${
                settingsImportPreview.valid ? " is-valid" : " is-invalid"
              }`}
            >
              <strong>{settingsImportPreview.title}</strong>
              <p>{settingsImportPreview.detail}</p>
              {settingsImportPreview.errors.length > 0 ? (
                <ul>
                  {settingsImportPreview.errors.map((error) => (
                    <li key={error}>{error}</li>
                  ))}
                </ul>
              ) : null}
              {Object.keys(settingsImportPreview.safePreferences).length > 0 ? (
                <p>
                  Safe preferences:{" "}
                  {Object.entries(settingsImportPreview.safePreferences)
                    .map(([key, value]) => `${key} ${value}`)
                    .join(", ")}
                </p>
              ) : null}
              {settingsImportPreview.migrationActions.length > 0 ? (
                <div
                  className="settings-transfer__migration"
                  aria-label="Settings migration actions"
                >
                  {settingsImportPreview.migrationActions
                    .slice(0, 8)
                    .map((action) => (
                      <div
                        className={`settings-transfer__migration-row settings-transfer__migration-row--${action.status}`}
                        key={action.id}
                      >
                        <span>{action.label}</span>
                        <strong>{action.status}</strong>
                        <small>{action.detail}</small>
                      </div>
                    ))}
                </div>
              ) : null}
              {settingsImportPreview.manualItems.length > 0 ? (
                <ul>
                  {settingsImportPreview.manualItems.slice(0, 6).map((item) => (
                    <li key={item}>{item}</li>
                  ))}
                </ul>
              ) : null}
            </div>
          ) : null}
        </article>

        <SettingsConnectorPanel
          connectors={connectors}
          connectorsBusy={connectorsBusy}
          connectorsError={connectorsError}
          copyPlannedConnectorCommand={copyPlannedConnectorCommand}
          openConnectorHelpId={openConnectorHelpId}
          plannedConnectorCopyNotice={plannedConnectorCopyNotice}
          plannedConnectorReadiness={plannedConnectorReadiness}
          setOpenConnectorHelpId={setOpenConnectorHelpId}
          toggleConnector={toggleConnector}
        />

        <SettingsRuntimeStatusCard
          appSemver={appSemver}
          appUpdateBusy={appUpdateBusy}
          appUpdateConfig={appUpdateConfig}
          appUpdateInstallBusy={appUpdateInstallBusy}
          appUpdateStatusCopy={appUpdateStatusCopy}
          checkForAppUpdate={() => void checkForAppUpdate()}
          headroomLifetimeSavingsPct={headroomLifetimeSavingsPct}
          headroomLogLines={headroomLogLines}
          headroomLogRef={headroomLogRef}
          headroomVersion={headroomVersion}
          kompressWarming={kompressWarming}
          onOpenHeadroomDashboard={() => void openHeadroomDashboard()}
          onToggleHeadroomDetails={() => {
            const next = !showHeadroomDetails;
            setShowHeadroomDetails(next);
            if (next) {
              void invoke<string[]>("get_headroom_logs", { maxLines: 80 });
            }
          }}
          runtimeActionError={runtimeActionError}
          runtimeLabel="Switchboard runtime"
          runtimeStatus={runtimeStatus}
          showHeadroomDetails={showHeadroomDetails}
        />
        <SettingsReleaseReadinessCard
          copyReleaseReadinessReport={copyReleaseReadinessReport}
          formatLocalReleaseEvidenceSequenceCopy={
            formatLocalReleaseEvidenceSequenceCopy
          }
          refreshReleaseReadinessReport={refreshReleaseReadinessReport}
          releaseEvidenceBusyId={releaseEvidenceBusyId}
          releaseLocalEvidenceRows={releaseLocalEvidenceRows}
          releaseReadinessAction={releaseReadinessAction}
          releaseReadinessCommandProp={releaseReadinessCommandProp}
          releaseReadinessCopyNotice={releaseReadinessCopyNotice}
          releaseReadinessCounts={releaseReadinessCounts}
          releaseReadinessError={releaseReadinessError}
          releaseReadinessEvidence={releaseReadinessEvidence}
          releaseReadinessRefreshing={releaseReadinessRefreshing}
          releaseReadinessReport={releaseReadinessReport}
          releaseReadinessRows={releaseReadinessRows}
          runLocalReleaseEvidenceSequence={runLocalReleaseEvidenceSequence}
          runReleaseEvidenceCommand={runReleaseEvidenceCommand}
        />

        <SettingsOpenLoginCard
          autostartBusy={autostartBusy}
          autostartEnabled={autostartEnabled}
          onToggle={handleAutostartToggle}
        />

        <article className="soft-card panel-card rollback-center-card">
          <div className="panel-card__header">
            <div>
              <h3>Rollback Center</h3>
              <p>
                Managed local changes Switchboard can disclose or undo with
                guarded restore or cleanup previews.
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
                  Native undo-all: {rollbackUndoAllPreview.ready.length} ready,{" "}
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
                      <span>Evidence: {plan.evidenceRequired[0]}</span>
                      <span>
                        Native restore:{" "}
                        {executionPreview.executionStatus.replace(/_/g, " ")}
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
                            Applied:{" "}
                            {applyResult.changed
                              ? "changed"
                              : "already current"}
                            ; backup: {applyResult.backupPath ?? "not created"}
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
                              Backup: {nativePreview.backupPath ?? "not found"}
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
                            Restored from {nativeResult.restoredFrom}; safety
                            backup:{" "}
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
            <p className="rollback-center-card__notice">{rollbackCopyNotice}</p>
          ) : null}
        </article>

        <SettingsUninstallCard
          onOpenUninstallDialog={() => {
            setUninstallError(null);
            setShowUninstallDialog(true);
          }}
        />

        <SettingsFooterActions supportUrl={SUPPORT_ISSUES_URL} />
      </section>
    </div>
  );
}
