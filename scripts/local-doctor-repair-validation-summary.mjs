import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const summaryPath = "dist/local-doctor-repair-validation-summary.md";
const jsonPath = "dist/local-doctor-repair-validation-summary.json";

const steps = [
  {
    id: "doctor-frontend",
    label: "Doctor frontend and copy tests",
    command: "npm",
    args: [
      "test",
      "--",
      "src/components/SwitchboardDoctorPanel.test.tsx",
      "src/lib/doctorRepairCopy.test.ts",
      "src/lib/doctorRepairClassification.test.ts",
    ],
  },
  {
    id: "off-mode-guard",
    label: "Off-mode Doctor repair guard backend test",
    command: "cargo",
    args: [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "off_mode_blocks_doctor_repairs_that_restore_headroom",
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
  kind: "mac_ai_switchboard.local_doctor_repair_validation",
  releaseGateEvidence: false,
  passed,
  steps: results.map(({ stdout, stderr, ...result }) => ({
    ...result,
    stdoutPreview: stdout.slice(0, 2000),
    stderrPreview: stderr.slice(0, 2000),
  })),
};

const summary = `# Local Doctor Repair Validation Summary

Generated: ${generatedAt}

- Evidence kind: local Doctor repair validation
- Release gate evidence: no
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
This smoke proves the local Doctor repair validation checks completed from the one-click evidence flow. It does not prove signed/notarized public release readiness.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, summary);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log("Local Doctor repair validation summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!passed) {
  process.exit(1);
}
