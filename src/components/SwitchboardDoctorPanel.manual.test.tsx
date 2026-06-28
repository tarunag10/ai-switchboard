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
    expect(screen.getByText("0 automatic")).toBeInTheDocument();
    expect(screen.getByText("1 manual")).toBeInTheDocument();
    expect(
      screen.getByText(
        "No automatic repair is available yet. Follow the issue guidance, then re-run Doctor.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Repair all" }),
    ).not.toBeInTheDocument();
  });

  it("separates manual connector guidance from automatic Repo Intelligence cleanup", () => {
    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Mixed setup required.",
          issues: [
            {
              id: "planned_connectors_detected",
              title: "Planned coding tools detected",
              body: "Gemini CLI detected.",
              severity: "warning",
              repairAction: null,
            },
            {
              id: "repo_intelligence_repo_missing",
              title: "Repo Intelligence index points to a missing folder",
              body: "The last indexed path is gone.",
              severity: "warning",
              repairAction: "clear_repo_intelligence_index",
            },
            {
              id: "repo_intelligence_stale",
              title: "Repo Intelligence index is stale",
              body: "The last index is more than 7 days old.",
              severity: "warning",
              repairAction: "clear_repo_intelligence_index",
            },
            {
              id: "headroom_paused",
              title: "Headroom engine is paused",
              body: "The proxy is intentionally off.",
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

    expect(screen.getByText("2 automatic")).toBeInTheDocument();
    expect(screen.getByText("2 manual")).toBeInTheDocument();
    expect(
      screen.getByText(/review each planned connector's detection evidence/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/next automation gate is backup implemented/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Manual only, Automation gated/i),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("Planned connector readiness preview"),
    ).toBeInTheDocument();
    expect(screen.getByText("Connector readiness")).toBeInTheDocument();
    expect(screen.getByText("11 planned")).toBeInTheDocument();
    expect(screen.getByText("Gemini CLI")).toBeInTheDocument();
    expect(screen.getByText("OpenCode")).toBeInTheDocument();
    expect(screen.getAllByText("Backup Implemented").length).toBeGreaterThan(0);
    expect(
      screen.getByText(
        "Config automation stays off until every dossier gate is verified.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getAllByText(
        "Clears the saved Repo Intelligence summary so stale, missing, moved, or replaced repo paths no longer appear in Doctor. Re-index the current local repo path from Addons when ready.",
      ),
    ).toHaveLength(2);
    expect(
      screen.getByText(
        "Choose Full optimization or Headroom only to resume routing, or stay in Off mode if you want clients to bypass Headroom.",
      ),
    ).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Clear index" })).toHaveLength(
      2,
    );
  });

  it("warns repair all will leave manual follow-up", () => {
    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Mixed setup required.",
          issues: [
            {
              id: "rtk_not_active",
              title: "RTK is not active",
              body: "Repair will install RTK.",
              severity: "warning",
              repairAction: "repair_rtk_runtime",
            },
            {
              id: "planned_connectors_detected",
              title: "Planned coding tools detected",
              body: "Gemini CLI detected.",
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

    expect(screen.getByText("1 automatic")).toBeInTheDocument();
    expect(screen.getByText("1 manual")).toBeInTheDocument();
    expect(
      screen.getByText("Repair all will leave manual steps visible."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Repair all" }),
    ).toBeInTheDocument();
  });

  it("labels repairable issues automatic", () => {
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
