import type { ComponentProps } from "react";

import { DoctorTimelineCard } from "./DoctorTimelineCard";
import { SwitchboardDoctorPanel } from "./SwitchboardDoctorPanel";

type DoctorPanelProps = ComponentProps<typeof SwitchboardDoctorPanel>;
type DoctorTimelineProps = ComponentProps<typeof DoctorTimelineCard>;

export interface DoctorViewProps {
  hidden: boolean;
  report: DoctorPanelProps["report"];
  busyAction: DoctorPanelProps["busyAction"];
  error: DoctorPanelProps["error"];
  successMessage: DoctorPanelProps["successMessage"];
  footprintReport: DoctorPanelProps["footprintReport"];
  onRepair: DoctorPanelProps["onRepair"];
  timelineEvents: DoctorTimelineProps["events"];
}

export function DoctorView({
  hidden,
  report,
  busyAction,
  error,
  successMessage,
  footprintReport,
  onRepair,
  timelineEvents,
}: DoctorViewProps) {
  return (
    <div className="tray-content" hidden={hidden}>
      <section className="repo-intelligence-view">
        <header className="repo-intelligence-view__header">
          <div>
            <h1>Doctor</h1>
            <p className="repo-intelligence-view__subtitle">
              Inspect AI Switchboard setup, run fixes, copy reports, and repair local routing drift.
            </p>
          </div>
          <span className="repo-intelligence-view__badge">Fixes</span>
        </header>
        <SwitchboardDoctorPanel
          report={report}
          busyAction={busyAction}
          error={error}
          successMessage={successMessage}
          footprintReport={footprintReport}
          onRepair={onRepair}
        />
        <DoctorTimelineCard events={timelineEvents} />
      </section>
    </div>
  );
}

