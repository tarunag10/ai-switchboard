import { type RefObject } from "react";
import {
  ArrowClockwise,
  Copy,
  Info,
  Sparkle,
  Terminal,
} from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import {
  aggregateClientConnectors,
  connectorCompatibilityReport,
  connectorCompatibilityRoutingEvidenceLabel,
  connectorControlState,
  formatConnectorConfigDryRunPreview,
  formatPlannedConnectorConfigGateSummary,
  percent1,
  sortClientConnectors,
} from "../lib/dashboardHelpers";
import {
  buildManagedRollbackExecutionPreview,
  buildManagedRollbackPlan,
  canExecuteNativeManagedRollbackPreview,
  managedChangeRecords,
  supportsDedicatedCleanupRollbackRecord,
  supportsNativeManagedRollbackRecord,
} from "../lib/managedChanges";
import {
  formatPlannedConnectorConfigCreationPlansMarkdown,
  getPlannedConnector,
  getPlannedConnectorReadinessBadges,
  getPlannedConnectorReadinessContract,
  getPlannedConnectorSetupChecklistScript,
  getPlannedConnectorSetupGuide,
} from "../lib/plannedConnectors";
import {
  formatLocalReleaseEvidenceSequenceCopy,
  formatReleaseReadinessNextAction,
  formatReleaseReadinessSourceLabel,
  releaseReadinessCommand,
  releaseReadinessGroups,
  releaseReadinessItemCount,
  releaseShareableGates,
} from "../lib/releaseReadiness";
import { SettingsLegalPanel } from "./SettingsLegalPanel";
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
import type { ConnectorDossier } from "../lib/plannedConnectors";
import type { ReleaseReadinessReportSnapshot } from "../lib/releaseReadiness";
import type { ManagedChangeRecord } from "../lib/managedChanges";

const connectorSetupDetails: Record<string, string> = {
  claude_code:
    "Headroom injects ANTHROPIC_BASE_URL into shell profiles and ~/.claude/settings.json so Claude Code connects through Headroom. Token-saving add-ons like RTK are optional.",
  codex:
    "Headroom writes a managed provider block to ~/.codex/config.toml and exports OPENAI_BASE_URL in shell profiles so Codex connects through Headroom.",
  gemini_cli:
    "Headroom writes managed Gemini CLI shell routing exports and a rollback dossier so Gemini CLI connects through Switchboard while preserving user config.",
  opencode:
    "Headroom writes a managed OpenCode provider in ~/.config/opencode/opencode.json and a rollback dossier so OpenCode connects through Switchboard.",
  cursor:
    "Cursor is tracked as a gated editor connector. App-guided setup is shown first because Cursor settings and account behavior can vary by release channel.",
  grok_cli:
    "Grok / xAI CLI is tracked as a gated provider connector. Switchboard will keep model and account compatibility visible before routing it.",
  aider:
    "Aider is tracked as a gated agent connector. RTK-only mode can already reduce noisy shell output while provider wrapping is built.",
  continue:
    "Continue is tracked as a gated editor connector. App-guided setup requires approval until provider config backup and restore coverage is ready.",
  goose:
    "Goose is tracked as a gated agent connector. Local provider and MCP handoff support will be added after reversible setup coverage.",
  qwen_code:
    "Qwen Code is tracked as a gated CLI connector. Use Repo Intelligence packs today while provider routing waits for model and account guardrails.",
  amazon_q:
    "Amazon Q Developer CLI is tracked as a gated CLI connector. Verification packs are safe today; AWS credential and profile state stay outside app-managed setup.",
  windsurf:
    "Windsurf is a managed editor connector. Switchboard manages editor settings routing with backups, verification, rollback, and Off cleanup.",
  zed_ai:
    "Zed AI is a managed editor connector. Switchboard manages assistant settings routing with backups, verification, rollback, and Off cleanup.",
};

