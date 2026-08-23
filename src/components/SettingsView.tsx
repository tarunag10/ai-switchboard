import type { Dispatch, RefObject, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ProxySessionAuthCard } from "./ProxySessionAuthCard";
import { RollbackCenter } from "./RollbackCenter";
import { SettingsConnectorPanel } from "./SettingsConnectorPanel";
import { SettingsLegalPanel } from "./SettingsLegalPanel";
import { SettingsOpenLoginCard } from "./SettingsOpenLoginCard";
import { SettingsReleaseReadinessCard } from "./SettingsReleaseReadinessCard";
import { SettingsRuntimeStatusCard } from "./SettingsRuntimeStatusCard";
import { SettingsTransferCard } from "./SettingsTransferCard";
import { SettingsUninstallCard } from "./SettingsUninstallCard";
import type { SettingsImportPreview } from "../lib/settingsTransfer";
import type {
  AppUpdateConfiguration,
  ClientConnectorStatus,
  DashboardState,
  RuntimeStatus,
  SavingsMode,
  SwitchboardMode,
} from "../lib/types";
import type {
  ReleaseEvidenceCommandResult,
  ReleaseReadinessReportPayload,
} from "../lib/releaseEvidenceController";
import type { ReleaseReadinessNextAction } from "../lib/releaseReadiness";

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

export interface SettingsViewProps {
  hidden: boolean;
  readinessSignals: readonly string[];
  dashboard: DashboardState;
  switchboardMode: SwitchboardMode;
  savingsMode: SavingsMode;
  connectors: ClientConnectorStatus[];
  appSemver: string;

  settingsTransferNotice: string | null;
  setSettingsImportText: (value: string) => void;
  setSettingsImportPreview: (value: SettingsImportPreview | null) => void;
  setSettingsTransferNotice: (value: string | null) => void;
  settingsImportText: string;
  settingsImportPreview: SettingsImportPreview | null;
  settingsImportBusy: boolean;
  copySettingsExport: () => Promise<void>;
  previewSettingsImport: () => void;
  applySettingsImport: () => Promise<void>;

  plannedConnectorReadiness: PlannedConnectorReadinessSummary;
  plannedConnectorCopyNotice: string | null;

  connectorsBusy: boolean;
  connectorsError: string | null;
  verifyConnectors: () => Promise<void>;
  openConnectorHelpId: string | null;
  setOpenConnectorHelpId: Dispatch<SetStateAction<string | null>>;
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
  setHeadroomLogLines: Dispatch<SetStateAction<string[]>>;
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
  releaseEvidenceResult: ReleaseEvidenceCommandResult | null;
  releaseReadinessCommand: string;
  releaseReadinessReport: ReleaseReadinessReportPayload | null;
  releaseReadinessEvidence: { copy: string };
  releaseReadinessAction: ReleaseReadinessNextAction | null;
  releaseReadinessError: string | null;
  releaseReadinessCounts: ReleaseReadinessCounts;
  releaseReadinessRows: ReleaseReadinessRow[];
  releaseLocalEvidenceRows: ReleaseLocalEvidenceRow[];
  releaseReadinessCopyNotice: string | null;
  copyReleaseReadinessReport: () => Promise<void>;
  refreshReleaseReadinessReport: () => Promise<void>;
  runReleaseEvidenceCommand: (commandId: string) => Promise<void>;
  runLocalReleaseEvidenceSequence: () => Promise<void>;
  formatLocalReleaseEvidenceSequenceCopy: () => string;

  setUninstallError: (value: string | null) => void;
  setShowUninstallDialog: (value: boolean) => void;

  SUPPORT_ISSUES_URL: string;
}

