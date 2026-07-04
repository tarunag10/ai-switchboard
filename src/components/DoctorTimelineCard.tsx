import { useState } from "react";
import type { DoctorTimelineEvent } from "../lib/doctorRepairCopy";
import {
  doctorTimelineKindLabel,
  formatDoctorTimelineShareText,
} from "../lib/doctorRepairCopy";

export function DoctorTimelineCard({
  events,
}: {
  events: DoctorTimelineEvent[];
}) {
  const [copyNotice, setCopyNotice] = useState<string | null>(null);

  async function copyTimeline() {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }
    await navigator.clipboard.writeText(formatDoctorTimelineShareText(events));
    setCopyNotice("Copied timeline.");
    window.setTimeout(() => setCopyNotice(null), 2500);
  }

  return (
    <article className="soft-card doctor-timeline">
      <div className="doctor-timeline__head">
        <div>
          <span>Doctor timeline</span>
          <strong>{events.length} event{events.length === 1 ? "" : "s"}</strong>
        </div>
        <button
          className="secondary-button secondary-button--small"
          onClick={() => void copyTimeline()}
          type="button"
        >
          {copyNotice ?? "Copy timeline"}
        </button>
      </div>
      <div className="doctor-timeline__list">
        {events.map((event) => (
          <div className="doctor-timeline__event" key={event.id}>
            <div>
              <strong>{event.title}</strong>
              <span>{event.body}</span>
            </div>
            <div>
              <span>{doctorTimelineKindLabel(event.kind)}</span>
              <span>{event.status}</span>
            </div>
          </div>
        ))}
      </div>
    </article>
  );
}
