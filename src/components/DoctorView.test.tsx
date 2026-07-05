import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { DoctorReport, ManagedFootprintReport } from "../lib/types";
import { DoctorView } from "./DoctorView";

const report = {
  status: "warning",
  issues: [
    {
      id: "codex_provider_mismatch",
      title: "Codex routing config needs repair",
      body: "The provider block drifted.",
      severity: "warning",
      repairAction: "repair_codex_setup",
    },
  ],
} as unknown as DoctorReport;

const footprintReport = {
  generatedAt: "2026-07-05T10:00:00.000Z",
  items: [],
} as ManagedFootprintReport;

describe("DoctorView", () => {
  it("renders doctor repair and timeline sections", () => {
    render(
      <DoctorView
        hidden={false}
        report={report}
        busyAction={null}
        error={null}
        successMessage="Repair complete."
        footprintReport={footprintReport}
        onRepair={vi.fn()}
        timelineEvents={[
          {
            id: "success",
            kind: "repair",
            status: "ok",
          } as never,
        ]}
      />,
    );

    expect(screen.getByRole("heading", { name: "Doctor" })).toBeInTheDocument();
    expect(screen.getByText("Codex routing config needs repair")).toBeInTheDocument();
    expect(screen.getByText("Repair complete.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copy timeline" })).toBeInTheDocument();
  });
});
