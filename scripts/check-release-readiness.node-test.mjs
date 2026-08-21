import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { actionForBlocker, buildReleaseReadinessActions } from "./release-readiness-actions.mjs";

test("maps signing and toolchain blockers to actionable commands", () => {
  assert.equal(actionForBlocker({ label: "missing environment: APPLE_SIGNING_IDENTITY", hint: "x" }).label, "Set Developer ID identity");
  assert.match(actionForBlocker({ label: "missing command: cargo", hint: "x" }).command, /rustup/);
});

test("deduplicates blocked release actions and includes installed smoke gaps", () => {
  const actions = buildReleaseReadinessActions({
    releaseEnv: { blockers: [{ label: "missing notarization credentials", hint: "x" }, { label: "missing notarization credentials", hint: "x" }] },
    backendValidation: { ready: false, unblockCommands: ["npm run backend-check"], message: "backend unavailable" },
    installedSmoke: { installedAppPresent: false, evidenceReady: false, missingEvidence: ["smoke summary"] },
  });
  assert.equal(actions.filter((action) => action.label === "Set notarization credentials").length, 1);
  assert.equal(actions.some((action) => action.label === "Install signed DMG"), true);
  assert.equal(actions.some((action) => action.label === "Run backend validation"), true);
});

test("ready reports produce no actions", () => {
  assert.deepEqual(buildReleaseReadinessActions({
    releaseEnv: { blockers: [] },
    backendValidation: { ready: true, unblockCommands: [], message: "" },
    installedSmoke: { installedAppPresent: true, evidenceReady: true, missingEvidence: [] },
  }), []);
});

test("no-refresh mode fails clearly when its report is absent", () => {
  const run = spawnSync(process.execPath, ["scripts/check-release-readiness.mjs", "--json", "--no-refresh", "--report", "/tmp/switchboard-release-report-that-does-not-exist.json"], { encoding: "utf8" });
  assert.equal(run.status, 1);
  assert.match(run.stderr, /release readiness report not found/);
});
