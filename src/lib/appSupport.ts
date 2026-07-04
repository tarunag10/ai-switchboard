import type { DoctorReport, RuntimeUpgradeFailure } from "./types";
import {
  buildDoctorReportTimelineEvents,
  buildManagedChangeTimelineEvents,
  sortDoctorTimelineEvents,
  type DoctorTimelineEvent,
} from "./doctorRepairCopy";
import { managedChangeRecords, type ManagedChangeRecord } from "./managedChanges";

export function sampleManagedBlock(record: ManagedChangeRecord) {
  return [
    `# >>> ${record.markerId} >>>`,
    `# Managed by AI Switchboard for ${record.owner}.`,
    "# Actual write paths fill this block from the connector adapter dry-run.",
    `# <<< ${record.markerId} <<<`,
  ].join("\n");
}

export function buildDoctorTimelinePreview(
  report: DoctorReport | null,
  successMessage: string | null,
  nowIso = new Date().toISOString(),
): DoctorTimelineEvent[] {
  return sortDoctorTimelineEvents([
    ...buildDoctorReportTimelineEvents(report, successMessage, nowIso),
    ...buildManagedChangeTimelineEvents(managedChangeRecords, nowIso),
  ]);
}

export function buildUpgradeIssueUrl(
  supportIssuesUrl: string,
  failure: RuntimeUpgradeFailure,
) {
  const subject = `AI Switchboard engine update issue (${failure.targetHeadroomVersion}, ${failure.failurePhase})`;
  const diagnosticLines = [
    `App version: ${failure.appVersion}`,
    `Target Headroom: ${failure.targetHeadroomVersion}`,
    failure.fallbackHeadroomVersion
      ? `Fallback running: ${failure.fallbackHeadroomVersion}`
      : null,
    `Failure phase: ${failure.failurePhase}`,
    `Attempts: ${failure.attempts}`,
    `First attempt: ${failure.firstAttemptAt}`,
    `Last attempt: ${failure.lastAttemptAt}`,
    `Rollback restored: ${failure.rollbackRestored ? "yes" : "no"}`,
    "",
    "Error:",
    failure.errorMessage,
  ].filter((line): line is string => line !== null);
  const body =
    "What were you doing when this happened?\n\n\n" +
    "---\n" +
    "Diagnostic info (please keep):\n" +
    diagnosticLines.join("\n");
  return `${supportIssuesUrl}/new?title=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
}
