import type { DoctorReport, ManagedFootprintReport } from "./types";

export type DoctorRepairInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

export interface RefreshDoctorReportOptions {
  invoke: DoctorRepairInvoke;
  setDoctorReport: (report: DoctorReport | null) => void;
  setManagedFootprintReport: (report: ManagedFootprintReport | null) => void;
}

export interface RunDoctorRepairActionOptions {
  currentBusyAction: string | null;
  invoke: DoctorRepairInvoke;
  refreshSwitchboardState: () => Promise<void> | void;
  setDoctorRepairBusy: (action: string | null) => void;
  setDoctorRepairError: (message: string | null) => void;
  setDoctorRepairSuccess: (message: string | null) => void;
  setDoctorReport: (report: DoctorReport | null) => void;
}

export async function refreshDoctorReport({
  invoke,
  setDoctorReport,
  setManagedFootprintReport,
}: RefreshDoctorReportOptions) {
  try {
    const [report, footprint] = await Promise.all([
      invoke<DoctorReport>("get_doctor_report"),
      invoke<ManagedFootprintReport>("get_managed_footprint").catch(() => null),
    ]);
    setDoctorReport(report);
    setManagedFootprintReport(footprint);
  } catch {
    setDoctorReport(null);
    setManagedFootprintReport(null);
  }
}

export async function runDoctorRepairAction(
  action: string,
  {
    currentBusyAction,
    invoke,
    refreshSwitchboardState,
    setDoctorRepairBusy,
    setDoctorRepairError,
    setDoctorRepairSuccess,
    setDoctorReport,
  }: RunDoctorRepairActionOptions,
) {
  if (currentBusyAction) {
    return;
  }

  setDoctorRepairBusy(action);
  setDoctorRepairError(null);
  setDoctorRepairSuccess(null);

  try {
    const report = await invoke<DoctorReport>("run_doctor_repair", { action });
    setDoctorReport(report);
    setDoctorRepairSuccess(doctorRepairSuccessMessage(action, report));
    await refreshSwitchboardState();
  } catch (error) {
    setDoctorRepairError(error instanceof Error ? error.message : "Could not run repair.");
  } finally {
    setDoctorRepairBusy(null);
  }
}

export function doctorRepairSuccessMessage(action: string, report: DoctorReport) {
  if (action === "verify_off_mode") {
    return "Off mode verification refreshed.";
  }
  return report.status === "ok" && report.issues.length === 0
    ? "Repair complete. Switchboard looks ready."
    : "Repair finished. Review the remaining Doctor items.";
}
