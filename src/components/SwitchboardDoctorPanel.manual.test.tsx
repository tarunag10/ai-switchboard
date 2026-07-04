import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SwitchboardDoctorPanel } from "./SwitchboardDoctorPanel";

describe("SwitchboardDoctorPanel manual issue guidance", () => {
  it("labels manual issues and hides repair-all when nothing is repairable", () => {
    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Approval required.",
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

    expect(screen.getByText("Approval needed")).toBeInTheDocument();
    expect(screen.getByText("0 automatic")).toBeInTheDocument();
    expect(screen.getByText("1 approval")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Open or install a supported app-managed client, then use Auto-fix setup or Doctor to apply reversible setup. Doctor repair becomes available after a supported client is detected.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Repair all" }),
    ).not.toBeInTheDocument();
  });

  it("separates manual connector guidance from automatic Repo Intelligence cleanup", async () => {
    const user = userEvent.setup();
    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Mixed setup required.",
          issues: [
            {
              id: "planned_connectors_detected",
              title: "Gated connector readiness detected",
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
    expect(screen.getByText("2 approval")).toBeInTheDocument();
    expect(
      screen.getByText(/review each detected connector's evidence/i),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("Connector readiness preview"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(
        "Dry-run target: User/settings.json; marker: mac-ai-switchboard:cursor",
      ),
    ).not.toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Show details" })[1]);

    expect(
      screen.getByText(
        "Dry-run target: User/settings.json; marker: mac-ai-switchboard:cursor",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/RTK-only mode/i)).toBeInTheDocument();
    expect(
      screen.getByText(
        "Clear the saved Repo Intelligence index, then open Addons and index an available local repo when ready.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Clear the stale saved Repo Intelligence index, then open Addons and re-index the repo before copying packs into another agent.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Choose Full optimization or Headroom only to resume routing, or stay in Off mode if you want clients to bypass Headroom.",
      ),
    ).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Clear index" })).toHaveLength(
      2,
    );
  });

  it("treats corrupt Repo Intelligence storage as automatic cleanup", () => {
    const onRepair = vi.fn();

    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Repo Intelligence storage needs cleanup.",
          issues: [
            {
              id: "repo_intelligence_storage_corrupt",
              title: "Repo Intelligence index cannot be read",
              body: "The saved Repo Intelligence index could not be parsed.",
              severity: "warning",
              repairAction: "clear_repo_intelligence_index",
            },
          ],
        }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={onRepair}
      />,
    );

    expect(screen.getByText("1 automatic")).toBeInTheDocument();
    expect(screen.getByText("0 approval")).toBeInTheDocument();
    expect(screen.getByText("Auto repair")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Use Clear index to remove the corrupt or unreadable saved Repo Intelligence summary from Switchboard managed storage, then open Addons and re-index a local repo before copying packs into another agent.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Clear index" }),
    ).toBeInTheDocument();
  });

  it("warns repair all will leave approval follow-up", () => {
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
              title: "Gated connector readiness detected",
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
    expect(screen.getByText("1 approval")).toBeInTheDocument();
    expect(
      screen.getByText("Auto-fix will leave approval-only steps visible."),
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
