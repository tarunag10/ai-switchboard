import { ArrowClockwise, Copy, Terminal } from "@phosphor-icons/react";

import {
  formatReleaseReadinessNextAction,
  formatReleaseReadinessSourceLabel,
  releaseReadinessGroups,
  releaseReadinessItemCount,
  releaseShareableGates,
  type ReleaseReadinessNextAction,
} from "../lib/releaseReadiness";

import type { ReleaseReadinessReportSnapshot } from "../lib/releaseReadiness";

export interface ReleaseEvidenceResult {
  commandId: string;
  command: string;
  summaryPath: string | null;
  stdout: string;
  stderr: string;
}

export interface ReleaseReadinessReportPayload {
  reportPath: string | null;
  report: ReleaseReadinessReportSnapshot | null;
}

interface ReleaseReadinessRow {
  id: string;
  label: string;
  detail: string;
  source: string;
  statusLabel: string;
  tone: string;
}

interface ReleaseLocalEvidenceRow {
  id: string;
  label: string;
  detail: string;
  statusLabel: string;
  passed: boolean;
  command: string | null;
  summaryPath: string | null;
}

interface ReleaseReadinessCounts {
  ready: number;
  blocked: number;
  "local-only": number;
}

interface SettingsReleaseReadinessCardProps {
  releaseEvidenceBusyId: string | null;
  releaseEvidenceResult?: ReleaseEvidenceResult | null;
  releaseLocalEvidenceRows: ReleaseLocalEvidenceRow[];
  releaseReadinessAction: ReleaseReadinessNextAction | null | string;
  releaseReadinessCommandProp: string;
  releaseReadinessCopyNotice: string | null;
  releaseReadinessCounts: ReleaseReadinessCounts;
  releaseReadinessError: string | null;
  releaseReadinessEvidence: { copy: string };
  releaseReadinessRefreshing: boolean;
  releaseReadinessReport: ReleaseReadinessReportPayload | null;
  releaseReadinessRows: ReleaseReadinessRow[];
  copyReleaseReadinessReport: () => void | Promise<void>;
  formatLocalReleaseEvidenceSequenceCopy: () => string;
  refreshReleaseReadinessReport: () => void | Promise<void>;
  runLocalReleaseEvidenceSequence: () => void | Promise<void>;
  runReleaseEvidenceCommand: (commandId: string) => void | Promise<void>;
}

export function SettingsReleaseReadinessCard({
  copyReleaseReadinessReport,
  formatLocalReleaseEvidenceSequenceCopy,
  refreshReleaseReadinessReport,
  releaseEvidenceBusyId,
  releaseEvidenceResult,
  releaseLocalEvidenceRows,
  releaseReadinessAction,
  releaseReadinessCommandProp,
  releaseReadinessCopyNotice,
  releaseReadinessCounts,
  releaseReadinessError,
  releaseReadinessEvidence,
  releaseReadinessRefreshing,
  releaseReadinessReport,
  releaseReadinessRows,
  runLocalReleaseEvidenceSequence,
  runReleaseEvidenceCommand,
}: SettingsReleaseReadinessCardProps) {
  const releaseReadinessNextActionCopy =
    typeof releaseReadinessAction === "string"
      ? releaseReadinessAction
      : formatReleaseReadinessNextAction(releaseReadinessAction);

  return (
    <article
      className="soft-card panel-card release-readiness-card"
      id="release-readiness"
    >
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
        {releaseReadinessNextActionCopy}
      </p>
      {releaseReadinessError ? (
        <p className="release-readiness-card__error">{releaseReadinessError}</p>
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
              <div className="release-readiness-card__status-row" key={row.id}>
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
                      onClick={() => void runReleaseEvidenceCommand(item.id)}
                      type="button"
                    >
                      <ArrowClockwise size={14} weight="bold" />
                      {releaseEvidenceBusyId === item.id
                        ? "Running"
                        : "Run evidence"}
                    </button>
                  ) : null}
                  {releaseEvidenceResult?.commandId === item.id ? (
                    <span className="release-readiness-card__evidence-result">
                      Generated{" "}
                      {releaseEvidenceResult.summaryPath ??
                        releaseEvidenceResult.command}
                    </span>
                  ) : null}
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
      {releaseReadinessCopyNotice ? (
        <p className="connector-copy-notice">{releaseReadinessCopyNotice}</p>
      ) : null}
    </article>
  );
}
