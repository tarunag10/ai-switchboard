import test from "node:test";
import assert from "node:assert/strict";
import { validateModeRelaunchSummary } from "./local-mode-relaunch-contract.mjs";

function validSummary() {
  return {
    schemaVersion: 2,
    kind: "mac_ai_switchboard.local_mode_relaunch_smoke",
    releaseGateEvidence: false,
    evidenceBoundary: "config_persistence_only",
    appInternalModeObserved: false,
    restored: true,
    passed: true,
    modes: [
      { mode: "off", pass: true, launchOk: true, appRunning: true, persistedMode: "off" },
      { mode: "rtk", pass: true, launchOk: true, appRunning: true, persistedMode: "rtk" },
    ],
  };
}

test("accepts an explicit config-persistence-only mode summary", () => {
  assert.deepEqual(validateModeRelaunchSummary(validSummary()), []);
});

test("rejects wrong schema, missing mode, mismatched persistence, and claimed app internals", () => {
  const report = validSummary();
  report.schemaVersion = 1;
  report.modes.pop();
  report.appInternalModeObserved = true;
  assert.match(validateModeRelaunchSummary(report).join("\n"), /schemaVersion|modes|appInternalModeObserved/);

  const mismatch = validSummary();
  mismatch.modes[1].persistedMode = "off";
  assert.match(validateModeRelaunchSummary(mismatch).join("\n"), /persisted mode must match/);
});
