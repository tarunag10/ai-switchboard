import {
  canRepairIssue,
  doctorIssueActionKind,
  doctorIssueActionLabel,
  doctorIssueGuidance,
  doctorRepairLabel,
} from "../lib/doctorRepairCopy";
import type { DoctorIssue, DoctorReport } from "../lib/types";

interface SwitchboardDoctorPanelProps {
  report: DoctorReport | null;
  busyAction: string | null;
  error: string | null;
  successMessage?: string | null;
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
  onRepair,
}: SwitchboardDoctorPanelProps) {
  if (
    !report ||
    (report.status === "ok" && report.issues.length === 0 && !successMessage)
  ) {
    return null;
  }

  const canRepair = report.issues.some((issue) =>
    canRepairIssue(issue.repairAction),
  );
  const title = report.status === "ok" ? "Ready" : "Needs attention";

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

      {successMessage ? (
        <p className="switchboard-doctor__success">{successMessage}</p>
      ) : null}

<div className="switchboard-doctor__issues">
{report.issues.map((issue) => {
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
{doctorIssueActionLabel(repairAction)}
</span>
</div>
<p>{issue.body}</p>
<p className="switchboard-doctor__hint">
                {doctorIssueGuidance(issue)}
</p>
</div>
              {repairable ? (
                <button
                  type="button"
                  className="switchboard-doctor__repair"
                  disabled={busyAction !== null}
                  onClick={() => onRepair(repairAction as string)}
                >
                  {busyAction === repairAction
                    ? "Repairing"
                    : doctorRepairLabel(repairAction as string)}
                </button>
              ) : null}
            </article>
          );
        })}
      </div>

      {error ? <p className="switchboard-doctor__error">{error}</p> : null}
    </section>
  );
}
