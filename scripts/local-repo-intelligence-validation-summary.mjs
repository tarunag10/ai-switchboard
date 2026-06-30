import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const summaryPath = "dist/local-repo-intelligence-validation-summary.md";
const jsonPath = "dist/local-repo-intelligence-validation-summary.json";

const steps = [
  {
    id: "repo-intelligence-frontend",
    label: "Repo Intelligence frontend pack and safety tests",
    command: "npm",
    args: ["test", "--", "src/lib/repoIntelligence.test.ts"],
  },
  {
    id: "repo-intelligence-backend",
    label: "Repo Intelligence backend read-only API tests",
    command: "cargo",
    args: [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "repo_intelligence",
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
  kind: "mac_ai_switchboard.local_repo_intelligence_validation",
  releaseGateEvidence: false,
  readOnly: true,
  modifiesRepository: false,
  passed,
  steps: results.map(({ stdout, stderr, ...result }) => ({
    ...result,
    stdoutPreview: stdout.slice(0, 2000),
    stderrPreview: stderr.slice(0, 2000),
  })),
};

const summary = `# Local Repo Intelligence Validation Summary

Generated: ${generatedAt}

- Evidence kind: local Repo Intelligence validation
- Release gate evidence: no
- Read-only: yes
- Modifies repository: no
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
This smoke proves Repo Intelligence pack generation and read-only API payloads completed locally. Repo Memory MCP bridge checks are recorded separately by npm run smoke:repo-memory-mcp:local. It does not mutate the user repository and does not prove signed/notarized public release readiness.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, summary);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log("Local Repo Intelligence validation summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!passed) {
  process.exit(1);
}
