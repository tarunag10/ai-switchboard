import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const summaryPath = "dist/local-only-network-validation-summary.md";
const jsonPath = "dist/local-only-network-validation-summary.json";
const coverage = {
  guardSurfaces: 6,
  appOwnedRemoteServiceSurfaces: 4,
  providerTrafficSurfaces: 2,
  managedDownloadSurfaces: 2,
  forbiddenRemoteFragments: ["buy.polar.sh", "app.aptabase.com", "clarity.ms", "api.headroom"],
};

const steps = [
  {
    id: "local-only-network-certification",
    label: "Local-only network certification",
    command: "npm",
    args: ["run", "check:local-only-network"],
  },
  {
    id: "local-build-privacy-scan",
    label: "Local-free build privacy scan",
    command: "npm",
    args: ["run", "check:local-build-privacy"],
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
  kind: "mac_ai_switchboard.local_only_network_validation",
  releaseGateEvidence: false,
  localOnly: true,
  appOwnedRemoteCallsBlocked: passed,
  coverage,
  passed,
  steps: results.map(({ stdout, stderr, ...result }) => ({
    ...result,
    stdoutPreview: stdout.slice(0, 2000),
    stderrPreview: stderr.slice(0, 2000),
  })),
};

const summary = `# Local-Only Network Validation Summary

Generated: ${generatedAt}

- Evidence kind: local-only network validation
- Release gate evidence: no
- Local-only: yes
- App-owned remote calls blocked: ${passed ? "yes" : "no"}
- Guard surfaces checked: ${coverage.guardSurfaces}
- App-owned remote-service surfaces documented: ${coverage.appOwnedRemoteServiceSurfaces}
- Provider-traffic surfaces documented: ${coverage.providerTrafficSurfaces}
- Managed-download surfaces documented: ${coverage.managedDownloadSurfaces}
- Forbidden remote fragments checked: ${coverage.forbiddenRemoteFragments
  .map((fragment) => `\`${fragment}\``)
  .join(", ")}
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
This smoke proves the local-only network registry, frontend/backend guards, and local-free bundle privacy scan completed locally. It does not prove signed/notarized public release readiness.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, summary);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log("Local-only network validation summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!passed) {
  process.exit(1);
}