const connectorUnavailableReasons: Record<string, string> = {
  claude_code:
    "Claude Code was not detected. Install Claude Code, then reopen AI Switchboard for Mac.",
  codex:
    "Codex was not detected. Install the Codex CLI, then reopen AI Switchboard for Mac.",
  gemini_cli:
    "Gemini CLI was not detected. Install Gemini CLI, then reopen AI Switchboard for Mac.",
  opencode:
    "OpenCode was not detected. Install OpenCode, then reopen AI Switchboard for Mac.",
  cursor: "Cursor setup is gated until editor-settings backup, verify, rollback, and Off cleanup coverage is promoted.",
  grok_cli: "Grok / xAI CLI setup is gated until model/account guardrails and reversible provider routing are proven.",
  aider: "Aider setup is gated until wrapper/env backup, verify, rollback, and Off cleanup coverage is promoted.",
  continue: "Continue setup is gated until provider config backup, verify, rollback, and Off cleanup coverage is promoted.",
  goose: "Goose setup is gated until MCP/provider config backup, verify, rollback, and Off cleanup coverage is promoted.",
  qwen_code: "Qwen Code setup is gated until model/account guardrails and reversible provider routing are proven.",
  amazon_q:
    "Amazon Q Developer CLI setup is gated until credential-safe profile handling and reversible routing are proven.",
  windsurf: "Windsurf was not detected. Install Windsurf, then reopen AI Switchboard for Mac.",
  zed_ai: "Zed was not detected. Install Zed, then reopen AI Switchboard for Mac.",
};

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

function renderConnectorLogo(clientId: string) {
  return <Sparkle className="client-logo__glyph" size={20} weight="duotone" />;
}

function firstManagedConfigTarget(record: ManagedChangeRecord) {
  return record.paths[0] ?? "~/.config/mac-ai-switchboard-managed";
}

function supportsNativeManagedRollback(record: ManagedChangeRecord) {
  return (
    supportsNativeManagedRollbackRecord(record.id) ||
    supportsDedicatedCleanupRollbackRecord(record.id)
  );
}

function supportsNativeConfigApply(record: ManagedChangeRecord) {
  return record.id === "opencode-routing";
}

function getConnectorUnavailableReason(connector: ClientConnectorStatus) {
  return connectorControlState(connector).reason;
}

function getConnectorDetectionWarning(connector: ClientConnectorStatus) {
  if (connector.installed) {
    return null;
  }
  return connectorUnavailableReasons[connector.clientId] ?? null;
}

function getPlannedConnectorNextStep(
  connector: ClientConnectorStatus,
  plannedConnector: ConnectorDossier,
) {
  if (!connector.installed) {
    return "Install the tool first, then Switchboard will detect it here.";
  }

  if (plannedConnector.setupPhase === "Managed") {
    return "Detected. Managed routing can be repaired by Doctor if setup drifts.";
  }

  if (plannedConnector.setupPhase === "Detect") {
    return "Detected. Keep using RTK-only mode while a reversible routing adapter is researched.";
  }

  if (plannedConnector.setupPhase === "Guide") {
    return "Detected. App-guided setup is next so account-specific provider settings stay under your control.";
  }

  return "Detected. Automatic setup waits for backup, restore, and off-mode cleanup coverage.";
}

