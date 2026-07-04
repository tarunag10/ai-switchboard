import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const summaryPath = "dist/local-uninstall-validation-summary.md";
const jsonPath = "dist/local-uninstall-validation-summary.json";

const steps = [
  {
    id: "uninstall-disclosure-frontend",
    label: "Uninstall disclosure frontend tests",
    command: "npm",
    args: ["test", "--", "src/lib/uninstallDisclosure.test.ts"],
  },
  {
    id: "uninstall-dry-run-backend",
    label: "Backend uninstall dry-run target inventory test",
    command: "cargo",
    args: [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "uninstall_dry_run_lists_current_cleanup_targets",
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
  schemaVersion: 1,
  generatedAt,
  kind: "mac_ai_switchboard.local_uninstall_validation",
  releaseGateEvidence: false,
  destructive: false,
  passed,
  steps: results.map(({ stdout, stderr, ...result }) => ({
    ...result,
    stdoutPreview: stdout.slice(0, 2000),
    stderrPreview: stderr.slice(0, 2000),
  })),
};

const summary = `# Local Uninstall Validation Summary

Generated: ${generatedAt}

- Evidence kind: local uninstall validation
- Release gate evidence: no
- Destructive actions: no
- Overall result: ${passed ? "pass" : "fail"}

${results
  .map(
    (result) => `## ${result.label}

- Command: \`${result.fullCommand}\`
- Result: ${result.ok ? "pass" : "fail"}
- Exit status: ${result.status ?? "unknown"}
`,
  )
  .join("\n")}
This smoke proves the local uninstall disclosure and backend dry-run target inventory are current. It does not delete files and does not prove signed/notarized public release readiness.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, summary);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log("Local uninstall validation summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!passed) {
  process.exit(1);
}
