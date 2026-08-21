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

test("no-refresh mode fails cleanly for corrupt JSON without rewriting it", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-release-corrupt-"));
  const reportPath = path.join(tempDir, "corrupt-report.json");
  const before = "{\"status\":\"blocked\",";
  fs.writeFileSync(reportPath, before);
  const run = spawnSync(process.execPath, [
    "scripts/check-release-readiness.mjs",
    "--json",
    "--no-refresh",
    "--report",
    reportPath,
  ], { encoding: "utf8" });
  assert.equal(run.status, 1);
  assert.equal(run.stdout, "");
  assert.match(run.stderr, /release readiness report invalid JSON/);
  assert.equal(fs.readFileSync(reportPath, "utf8"), before);
  fs.rmSync(tempDir, { recursive: true, force: true });
});

test("rejects malformed readiness reports before action mapping", () => {
  const failures = validateReleaseReadinessReport({ status: "blocked", releaseEnv: {}, backendValidation: {}, installedSmoke: {} });
  assert.match(failures.join("\n"), /releaseEnv.blockers/);
  assert.match(failures.join("\n"), /backendValidation.ready/);
  assert.match(failures.join("\n"), /installedSmoke/);
});

test("rejects malformed blocker entries before action mapping", () => {
  const failures = validateReleaseReadinessReport({
    status: "blocked",
    releaseEnv: { blockers: [null, {}, { label: "", hint: 42 }, { label: 7, hint: "ok" }] },
    backendValidation: { ready: true },
    installedSmoke: { installedAppPresent: true, evidenceReady: true, missingEvidence: [] },
  });
  assert.match(failures.join("\n"), /blockers\[0\] must be an object/);
  assert.match(failures.join("\n"), /blockers\[1\]\.label must be a non-empty string/);
  assert.match(failures.join("\n"), /blockers\[2\]\.hint must be a string/);
  assert.match(failures.join("\n"), /blockers\[3\]\.label must be a non-empty string/);
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

test("strict blocked readiness exits non-zero without rewriting the report", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-release-strict-blocked-"));
  const reportPath = path.join(tempDir, "blocked-report.json");
  const report = {
    status: "blocked",
    releaseEnv: { blockers: [{ label: "missing signing evidence", hint: "signing required" }] },
    backendValidation: { ready: true, unblockCommands: [], message: "" },
    installedSmoke: { installedAppPresent: true, evidenceReady: true, missingEvidence: [] },
  };
  fs.writeFileSync(reportPath, JSON.stringify(report));
  const before = fs.readFileSync(reportPath, "utf8");
  const run = spawnSync(process.execPath, [
    "scripts/check-release-readiness.mjs",
    "--strict",
    "--json",
    "--no-refresh",
    "--report",
    reportPath,
  ], { encoding: "utf8" });
  assert.equal(run.status, 1);
  assert.equal(JSON.parse(run.stdout).status, "blocked");
  assert.equal(JSON.parse(run.stdout).strict, true);
  assert.equal(fs.readFileSync(reportPath, "utf8"), before);
  fs.rmSync(tempDir, { recursive: true, force: true });
});

test("strict ready readiness exits zero without rewriting the report", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-release-strict-ready-"));
  const reportPath = path.join(tempDir, "ready-report.json");
  const report = {
    status: "ready",
    releaseEnv: { blockers: [] },
    backendValidation: { ready: true, unblockCommands: [], message: "" },
    installedSmoke: { installedAppPresent: true, evidenceReady: true, missingEvidence: [] },
  };
  fs.writeFileSync(reportPath, JSON.stringify(report));
  const before = fs.readFileSync(reportPath, "utf8");
  const run = spawnSync(process.execPath, [
    "scripts/check-release-readiness.mjs",
    "--strict",
    "--json",
    "--no-refresh",
    "--report",
    reportPath,
  ], { encoding: "utf8" });
  assert.equal(run.status, 0);
  const result = JSON.parse(run.stdout);
  assert.equal(result.status, "ready");
  assert.deepEqual(result.actions, []);
  assert.equal(fs.readFileSync(reportPath, "utf8"), before);
  fs.rmSync(tempDir, { recursive: true, force: true });
});