function formatBackendConnectorConfigPlan(
  connector: ClientConnectorStatus,
  plannedConnector: ConnectorDossier,
) {
  const stepDetails = connector.configCreationStepDetails ?? [];
  const stepLabels = connector.configCreationSteps ?? [];
  if (stepDetails.length === 0 && stepLabels.length === 0) {
    return formatPlannedConnectorConfigCreationPlansMarkdown([
      plannedConnector,
    ]);
  }

  return [
    "# AI Switchboard Connector Config Creation Plan",
    "",
    `## ${connector.name}`,
    "- Automation enabled: no",
    "- Safety note: Backend metadata keeps config creation gated until every step has tests and Doctor evidence.",
    ...(stepDetails.length > 0
      ? stepDetails.map((step) => {
          const evidence = step.requiredEvidence?.length
            ? ` Required evidence: ${step.requiredEvidence.join(" ")}`
            : "";
          return `- ${step.label}: ${step.detail}${evidence}`;
        })
      : stepLabels.map((step) => `- ${step}`)),
    "",
    formatConnectorConfigDryRunPreview(connector),
  ].join("\n");
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
            AI Switchboard does not include remote account, billing,
            checkout, or paid pricing APIs. Provider model calls still use the
            accounts you configure in Claude, Codex, or other tools.
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

        <article className="soft-card panel-card">
          <div className="panel-card__header">
            <div />
          </div>
          <div className="connector-readiness">
            <div>
              <span className="connector-readiness__eyebrow">
                Connector readiness
              </span>
              <strong>{plannedConnectorReadiness.headline}</strong>
              <p>{plannedConnectorReadiness.detail}</p>
            </div>
            <div className="connector-readiness__actions">
              <div
                className="connector-readiness__metrics"
                aria-label="Connector readiness summary"
              >
                <span>
                  <strong>{plannedConnectorReadiness.detectedCount}</strong>
                  detected
                </span>
                <span>
                  <strong>{plannedConnectorReadiness.manualOnlyCount}</strong>
                  approval
                </span>
                <span>
                  <strong>{plannedConnectorReadiness.notDetectedCount}</strong>
                  missing
                </span>
                <span>
                  <strong>{plannedConnectorReadiness.safeTodayCount}</strong>
                  safe now
                </span>
                <span>
                  <strong>
                    {plannedConnectorReadiness.automationGateCount}
                  </strong>
                  gates
                </span>
              </div>
              <button
                type="button"
                className="connector-readiness__copy"
                onClick={() =>
                  void copyPlannedConnectorCommand(
                    getPlannedConnectorSetupChecklistScript(),
                    "Connector checklist",
                  )
                }
              >
                <Copy size={13} weight="bold" />
                Copy checks
              </button>
              <button
                type="button"
                className="connector-readiness__copy"
                onClick={() =>
                  void copyPlannedConnectorCommand(
                    formatPlannedConnectorConfigCreationPlansMarkdown(),
                    "Connector config plans",
                  )
                }
              >
                <Copy size={13} weight="bold" />
                Copy config plans
              </button>
            </div>
          </div>
          <div className="connector-list">
            {sortClientConnectors(aggregateClientConnectors(connectors)).map(
              (connector) => {
                const connectorLabel =
                  connector.clientId === "claude_code"
                    ? "Claude Code connection"
                    : connector.clientId === "codex"
                      ? "Codex connection"
                      : connector.name;
                const controlState = connectorControlState(connector);
                const unavailableReason =
                  getConnectorUnavailableReason(connector);
                const detectionWarning =
                  getConnectorDetectionWarning(connector);
                const toggleDisabled = connectorsBusy || controlState.disabled;
                const plannedConnector = getPlannedConnector(
                  connector.clientId,
                );
                const plannedSetupGuide = plannedConnector
                  ? getPlannedConnectorSetupGuide(plannedConnector.id)
                  : null;
                const plannedReadiness = plannedConnector
                  ? getPlannedConnectorReadinessContract(plannedConnector)
                  : null;
                const plannedReadinessBadges = plannedConnector
                  ? getPlannedConnectorReadinessBadges(plannedConnector)
                  : [];
                const connectorSetupPhase =
                  connector.setupPhase ?? plannedConnector?.setupPhase ?? null;
                const connectorSetupHint =
                  connector.setupHint ?? plannedConnector?.notes ?? null;
                const compatibilityReport =
                  connectorCompatibilityReport(connector);
                const configGateSummary =
                  formatPlannedConnectorConfigGateSummary(connector);
                return (
                  <article className="connector-item" key={connector.clientId}>
                    <div>
                      <h3>
                        <span className="client-logo" aria-hidden="true">
                          {renderConnectorLogo(connector.clientId)}
                        </span>
                        {connectorLabel}
                        {connector.supportStatus === "planned" ? (
                          <span className="connector-item__badge connector-item__badge--planned">
                            Gated
                          </span>
                        ) : null}
                        <button
                          className="connector-help"
                          onClick={() =>
                            setOpenConnectorHelpId((current) =>
                              current === connector.clientId
                                ? null
                                : connector.clientId,
                            )
                          }
                          type="button"
                          aria-label={`Show setup details for ${connector.name}`}
                          aria-expanded={
                            openConnectorHelpId === connector.clientId
                          }
                        >
                          <Info size={11} weight="bold" />
                        </button>
                      </h3>
                      {openConnectorHelpId === connector.clientId ? (
                        <p className="connector-tooltip">
                          {connectorSetupHint ??
                            connectorSetupDetails[connector.clientId] ??
                            "Switchboard applies local connector configuration."}
                        </p>
                      ) : null}
                      {plannedConnector ? (
                        <div className="connector-plan">
                          <div className="connector-plan__meta">
                            <span>{connectorSetupPhase}</span>
                            <span>
                              {connector.category ?? plannedConnector.category}
                            </span>
                          </div>
                          <p className="connector-plan__target">
                            {plannedConnector.integrationTarget}
                          </p>
                          {plannedReadiness ? (
                            <div className="connector-plan__readiness">
                              <div>
                                <strong>Readiness contract</strong>
                                <span>
                                  Next gate:{" "}
                                  {plannedReadiness.stages.find(
                                    (stage) =>
                                      stage.id ===
                                      plannedReadiness.nextBlockedStage,
                                  )?.label ?? "Automation ready"}
                                </span>
                              </div>
                              <div
                                className="connector-plan__stage-row"
                                aria-label={`${connector.name} readiness contract`}
                              >
                                {plannedReadiness.stages.map((stage) => (
                                  <span
                                    className={`connector-plan__stage connector-plan__stage--${stage.state}`}
                                    key={stage.id}
                                    title={stage.evidence}
                                  >
                                    {stage.label}
                                  </span>
                                ))}
                              </div>
                            </div>
                          ) : null}
                          {plannedReadinessBadges.length ? (
                            <div
                              className="connector-plan__badges"
                              aria-label={`${connector.name} safety badges`}
                            >
                              {plannedReadinessBadges.map((badge) => (
                                <span
                                  className={`connector-plan__badge connector-plan__badge--${badge.kind}`}
                                  key={badge.kind}
                                  title={badge.detail}
                                >
                                  {badge.label}
                                </span>
                              ))}
                            </div>
                          ) : null}
                          {compatibilityReport ? (
                            <div className="connector-plan__compatibility">
                              <strong>{compatibilityReport.title}</strong>
                              {compatibilityReport.binaryPath ? (
                                <span>
                                  {compatibilityReport.primaryPathLabel}{" "}
                                  {compatibilityReport.binaryPath}
                                </span>
                              ) : null}
                              {compatibilityReport.version ? (
                                <span>Version {compatibilityReport.version}</span>
                              ) : null}
                              {compatibilityReport.configSurface ? (
                                <span>
                                  Config {compatibilityReport.configSurface}
                                </span>
                              ) : null}
                              {compatibilityReport.routingBlocker ? (
                                <span>
                                  {connectorCompatibilityRoutingEvidenceLabel(
                                    compatibilityReport,
                                  )}{" "}
                                  {compatibilityReport.routingBlocker}
                                </span>
                              ) : null}
                              {compatibilityReport.configCreationGates
                                .length ? (
                                <span>
                                  Config gates{" "}
                                  {compatibilityReport.configCreationGates
                                    .map((gate) => gate.label)
                                    .join(" -> ")}
                                </span>
                              ) : null}
                              <span>
                                Automation{" "}
                                {compatibilityReport.automationEnabled
                                  ? "enabled"
                                  : "approval required"}
                              </span>
                            </div>
                          ) : null}
                          {configGateSummary ? (
                            <div className="connector-plan__config-gates">
                              <strong>{configGateSummary.title}</strong>
                              <span>{configGateSummary.detail}</span>
                              <span>
                                Next: {configGateSummary.nextGateLabel}
                              </span>
                              <span>{configGateSummary.safetyNote}</span>
                            </div>
                          ) : null}
                          {connector.detectionSources?.length ||
                          connector.configLocations?.length ||
                          connector.detectionEvidence?.length ||
                          connector.automationGates?.length ||
                          connector.manualWorkflow?.length ||
                          connector.configCreationStepDetails?.length ||
                          connector.configCreationSteps?.length ||
                          connector.automationPath?.length ? (
                            <div className="connector-plan__backend">
                              <strong>Backend checks</strong>
                              {connector.detectionSources?.length ? (
                                <span>
                                  Detects{" "}
                                  {connector.detectionSources
                                    .slice(0, 3)
                                    .join(", ")}
                                </span>
                              ) : null}
                              {connector.configLocations?.length ? (
                                <span>
                                  Watches{" "}
                                  {connector.configLocations
                                    .slice(0, 2)
                                    .join(", ")}
                                </span>
                              ) : null}
                              {connector.detectionEvidence?.length ? (
                                <span>
                                  Evidence{" "}
                                  {connector.detectionEvidence
                                    .slice(0, 2)
                                    .join(" · ")}
                                </span>
                              ) : null}
                              {connector.automationGates?.length ? (
                                <span>
                                  Gates{" "}
                                  {connector.automationGates
                                    .slice(0, 2)
                                    .join(" · ")}
                                </span>
                              ) : null}
                              {connector.manualWorkflow?.length ? (
                                <span>
                                  Approval needed{" "}
                                  {connector.manualWorkflow
                                    .slice(0, 2)
                                    .join(" · ")}
                                </span>
                              ) : null}
                              {connector.configCreationSteps?.length ? (
                                <span>
                                  Config plan{" "}
                                  {(
                                    connector.configCreationStepDetails
                                      ?.slice(0, 4)
                                      .map(
                                        (step) =>
                                          `${step.label}: ${step.detail}${
                                            step.requiredEvidence?.length
                                              ? ` Evidence ${step.requiredEvidence.join(" ")}`
                                              : ""
                                          }`,
                                      ) ??
                                    connector.configCreationSteps.slice(0, 4)
                                  ).join(" -> ")}
                                </span>
                              ) : null}
                              {connector.automationPath?.length ? (
                                <span>
                                  Automation path{" "}
                                  {connector.automationPath
                                    .slice(0, 7)
                                    .map(
                                      (stage) =>
                                        `${stage.label}: ${stage.status}`,
                                    )
                                    .join(" -> ")}
                                </span>
                              ) : null}
                            </div>
                          ) : null}
                          <div className="connector-plan__capabilities">
                            {plannedConnector.capabilityRows.map(
                              (capability) => (
                                <div
                                  className="connector-plan__capability"
                                  key={`${plannedConnector.id}-${capability.label}`}
                                >
                                  <div>
                                    <strong>{capability.label}</strong>
                                    <span>{capability.detail}</span>
                                  </div>
                                  <span
                                    className={`connector-plan__state connector-plan__state--${capability.state
                                      .toLowerCase()
                                      .replace(/\s+/g, "-")}`}
                                  >
                                    {capability.state}
                                  </span>
                                </div>
                              ),
                            )}
                          </div>
                          <p className="connector-plan__next">
                            {getPlannedConnectorNextStep(
                              connector,
                              plannedConnector,
                            )}
                          </p>
                          {plannedSetupGuide ? (
                            <div className="connector-plan__guide">
                              <div>
                                <strong>{plannedSetupGuide.label}</strong>
                                <code>{plannedSetupGuide.command}</code>
                              </div>
                              <button
                                type="button"
                                className="connector-plan__copy"
                                onClick={() =>
                                  void copyPlannedConnectorCommand(
                                    plannedSetupGuide.command,
                                    connector.name,
                                  )
                                }
                                aria-label={`Copy ${connector.name} setup check command`}
                              >
                                <Copy size={13} weight="bold" />
                              </button>
                              <button
                                type="button"
                                className="connector-plan__copy"
                                onClick={() =>
                                  void copyPlannedConnectorCommand(
                                    formatBackendConnectorConfigPlan(
                                      connector,
                                      plannedConnector,
                                    ),
                                    `${connector.name} config plan`,
                                  )
                                }
                                aria-label={`Copy ${connector.name} config creation plan`}
                              >
                                <Copy size={13} weight="duotone" />
                              </button>
                            </div>
                          ) : null}
                          {plannedSetupGuide ? (
                            <p className="connector-plan__note">
                              {plannedSetupGuide.notes}
                            </p>
                          ) : null}
                        </div>
                      ) : null}
                      {connector.enabled &&
                      !connector.verified &&
                      connector.installed ? (
                        <p className="connector-item__restart">
                          Restart {connector.name} to start routing through
                          Headroom.
                        </p>
                      ) : null}
                      {(detectionWarning ?? unavailableReason) ? (
                        <p className="connector-item__reason">
                          {detectionWarning ?? unavailableReason}
                        </p>
                      ) : null}
                    </div>
                    <div className="connector-item__controls">
                      <button
                        aria-checked={connector.enabled}
                        aria-label={`${connector.enabled ? "Disable" : "Enable"} ${connector.name} connector`}
                        className={`connector-switch${connector.enabled ? " is-on" : ""}`}
                        disabled={toggleDisabled}
                        onClick={() =>
                          void toggleConnector(connector, !connector.enabled)
                        }
                        role="switch"
                        title={
                          controlState.reason ?? unavailableReason ?? undefined
                        }
                        type="button"
                      >
                        <span className="connector-switch__thumb" />
                      </button>
                    </div>
                  </article>
                );
              },
            )}
          </div>
          {connectorsError ? (
            <p className="install-progress__error">{connectorsError}</p>
          ) : null}
          {plannedConnectorCopyNotice ? (
            <p className="connector-copy-notice">
              {plannedConnectorCopyNotice}
            </p>
          ) : null}
        </article>

        <article className="soft-card panel-card">
          <div className="panel-card__header">
            <div>
              <h3>Tools status</h3>
            </div>
          </div>
          <div className="runtime-status">
            <div className="runtime-status__topline">
              <span className="runtime-status__section-title">
                AI Switchboard for Mac app ({appSemver})
                {appUpdateConfig?.betaChannelEnabled ? (
                  <span className="runtime-status__channel-pill">
                    beta channel
                  </span>
                ) : null}
              </span>
            </div>
            <div className="runtime-status__section-action-row">
              <button
                className="secondary-button secondary-button--small"
                disabled={appUpdateBusy || appUpdateInstallBusy}
                onClick={() => void checkForAppUpdate()}
                type="button"
              >
                {appUpdateBusy ? "Checking\u2026" : "Check for updates"}
              </button>
              {appUpdateStatusCopy ? (
                <p className="app-update-card__summary runtime-status__summary">
                  {appUpdateStatusCopy}
                </p>
              ) : null}
            </div>
            <div className="runtime-status__meta">
              <span className="runtime-status__section-title">
                Headroom CLI ({headroomVersion})
                {headroomLifetimeSavingsPct !== null ? (
                  <span className="runtime-status__section-context">
                    {" "}
                    ({percent1(headroomLifetimeSavingsPct)}% all-time savings)
                  </span>
                ) : null}
              </span>
            </div>
            <div className="runtime-status__grid runtime-status__grid--4">
              {(
                [
                  {
                    name: "Runtime",
                    ok: runtimeStatus?.running === true,
                  },
                  {
                    name: "Proxy",
                    ok: runtimeStatus?.proxyReachable === true,
                    suffix: "6767",
                    onClick: () => void invoke("open_headroom_dashboard"),
                  },
                  {
                    name: "MCP",
                    ok:
                      runtimeStatus?.mcpConfigured === true
                        ? true
                        : runtimeStatus?.mcpConfigured === false
                          ? false
                          : null,
                  },
                  {
                    name: "Kompress",
                    ok: kompressWarming
                      ? null
                      : runtimeStatus?.kompressEnabled === true
                        ? true
                        : runtimeStatus?.kompressEnabled === false
                          ? false
                          : null,
                    suffix: kompressWarming ? "warming up" : undefined,
                  },
                ] as {
                  name: string;
                  ok: boolean | null;
                  suffix?: string;
                  onClick?: () => void;
                }[]
              ).map((s) => {
                const indicatorClass =
                  s.ok === true
                    ? "runtime-status__indicator--ok"
                    : s.ok === false
                      ? "runtime-status__indicator--off"
                      : "runtime-status__indicator--unknown";
                const indicatorSymbol =
                  s.ok === true
                    ? "\u2714"
                    : s.ok === false
                      ? "\u2716"
                      : "\u2013";
                return (
                  <span
                    key={s.name}
                    className={`runtime-status__item${s.onClick ? " runtime-status__item--clickable" : ""}`}
                    onClick={s.onClick}
                    title={
                      s.ok === null ? `${s.name} status unknown` : undefined
                    }
                  >
                    <span className="runtime-status__label">{s.name}:</span>
                    <span
                      className={`runtime-status__indicator ${indicatorClass}`}
                    >
                      {indicatorSymbol}
                    </span>
                    {s.suffix && (
                      <span className="runtime-status__suffix">
                        ({s.suffix})
                      </span>
                    )}
                  </span>
                );
              })}
            </div>
            <button
              className="link-button runtime-status__section-action"
              onClick={async () => {
                const next = !showHeadroomDetails;
                setShowHeadroomDetails(next);
                if (next) {
                  try {
                    const lines = await invoke<string[]>("get_headroom_logs", {
                      maxLines: 80,
                    });
                    // parent handles setHeadroomLogLines
                  } catch {
                    // parent handles error state
                  }
                }
              }}
              type="button"
            >
              {showHeadroomDetails
                ? "Hide headroom logs"
                : "Show headroom logs"}
            </button>
            {showHeadroomDetails ? (
              <pre className="runtime-log" ref={headroomLogRef as React.RefObject<HTMLPreElement>}>
                {headroomLogLines.length > 0
                  ? headroomLogLines.join("\n")
                  : "No log output yet."}
              </pre>
            ) : null}
          </div>
        </article>
        <article className="soft-card panel-card release-readiness-card">
          <div className="panel-card__header">
            <div>
              <h3>Release readiness</h3>
              <p>
                {releaseReadinessItemCount()} checks before a signed DMG can be
                handed to testers.
              </p>
            </div>
            <div className="release-readiness-card__actions">
              <button
                className="secondary-button secondary-button--small"
                disabled={releaseReadinessRefreshing}
                onClick={() => void refreshReleaseReadinessReport()}
                type="button"
              >
                <ArrowClockwise size={14} weight="bold" />
                {releaseReadinessRefreshing ? "Refreshing" : "Refresh report"}
              </button>
              <button
                className="secondary-button secondary-button--small"
                disabled={releaseEvidenceBusyId !== null}
                onClick={() => void runLocalReleaseEvidenceSequence()}
                title={formatLocalReleaseEvidenceSequenceCopy()}
                type="button"
              >
                <ArrowClockwise size={14} weight="bold" />
                {releaseEvidenceBusyId === "local-evidence"
                  ? "Running local evidence"
                  : "Run local evidence"}
              </button>
              <button
                className="secondary-button secondary-button--small"
                onClick={() => void copyReleaseReadinessReport()}
                type="button"
              >
                <Copy size={14} weight="bold" />
                {releaseReadinessReport?.report
                  ? "Copy report snapshot"
                  : "Copy report command"}
              </button>
            </div>
          </div>
          <div className="release-readiness-card__command">
            <Terminal size={15} weight="duotone" />
            <code>{releaseReadinessCommandProp}</code>
          </div>
          <p className="release-readiness-card__source">
            {formatReleaseReadinessSourceLabel(
              releaseReadinessReport?.report
                ? releaseReadinessReport.reportPath
                : null,
            )}
          </p>
          <p className="release-readiness-card__source">
            {releaseReadinessEvidence.copy}
          </p>
          <p className="release-readiness-card__source">
            {formatReleaseReadinessNextAction(releaseReadinessAction as any)}
          </p>
          {releaseReadinessError ? (
            <p className="release-readiness-card__error">
              {releaseReadinessError}
            </p>
          ) : null}
          <div
            className="release-readiness-card__summary"
            aria-label="Release readiness status summary"
          >
            <span>
              <strong>{releaseReadinessCounts.ready}</strong> scripted
            </span>
            <span>
              <strong>{releaseReadinessCounts.blocked}</strong> blocked
            </span>
            <span>
              <strong>{releaseReadinessCounts["local-only"]}</strong> local-only
            </span>
          </div>
          <div
            className="release-readiness-card__status-grid"
            aria-label="Release readiness source status"
          >
            {releaseReadinessRows.map((row) => (
              <div className="release-readiness-card__status-row" key={row.id}>
                <div>
                  <strong>{row.label}</strong>
                  <span>{row.detail}</span>
                </div>
                <span
                  className={`release-readiness-card__status-badge release-readiness-card__status-badge--${row.tone}`}
                >
                  {row.statusLabel}
                </span>
                <code>{row.source}</code>
              </div>
            ))}
          </div>
          {releaseLocalEvidenceRows.length > 0 ? (
            <div
              className="release-readiness-card__local-evidence"
              aria-label="Local validation evidence"
            >
              <h4>Local evidence</h4>
              <div className="release-readiness-card__status-grid">
                {releaseLocalEvidenceRows.map((row) => (
                  <div
                    className="release-readiness-card__status-row"
                    key={row.id}
                  >
                    <div>
                      <strong>{row.label}</strong>
                      <span>{row.detail}</span>
                    </div>
                    <span
                      className={`release-readiness-card__status-badge release-readiness-card__status-badge--${
                        row.passed ? "ready" : "blocked"
                      }`}
                    >
                      {row.statusLabel}
                    </span>
                    <code>{row.command}</code>
                    <code>{row.summaryPath}</code>
                  </div>
                ))}
              </div>
            </div>
          ) : null}
          <div
            className="release-readiness-card__gates"
            aria-label="Shareable DMG gates"
          >
            {releaseShareableGates.map((gate) => (
              <div className="release-readiness-card__gate" key={gate.id}>
                <strong>{gate.label}</strong>
                <span>{gate.detail}</span>
              </div>
            ))}
          </div>
          <div className="release-readiness-card__grid">
            {releaseReadinessGroups.map((group) => (
              <section className="release-readiness-card__group" key={group.id}>
                <h4>{group.title}</h4>
                <ul>
                  {group.items.map((item) => (
                    <li key={item.id}>
                      <strong>{item.label}</strong>
                      <span>{item.detail}</span>
                      {item.command ? <code>{item.command}</code> : null}
                      {item.executable ? (
                        <button
                          className="secondary-button secondary-button--small"
                          disabled={releaseEvidenceBusyId !== null}
                          onClick={() =>
                            void runReleaseEvidenceCommand(item.id)
                          }
                          type="button"
                        >
                          <ArrowClockwise size={14} weight="bold" />
                          {releaseEvidenceBusyId === item.id
                            ? "Running"
                            : "Run evidence"}
                        </button>
                      ) : null}
                    </li>
                  ))}
                </ul>
              </section>
            ))}
          </div>
          {releaseReadinessCopyNotice ? (
            <p className="connector-copy-notice">
              {releaseReadinessCopyNotice}
            </p>
          ) : null}
        </article>
        <article className="soft-card panel-card">
          <div className="panel-card__header">
            <div>
              <h3>Open on login</h3>
            </div>
            <div>
              <p>
                Automatically launch AI Switchboard for Mac whenever you log in or restart.
              </p>
            </div>
            <div className="connector-item__controls">
              <button
                aria-checked={autostartEnabled === true}
                aria-label={`${autostartEnabled ? "Disable" : "Enable"} open on login`}
                className={`connector-switch${autostartEnabled ? " is-on" : ""}`}
                disabled={autostartBusy || autostartEnabled === null}
                onClick={() => void handleAutostartToggle(!autostartEnabled)}
                role="switch"
                type="button"
              >
                <span className="connector-switch__thumb" />
              </button>
            </div>
          </div>
        </article>

        <article className="soft-card panel-card rollback-center-card">
          <div className="panel-card__header">
            <div>
              <h3>Rollback Center</h3>
              <p>
                Managed local changes Switchboard can disclose or undo
                with guarded restore or cleanup previews.
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

        <article className="soft-card panel-card">
          <div className="panel-card__header">
            <div>
              <h3>Uninstall</h3>
            </div>
          </div>
          <p>
            Reverses AI Switchboard for Mac changes: removes routing hooks, managed
            runtime storage, app state, login item, known Keychain entries, and
            managed config blocks. AI Switchboard for Mac will quit when done.
          </p>
          <button
            className="secondary-button secondary-button--small"
            onClick={() => {
              setUninstallError(null);
              setShowUninstallDialog(true);
            }}
            type="button"
          >
            Uninstall AI Switchboard for Mac
          </button>
        </article>

        <button
          className="contact-link"
          onClick={() =>
            void invoke("open_external_link", {
              url: SUPPORT_ISSUES_URL,
            })
          }
          type="button"
        >
          Contact us
        </button>
        <button
          className="quit-button"
          onClick={() => void invoke("quit_headroom")}
          type="button"
        >
          Quit AI Switchboard for Mac
        </button>
      </section>
    </div>
  );
}
