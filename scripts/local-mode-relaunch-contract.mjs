export const requiredModeNames = ["off", "rtk"];

export function validateModeRelaunchSummary(report) {
  const failures = [];
  if (!report || typeof report !== "object" || Array.isArray(report)) {
    return ["summary must be an object"];
  }
  if (report.schemaVersion !== 2) failures.push("schemaVersion must be 2");
  if (report.kind !== "mac_ai_switchboard.local_mode_relaunch_smoke") failures.push("unexpected summary kind");
  if (report.releaseGateEvidence !== false) failures.push("releaseGateEvidence must remain false");
  if (report.evidenceBoundary !== "config_persistence_only") failures.push("evidenceBoundary must be config_persistence_only");
  if (report.appInternalModeObserved !== false) failures.push("appInternalModeObserved must remain false");
  if (report.restored !== true) failures.push("restored must be true");
  if (!Array.isArray(report.modes) || report.modes.length !== requiredModeNames.length) {
    failures.push("modes must contain exactly off and rtk rows");
  } else {
    const names = report.modes.map((mode) => mode?.mode);
    if (JSON.stringify(names) !== JSON.stringify(requiredModeNames)) failures.push("modes must be ordered off,rtk");
    for (const mode of report.modes) {
      if (!mode || mode.pass !== true) failures.push(`${mode?.mode ?? "unknown"} mode must pass`);
      if (mode.persistedMode !== mode.mode) failures.push(`${mode?.mode ?? "unknown"} persisted mode must match requested mode`);
      if (mode.launchOk !== true || mode.appRunning !== true) failures.push(`${mode?.mode ?? "unknown"} app process evidence must pass`);
    }
  }
  if (report.passed !== true) failures.push("passed must be true");
  return failures;
}
