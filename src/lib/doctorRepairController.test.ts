import { describe, expect, it, vi } from "vitest";

import {
  doctorRepairSuccessMessage,
  refreshDoctorReport,
  runDoctorRepairAction,
  type DoctorRepairInvoke,
} from "./doctorRepairController";
import type { DoctorReport, ManagedFootprintReport } from "./types";

const healthyReport: DoctorReport = {
  status: "ok",
  summary: "Ready",
  issues: [],
};

const warningReport: DoctorReport = {
  status: "warning",
  summary: "Needs attention",
  issues: [],
};

const okReportWithRemainingIssue: DoctorReport = {
  status: "ok",
  summary: "Mostly ready",
  issues: [
    {
      id: "remaining",
      title: "Remaining issue",
      body: "Doctor still has something to show.",
      severity: "warning",
    },
  ],
};

const footprintReport: ManagedFootprintReport = {
  generatedAt: "2026-07-05T00:00:00Z",
  items: [],
};

function repairOptions(invoke: DoctorRepairInvoke, currentBusyAction: string | null = null) {
  return {
    currentBusyAction,
    invoke,
    refreshSwitchboardState: vi.fn(),
    setDoctorRepairBusy: vi.fn(),
    setDoctorRepairError: vi.fn(),
    setDoctorRepairSuccess: vi.fn(),
    setDoctorReport: vi.fn(),
  };
}

describe("doctorRepairController", () => {
  it("refreshes Doctor and managed footprint reports together", async () => {
    const invoke = vi.fn(async (command: string) => {
      if (command === "get_doctor_report") return healthyReport;
      if (command === "get_managed_footprint") return footprintReport;
      throw new Error(`unexpected command ${command}`);
    }) as DoctorRepairInvoke;
    const setDoctorReport = vi.fn();
    const setManagedFootprintReport = vi.fn();

    await refreshDoctorReport({
      invoke,
      setDoctorReport,
      setManagedFootprintReport,
    });

    expect(invoke).toHaveBeenCalledWith("get_doctor_report");
    expect(invoke).toHaveBeenCalledWith("get_managed_footprint");
    expect(setDoctorReport).toHaveBeenCalledWith(healthyReport);
    expect(setManagedFootprintReport).toHaveBeenCalledWith(footprintReport);
  });

  it("clears reports when Doctor refresh fails", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("offline");
    }) as DoctorRepairInvoke;
    const setDoctorReport = vi.fn();
    const setManagedFootprintReport = vi.fn();

    await refreshDoctorReport({
      invoke,
      setDoctorReport,
      setManagedFootprintReport,
    });

    expect(setDoctorReport).toHaveBeenCalledWith(null);
    expect(setManagedFootprintReport).toHaveBeenCalledWith(null);
  });

  it("runs the selected Doctor repair and refreshes Switchboard state after success", async () => {
    const invoke = vi.fn(async () => warningReport) as DoctorRepairInvoke;
    const options = repairOptions(invoke);

    await runDoctorRepairAction("repair_codex_setup", options);

    expect(invoke).toHaveBeenCalledWith("run_doctor_repair", {
      action: "repair_codex_setup",
    });
    expect(options.setDoctorRepairBusy).toHaveBeenNthCalledWith(
      1,
      "repair_codex_setup",
    );
    expect(options.setDoctorRepairError).toHaveBeenCalledWith(null);
    expect(options.setDoctorRepairSuccess).toHaveBeenNthCalledWith(1, null);
    expect(options.setDoctorReport).toHaveBeenCalledWith(warningReport);
    expect(options.setDoctorRepairSuccess).toHaveBeenLastCalledWith(
      "Repair finished. Review the remaining Doctor items.",
    );
    expect(options.refreshSwitchboardState).toHaveBeenCalledTimes(1);
    expect(options.setDoctorRepairBusy).toHaveBeenLastCalledWith(null);
  });

  it("does not start a second repair while another action is busy", async () => {
    const invoke = vi.fn(async () => healthyReport) as DoctorRepairInvoke;
    const options = repairOptions(invoke, "repair_runtime");

    await runDoctorRepairAction("repair_codex_setup", options);

    expect(invoke).not.toHaveBeenCalled();
    expect(options.setDoctorRepairBusy).not.toHaveBeenCalled();
  });

  it("surfaces repair failures and clears busy state", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("repair failed");
    }) as DoctorRepairInvoke;
    const options = repairOptions(invoke);

    await runDoctorRepairAction("repair_runtime", options);

    expect(options.setDoctorRepairError).toHaveBeenLastCalledWith("repair failed");
    expect(options.refreshSwitchboardState).not.toHaveBeenCalled();
    expect(options.setDoctorRepairBusy).toHaveBeenLastCalledWith(null);
  });

  it("keeps existing repair success copy", () => {
    expect(doctorRepairSuccessMessage("verify_off_mode", warningReport)).toBe(
      "Off mode verification refreshed.",
    );
    expect(doctorRepairSuccessMessage("repair_runtime", healthyReport)).toBe(
      "Repair complete. Switchboard looks ready.",
    );
    expect(
      doctorRepairSuccessMessage("repair_runtime", okReportWithRemainingIssue),
    ).toBe("Repair finished. Review the remaining Doctor items.");
    expect(doctorRepairSuccessMessage("repair_runtime", warningReport)).toBe(
      "Repair finished. Review the remaining Doctor items.",
    );
  });
});
