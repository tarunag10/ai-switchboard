import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SwitchboardDoctorPanel } from "./SwitchboardDoctorPanel";

describe("SwitchboardDoctorPanel manual issue guidance", () => {
  it("labels manual issues and hides repair-all when nothing is repairable", () => {
    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Manual setup required.",
          issues: [
            {
              id: "no_headroom_clients",
              title: "No clients routed through Headroom",
              body: "Install or open a supported coding client, then return to connect it.",
              severity: "warning",
              repairAction: null,
            },
          ],
        }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    expect(screen.getByText("Manual step")).toBeInTheDocument();
    expect(
      screen.getByText(
        "No automatic repair is available yet. Follow the issue guidance, then re-run Doctor.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Repair all" }),
    ).not.toBeInTheDocument();
  });

  it("labels repairable issues as automatic", () => {
    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Repair available.",
          issues: [
            {
              id: "rtk_not_active",
              title: "RTK is not active",
              body: "Repair will install RTK.",
              severity: "warning",
              repairAction: "repair_rtk_runtime",
            },
          ],
        }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    expect(screen.getByText("Auto repair")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Install RTK" }),
    ).toBeInTheDocument();
  });
});