export function SettingsView({
  hidden,
  readinessSignals,
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
  verifyConnectors,
  openConnectorHelpId,
  setOpenConnectorHelpId,
  toggleConnector,
  copyPlannedConnectorCommand,
  autostartEnabled,
  autostartBusy,
  handleAutostartToggle,
  showHeadroomDetails,
  setShowHeadroomDetails,
  setHeadroomLogLines,
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
  releaseEvidenceResult,
  releaseReadinessCommand,
  releaseReadinessReport,
  releaseReadinessEvidence,
  releaseReadinessAction,
  releaseReadinessError,
  releaseReadinessCounts,
  releaseReadinessRows,
  releaseLocalEvidenceRows,
  releaseReadinessCopyNotice,
  copyReleaseReadinessReport,
  refreshReleaseReadinessReport,
  runReleaseEvidenceCommand,
  runLocalReleaseEvidenceSequence,
  formatLocalReleaseEvidenceSequenceCopy,
  setUninstallError,
  setShowUninstallDialog,
  SUPPORT_ISSUES_URL,
}: SettingsViewProps) {
  return (
    <div
      className="tray-content"
      data-readiness-signals={readinessSignals.join(" | ")}
      hidden={hidden}
    >
      <section className="panel-stack">
        <article className="soft-card panel-card settings-account-card">
          <div className="settings-account-row">
            <p className="settings-account-copy">
              Account and paid APIs: <em>not included</em>
            </p>
            <span className="settings-account-badge">Local-free</span>
          </div>
          <p className="settings-account-notice">
            AI Switchboard does not include remote account, billing, checkout, or
            paid pricing APIs. Provider model calls still use the accounts you
            configure in Claude, Codex, or other tools.
          </p>
        </article>

        <SettingsLegalPanel
          requiredTermsVersion={dashboard.requiredTermsVersion}
        />

        <SettingsTransferCard
          switchboardMode={switchboardMode}
          savingsMode={savingsMode}
          connectorCount={connectors.length}
          addonCount={dashboard.tools.filter((tool) => !tool.required).length}
          importText={settingsImportText}
          importPreview={settingsImportPreview}
          importBusy={settingsImportBusy}
          notice={settingsTransferNotice}
          onCopyExport={() => void copySettingsExport()}
          onImportTextChange={(value) => {
            setSettingsImportText(value);
            setSettingsImportPreview(null);
            setSettingsTransferNotice(null);
          }}
          onPreviewImport={previewSettingsImport}
          onApplyImport={() => void applySettingsImport()}
        />

        <ProxySessionAuthCard />

        <SettingsConnectorPanel
          connectors={connectors}
          connectorsBusy={connectorsBusy}
          connectorsError={connectorsError}
          verifyConnectors={verifyConnectors}
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
          hideLogsLabel="Hide headroom logs"
          kompressWarming={kompressWarming}
          onOpenHeadroomDashboard={() => void invoke("open_headroom_dashboard")}
          onToggleHeadroomDetails={() => {
            const next = !showHeadroomDetails;
            setShowHeadroomDetails(next);
            if (next) {
              void invoke<string[]>("get_headroom_logs", {
                maxLines: 80,
              })
                .then(setHeadroomLogLines)
                .catch(() =>
                  setHeadroomLogLines(["Failed to load headroom logs."]),
                );
            }
          }}
          runtimeLabel="Headroom CLI"
          runtimeStatus={runtimeStatus}
          showHeadroomDetails={showHeadroomDetails}
          showLogsLabel="Show headroom logs"
        />
        <SettingsReleaseReadinessCard
          copyReleaseReadinessReport={() => void copyReleaseReadinessReport()}
          formatLocalReleaseEvidenceSequenceCopy={
            formatLocalReleaseEvidenceSequenceCopy
          }
          refreshReleaseReadinessReport={() =>
            void refreshReleaseReadinessReport()
          }
          releaseEvidenceBusyId={releaseEvidenceBusyId}
          releaseEvidenceResult={releaseEvidenceResult}
          releaseLocalEvidenceRows={releaseLocalEvidenceRows}
          releaseReadinessAction={releaseReadinessAction}
          releaseReadinessCommandProp={releaseReadinessCommand}
          releaseReadinessCopyNotice={releaseReadinessCopyNotice}
          releaseReadinessCounts={releaseReadinessCounts}
          releaseReadinessError={releaseReadinessError}
          releaseReadinessEvidence={releaseReadinessEvidence}
          releaseReadinessRefreshing={releaseReadinessRefreshing}
          releaseReadinessReport={releaseReadinessReport}
          releaseReadinessRows={releaseReadinessRows}
          runLocalReleaseEvidenceSequence={() =>
            void runLocalReleaseEvidenceSequence()
          }
          runReleaseEvidenceCommand={(commandId) =>
            void runReleaseEvidenceCommand(commandId)
          }
        />
        <SettingsOpenLoginCard
          autostartBusy={autostartBusy}
          autostartEnabled={autostartEnabled}
          onToggle={handleAutostartToggle}
        />

        <RollbackCenter />

        <SettingsUninstallCard
          onOpenUninstallDialog={() => {
            setUninstallError(null);
            setShowUninstallDialog(true);
          }}
        />

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
