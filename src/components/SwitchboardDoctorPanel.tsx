import { Copy } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  canRepairIssue,
  buildDoctorReportTimelineEvents,
  doctorIssueActionKind,
  doctorIssueActionLabelForIssue,
  doctorIssueGuidance,
  doctorRepairLabel,
  formatDoctorReportShareText,
  formatDoctorTimelineShareText,
  formatPlannedConnectorDoctorDossiers,
  formatVerifyOffModeShareText,
  plannedConnectorDoctorPreviewRows,
} from "../lib/doctorRepairCopy";
import {
  formatManagedFootprintReport,
  formatManagedRollbackInventory,
} from "../lib/managedChanges";
import type {
  DoctorIssue,
  DoctorReport,
  ManagedFootprintReport,
  CodexThreadRetaggingMode,
  CodexThreadRetaggingSettings,
} from "../lib/types";

interface SwitchboardDoctorPanelProps {
  report: DoctorReport | null;
  busyAction: string | null;
  error: string | null;
  successMessage?: string | null;
  footprintReport?: ManagedFootprintReport | null;
  onRepair: (action: string) => void;
}

function issueTone(issue: DoctorIssue): string {
  return issue.severity === "error" ? "error" : "warning";
}

export function SwitchboardDoctorPanel({
  report,
  busyAction,
  error,
  successMessage,
  footprintReport,
  onRepair,
}: SwitchboardDoctorPanelProps) {
  if (!report) {
    return null;
  }

  const canRepair = report.issues.some((issue) =>
    canRepairIssue(issue.repairAction),
  );
  const repairableCount = report.issues.filter((issue) =>
    canRepairIssue(issue.repairAction),
  ).length;
  const manualCount = Math.max(0, report.issues.length - repairableCount);
  const title = report.status === "ok" ? "Ready" : "Needs attention";
  const doctorReport = report;
  const automaticIssues = report.issues.filter((issue) =>
    canRepairIssue(issue.repairAction),
  );
  const manualIssues = report.issues.filter(
    (issue) => !canRepairIssue(issue.repairAction),
  );
  const nextAutomaticIssue = automaticIssues[0] ?? null;
  const hasOffModeVerification = report.issues.some(
    (issue) =>
      issue.id === "off_mode_not_clean" ||
      issue.repairAction === "verify_off_mode",
  );
  const hasPlannedConnectorEvidence = report.issues.some(
    (issue) => issue.id === "planned_connectors_detected",
  );
  const connectorPreviewRows = hasPlannedConnectorEvidence
    ? plannedConnectorDoctorPreviewRows()
    : [];

  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const [retaggingSettings, setRetaggingSettings] =
    useState<CodexThreadRetaggingSettings | null>(null);
  const [retaggingBusy, setRetaggingBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    invoke<CodexThreadRetaggingSettings>("get_codex_thread_retagging_settings")
      .then((settings) => {
        if (!cancelled) setRetaggingSettings(settings);
      })
      .catch(() => {
        if (!cancelled) setRetaggingSettings(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function setRetaggingMode(mode: CodexThreadRetaggingMode) {
    setRetaggingBusy(true);
    try {
      const settings = await invoke<CodexThreadRetaggingSettings>(
        "set_codex_thread_retagging_settings",
        { settings: { codexThreadRetagging: mode } },
      );
      setRetaggingSettings(settings);
      setCopyNotice(`Codex retagging set to ${mode}.`);
    } finally {
      setRetaggingBusy(false);
    }
  }

  function renderIssue(issue: DoctorIssue) {
    const repairAction = issue.repairAction ?? null;
    const repairable = canRepairIssue(repairAction);
    const actionKind = doctorIssueActionKind(repairAction);

    return (
      <article
        key={issue.id}
        className={`switchboard-doctor__issue switchboard-doctor__issue--${issueTone(issue)} switchboard-doctor__issue--${actionKind}`}
      >
        <div>
          <div className="switchboard-doctor__issue-title">
            <strong>{issue.title}</strong>
            <span
              className={`switchboard-doctor__action-kind switchboard-doctor__action-kind--${actionKind}`}
            >
              {doctorIssueActionLabelForIssue(issue)}
            </span>
          </div>
          <p>{issue.body}</p>
          <p className="switchboard-doctor__hint">
            {doctorIssueGuidance(issue)}
          </p>
        </div>
        {repairable && repairAction !== "verify_off_mode" ? (
          <button
            type="button"
            className="switchboard-doctor__repair"
            disabled={busyAction !== null}
            onClick={() => onRepair(repairAction as string)}
          >
            {busyAction === repairAction
              ? "Working"
              : doctorRepairLabel(repairAction as string)}
          </button>
        ) : null}
      </article>
    );
  }

  async function copyDoctorReport() {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }

    await navigator.clipboard.writeText(
      formatDoctorReportShareText(doctorReport),
    );
    setCopyNotice("Copied report.");
  }

  async function copyDoctorTimeline() {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }

    await navigator.clipboard.writeText(
      formatDoctorTimelineShareText(
        buildDoctorReportTimelineEvents(
          doctorReport,
          successMessage ?? null,
          new Date().toISOString(),
        ),
      ),
    );
    setCopyNotice("Copied timeline.");
  }

  async function copyVerifyOffReport() {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }

    await navigator.clipboard.writeText(
      formatVerifyOffModeShareText(doctorReport),
    );
    setCopyNotice("Copied Verify Off.");
  }

  async function copyConnectorDossiers() {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }

    await navigator.clipboard.writeText(formatPlannedConnectorDoctorDossiers());
    setCopyNotice("Copied connector dossiers.");
  }

  async function copyRollbackCenter() {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }

    await navigator.clipboard.writeText(formatManagedRollbackInventory());
    setCopyNotice("Copied Rollback Center.");
  }

  async function copyManagedFootprint() {
    if (!navigator.clipboard || !footprintReport) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }

    await navigator.clipboard.writeText(
      formatManagedFootprintReport(footprintReport),
    );
    setCopyNotice("Copied managed footprint.");
  }

  const footprintCategories = footprintReport
    ? Array.from(
        new Set(footprintReport.items.map((item) => item.category)),
      ).sort()
    : [];

  return (
    <section
      className={`switchboard-doctor switchboard-doctor--${report.status}`}
      aria-label="Switchboard Doctor"
    >
      <div className="switchboard-doctor__head">
        <div>
          <p className="switchboard-doctor__eyebrow">Doctor</p>
          <h2>{title}</h2>
        </div>
        <div className="switchboard-doctor__head-actions">
          <span
            className={`switchboard-doctor__badge switchboard-doctor__badge--${report.status}`}
          >
            {report.status}
          </span>
          {canRepair ? (
            <button
              type="button"
              className="switchboard-doctor__repair-all"
              disabled={busyAction !== null}
              onClick={() => onRepair("repair_all")}
            >
              {busyAction === "repair_all" ? "Repairing all" : "Repair all"}
            </button>
          ) : null}
        </div>
      </div>
      <p className="switchboard-doctor__summary">{report.summary}</p>

      <div className="switchboard-doctor__action-center">
        <div>
          <span>Automatic repairs</span>
          <strong>{repairableCount}</strong>
          <small>
            {nextAutomaticIssue
              ? `Next: ${doctorRepairLabel(nextAutomaticIssue.repairAction as string)}`
              : "No direct repair is needed right now."}
          </small>
        </div>
        <div>
          <span>Manual checks</span>
          <strong>{manualCount}</strong>
          <small>
            {manualCount > 0
              ? "Review guidance; no file changes will run from these rows."
              : "No manual-only warnings."}
          </small>
        </div>
        {hasOffModeVerification ? (
          <div>
            <span>Off mode proof</span>
            <strong>Verify</strong>
            <small>Run the local listener, hook, MCP, and routing check again.</small>
            <button
              type="button"
              className="switchboard-doctor__repair-all"
              disabled={busyAction !== null}
              onClick={() => onRepair("verify_off_mode")}
            >
              {busyAction === "verify_off_mode" ? "Verifying" : "Verify Off"}
            </button>
          </div>
        ) : null}
      </div>

      {footprintReport ? (
        <section className="switchboard-doctor__report-card">
          <div>
            <span>Managed footprint</span>
            <strong>{footprintReport.items.length}</strong>
            <small>
              {footprintCategories.length > 0
                ? footprintCategories.join(", ")
                : "No managed footprint rows."}
            </small>
          </div>
          <button
            type="button"
            className="switchboard-doctor__copy"
            onClick={() => void copyManagedFootprint()}
          >
            <Copy size={14} /> Copy footprint
          </button>
        </section>
      ) : null}

      <section className="switchboard-doctor__report-card switchboard-doctor__report-card--stacked">
        <div>
          <span>Codex history retagging</span>
          <strong>{retaggingSettings?.codexThreadRetagging ?? "ask"}</strong>
          <small>
            Enables SQLite thread-provider retagging only after consent. Every
            write creates a sibling backup; unknown schemas are skipped.
          </small>
        </div>
        <div className="switchboard-doctor__segmented">
          {(["ask", "enabled", "disabled"] as CodexThreadRetaggingMode[]).map(
            (mode) => (
              <button
                key={mode}
                type="button"
                className={
                  retaggingSettings?.codexThreadRetagging === mode
                    ? "is-selected"
                    : ""
                }
                disabled={retaggingBusy}
                onClick={() => void setRetaggingMode(mode)}
              >
                {mode}
              </button>
            ),
          )}
        </div>
      </section>

      {report.issues.length > 0 ? (
        <div
          className="switchboard-doctor__triage"
          aria-label="Doctor triage summary"
        >
          <span>{repairableCount} automatic</span>
          <span>{manualCount} manual</span>
          {canRepair && manualCount > 0 ? (
            <strong>Repair all will leave manual steps visible.</strong>
          ) : null}
        </div>
      ) : null}

      <details className="switchboard-doctor__exports">
        <summary>
          <span>Export reports</span>
          <strong>{copyNotice ?? "Optional"}</strong>
        </summary>
        <div className="switchboard-doctor__export-actions">
          <button
            type="button"
            className="switchboard-doctor__copy"
            onClick={copyDoctorReport}
            title="Copy Doctor report"
          >
            <Copy aria-hidden="true" weight="bold" />
            <span>Doctor report</span>
          </button>
          <button
            type="button"
            className="switchboard-doctor__copy"
            onClick={copyDoctorTimeline}
            title="Copy Doctor timeline"
          >
            <Copy aria-hidden="true" weight="bold" />
            <span>Timeline</span>
          </button>
          <button
            type="button"
            className="switchboard-doctor__copy"
            onClick={copyRollbackCenter}
            title="Copy Rollback Center inventory"
          >
            <Copy aria-hidden="true" weight="bold" />
            <span>Rollback inventory</span>
          </button>
          {hasOffModeVerification ? (
            <button
              type="button"
              className="switchboard-doctor__copy"
              onClick={copyVerifyOffReport}
              title="Copy Verify Off report"
            >
              <Copy aria-hidden="true" weight="bold" />
              <span>Verify Off report</span>
            </button>
          ) : null}
          {hasPlannedConnectorEvidence ? (
            <button
              type="button"
              className="switchboard-doctor__copy"
              onClick={copyConnectorDossiers}
              title="Copy connector readiness dossiers"
            >
              <Copy aria-hidden="true" weight="bold" />
              <span>Connector dossiers</span>
            </button>
          ) : null}
        </div>
      </details>

      {connectorPreviewRows.length > 0 ? (
        <div
          className="switchboard-doctor__connector-preview"
          aria-label="Connector readiness preview"
        >
          <div className="switchboard-doctor__connector-preview-head">
            <strong>Connector readiness</strong>
            <span>{connectorPreviewRows.length} gated</span>
          </div>
          <div className="switchboard-doctor__connector-preview-grid">
            {connectorPreviewRows.slice(0, 6).map((connector) => (
              <div
                className="switchboard-doctor__connector-preview-row"
                key={connector.id}
              >
                <div>
                  <strong>{connector.name}</strong>
                  <span>{connector.setupPhase}</span>
                </div>
                <p>{connector.nextBlockedGate}</p>
              </div>
            ))}
          </div>
          <p className="switchboard-doctor__connector-preview-note">
            Config automation stays off until every dossier gate is verified.
          </p>
        </div>
      ) : null}

      {successMessage ? (
        <p className="switchboard-doctor__success">{successMessage}</p>
      ) : null}

      {automaticIssues.length > 0 ? (
        <div className="switchboard-doctor__issues">
          <h3>Actions</h3>
          {automaticIssues.map(renderIssue)}
        </div>
      ) : null}

      {manualIssues.length > 0 ? (
        <div className="switchboard-doctor__issues">
          <h3>Manual review</h3>
          {manualIssues.map(renderIssue)}
        </div>
      ) : null}

      {report.issues.length === 0 ? (
        <div className="switchboard-doctor__empty">
          <strong>No Doctor actions are required.</strong>
          <p>Switchboard, Off cleanup, and local setup checks are clean.</p>
        </div>
      ) : null}

      {error ? <p className="switchboard-doctor__error">{error}</p> : null}
    </section>
  );
}
