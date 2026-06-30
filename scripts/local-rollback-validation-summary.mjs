import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const summaryPath = "dist/local-rollback-validation-summary.md";
const jsonPath = "dist/local-rollback-validation-summary.json";
const dedicatedCleanupDomains = [
  "managed-storage",
  "repo-intelligence",
  "login-item",
  "app-state",
  "plugins-backups",
];

const steps = [
  {
    id: "managed-changes-frontend",
    label: "Managed changes frontend tests",
    command: "npm",
    args: ["test", "--", "src/lib/managedChanges.test.ts"],
  },
  {
    id: "native-undo-all",
    label: "Native rollback undo-all backend test",
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
    label: "Gemini rollback cleanup backend test",
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
    label: "Dedicated cleanup rollback backend tests",
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
  return {
    ...step,
    fullCommand: [step.command, ...step.args].join(" "),
    startedAt,
    finishedAt: new Date().toISOString(),
    status: result.status,
    ok: result.status === 0,
    stdout: result.stdout?.trim() ?? "",
    stderr: result.stderr?.trim() ?? "",
  };
}

const generatedAt = new Date().toISOString();
const results = steps.map(runStep);
const passed = results.every((result) => result.ok);
const payload = {
  generatedAt,
  kind: "mac_ai_switchboard.local_rollback_validation",
  releaseGateEvidence: false,
  dedicatedCleanupDomains,
  passed,
  steps: results.map(({ stdout, stderr, ...result }) => ({
    ...result,
    stdoutPreview: stdout.slice(0, 2000),
    stderrPreview: stderr.slice(0, 2000),
  })),
};

const summary = `# Local Rollback Center Validation Summary

Generated: ${generatedAt}

- Evidence kind: local Rollback Center validation
- Release gate evidence: no
- Overall result: ${passed ? "pass" : "fail"}
- Dedicated cleanup domains covered: ${dedicatedCleanupDomains
  .map((domain) => `\`${domain}\``)
  .join(", ")}

${results
  .map(
    (result) => `## ${result.label}

- Command: \`${result.fullCommand}\`
- Result: ${result.ok ? "pass" : "fail"}
- Exit status: ${result.status ?? "unknown"}
`,
  )
  .join("\n")}
This smoke proves the local Rollback Center validation checks completed from the one-click evidence flow. It does not prove signed/notarized public release readiness.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, summary);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log("Local Rollback Center validation summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!passed) {
  process.exit(1);
}
