import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { actionForBlocker, buildReleaseReadinessActions, validateReleaseReadinessReport } from "./release-readiness-actions.mjs";

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

test("rejects malformed readiness reports before action mapping", () => {
  const failures = validateReleaseReadinessReport({ status: "blocked", releaseEnv: {}, backendValidation: {}, installedSmoke: {} });
  assert.match(failures.join("\n"), /releaseEnv.blockers/);
  assert.match(failures.join("\n"), /backendValidation.ready/);
  assert.match(failures.join("\n"), /installedSmoke/);
});

test("requires a value after --report", () => {
  const run = spawnSync(process.execPath, ["scripts/check-release-readiness.mjs", "--no-refresh", "--report", "--json"], { encoding: "utf8" });
  assert.equal(run.status, 1);
  assert.match(run.stderr, /--report requires a file path/);
});

test("rehearses blocked no-refresh action mapping without rewriting the report", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-release-rehearsal-"));
  const reportPath = path.join(tempDir, "blocked-report.json");
  const report = {
    status: "blocked",
    releaseEnv: {
      blockers: [
        { label: "missing environment: APPLE_SIGNING_IDENTITY", hint: "identity required" },
        { label: "missing command: cargo", hint: "cargo required" },
      ],
    },
    backendValidation: {
      ready: false,
      unblockCommands: ["rustup --version"],
      message: "backend unavailable",
    },
    installedSmoke: {
      installedAppPresent: false,
      evidenceReady: false,
      missingEvidence: ["smoke summary"],
    },
  };
  fs.writeFileSync(reportPath, JSON.stringify(report));
  const before = fs.readFileSync(reportPath, "utf8");
  const run = spawnSync(process.execPath, [
    "scripts/check-release-readiness.mjs",
    "--json",
    "--no-refresh",
    "--report",
    reportPath,
  ], { encoding: "utf8" });
  assert.equal(run.status, 0);
  const result = JSON.parse(run.stdout);
  assert.equal(result.status, "blocked");
  assert.deepEqual(result.actions.map((action) => action.label), [
    "Set Developer ID identity",
    "Install Rust toolchain",
    "Run backend validation",
    "Install signed DMG",
    "Record installed smoke evidence",
  ]);
  assert.equal(fs.readFileSync(reportPath, "utf8"), before);
  fs.rmSync(tempDir, { recursive: true, force: true });
});
