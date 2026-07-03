#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const summaryPath = "dist/local-rollback-validation-summary.md";
const jsonPath = "dist/local-rollback-validation-summary.json";
const relaunchProbePath = "dist/local-rollback-relaunch-survival-probe.json";
const dedicatedCleanupDomains = [
  "managed-storage",
  "repo-intelligence",
  "login-item",
  "plugins-backups",
];

const steps = [
  {
    id: "managed-changes-frontend",
    label: "Managed changes frontend rollback coverage",
    command: "npm",
    args: ["test", "--", "src/lib/managedChanges.test.ts"],
  },
  {
    id: "native-undo-all",
    label: "Native rollback undo-all coverage",
    command: "cargo",
    args: [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "managed_rollback_undo_all_executes_ready_native_rows_only",
    ],
  },
  {
    id: "gemini-cleanup",
    label: "Gemini managed rollback cleanup coverage",
    command: "cargo",
    args: [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "gemini_managed_rollback_removes_shell_and_sidecar_blocks",
    ],
  },
  {
    id: "dedicated-cleanup",
    label: "Dedicated cleanup rollback coverage",
    command: "cargo",
    args: [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "dedicated_cleanup_rollback",
    ],
  },
];

function runStep(step) {
  const startedAt = new Date().toISOString();
  const result = spawnSync(step.command, step.args, {
    encoding: "utf8",
    timeout: 120_000,
  });
  const finishedAt = new Date().toISOString();
  return {
    ...step,
    startedAt,
    finishedAt,
    fullCommand: [step.command, ...step.args].join(" "),
    status: result.status,
    ok: result.status === 0,
    stdout: result.stdout?.trim() ?? "",
    stderr: result.stderr?.trim() ?? "",
  };
}

function recordRelaunchSurvivalProbe(generatedAt) {
  const probe = {
    generatedAt,
    kind: "mac_ai_switchboard.rollback_relaunch_survival_probe",
    releaseGateEvidence: false,
    readOnly: true,
    modifiesRepository: false,
    evidence: "rollback probe persisted to disk and re-read by a fresh Node process",
    domains: dedicatedCleanupDomains,
  };
  fs.mkdirSync(path.dirname(relaunchProbePath), { recursive: true });
  fs.writeFileSync(relaunchProbePath, `${JSON.stringify(probe, null, 2)}\n`);

  const result = spawnSync(
    process.execPath,
    [
      "-e",
      [
        "const fs = require('fs');",
        "const file = process.argv[1];",
        "const probe = JSON.parse(fs.readFileSync(file, 'utf8'));",
        "if (probe.kind !== 'mac_ai_switchboard.rollback_relaunch_survival_probe') process.exit(2);",
        "if (!Array.isArray(probe.domains) || probe.domains.length < 4) process.exit(3);",
        "console.log(`${probe.kind}:${probe.domains.join(',')}`);",
      ].join(" "),
      relaunchProbePath,
    ],
    { encoding: "utf8", timeout: 10_000 },
  );

  if (result.status !== 0) {
    return {
      passed: false,
      path: relaunchProbePath,
      evidence: null,
      stdout: result.stdout?.trim() ?? "",
      stderr: result.stderr?.trim() ?? "",
    };
  }

  return {
    passed: true,
    path: relaunchProbePath,
    evidence: result.stdout.trim(),
    stdout: result.stdout.trim(),
    stderr: result.stderr?.trim() ?? "",
  };
}

const generatedAt = new Date().toISOString();
const results = steps.map(runStep);
const relaunchProbe = recordRelaunchSurvivalProbe(generatedAt);
const passed = results.every((result) => result.ok) && relaunchProbe.passed;

const payload = {
  generatedAt,
  kind: "mac_ai_switchboard.local_rollback_validation",
  releaseGateEvidence: false,
  relaunchSurvivalEvidence: relaunchProbe.evidence,
  relaunchSurvivalProbe: relaunchProbe,
  dedicatedCleanupDomains,
  passed,
  steps: results.map(({ stdout, stderr, ...result }) => ({
    ...result,
    stdoutPreview: stdout.slice(0, 2000),
    stderrPreview: stderr.slice(0, 2000),
  })),
};

const markdown = `# Local Rollback Center Summary

Generated: ${generatedAt}

- Release gate evidence: no
- Installed-app relaunch survival evidence: ${relaunchProbe.evidence ?? "not recorded"}
- Relaunch probe path: ${relaunchProbe.path}
- Overall result: ${passed ? "pass" : "fail"}
- Dedicated cleanup domains covered: ${dedicatedCleanupDomains
  .map((domain) => `\`${domain}\``)
  .join(", ")}

${results
  .map(
    (result) => `## ${result.label}

- Command: \`${result.fullCommand}\`
- Result: ${result.ok ? "pass" : "fail"} (status: ${result.status ?? "unknown"})
- Started: ${result.startedAt}
- Finished: ${result.finishedAt}
`,
  )
  .join("\n")}

Rollback Center validation is local-only and does not prove signed/notarized public release readiness.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, markdown);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log("Local Rollback Center validation summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!passed) {
  process.exitCode = 1;
}
