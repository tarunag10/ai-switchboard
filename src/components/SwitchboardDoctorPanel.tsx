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

function repairLabel(action: string): string {
  switch (action) {
    case "repair_runtime":
      return "Restart Headroom";
    case "reset_codex_bypass":
      return "Reset Codex";
    case "repair_client_setups":
      return "Repair clients";
    case "repair_rtk_integrations":
      return "Repair RTK";
    default:
      return "Repair";
  }
}

export function SwitchboardDoctorPanel({
report,
busyAction,
error,
successMessage,
onRepair
}: SwitchboardDoctorPanelProps) {
if (!report || (report.status === "ok" && report.issues.length === 0 && !successMessage)) {
return null;
}
const canRepair = report.issues.some((issue) => !!issue.repairAction);
const hasIssues = report.issues.length > 0;

  return (
    <section className="switchboard-doctor" aria-label="Switchboard doctor">
      <div className="switchboard-doctor__head">
<div>
<p className="switchboard-doctor__eyebrow">Doctor</p>
<h2>{hasIssues ? "Needs attention" : "Ready"}</h2>
</div>
        <span className={`switchboard-doctor__badge switchboard-doctor__badge--${report.status}`}>
          {report.status}
        </span>
</div>
<p className="switchboard-doctor__summary">{report.summary}</p>
{successMessage ? <p className="switchboard-doctor__success">{successMessage}</p> : null}
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
      <div className="switchboard-doctor__issues">
        {report.issues.map((issue) => (
          <article
            key={issue.id}
            className={`switchboard-doctor__issue switchboard-doctor__issue--${issueTone(issue)}`}
          >
            <div>
              <strong>{issue.title}</strong>
              <p>{issue.body}</p>
            </div>
            {issue.repairAction ? (
              <button
                type="button"
                className="switchboard-doctor__repair"
                disabled={busyAction !== null}
                onClick={() => onRepair(issue.repairAction as string)}
              >
                {busyAction === issue.repairAction ? "Repairing" : repairLabel(issue.repairAction)}
              </button>
            ) : null}
          </article>
        ))}
      </div>
      {error ? <p className="switchboard-doctor__error">{error}</p> : null}
    </section>
  );
}
