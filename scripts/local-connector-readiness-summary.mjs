#!/usr/bin/env node
import fs from "node:fs";
import { spawnSync } from "node:child_process";

const summaryPath = "dist/local-connector-readiness-summary.md";
const jsonPath = "dist/local-connector-readiness-summary.json";
const requiredGatedNativeWrite = ["aider", "cursor"];
const requiredLifecycleStages = [
  "detect",
  "dryRunDiff",
  "backup",
  "apply",
  "verify",
  "rollback",
  "offCleanup",
];
const generatedAt = new Date().toISOString();

const result = spawnSync("npm", ["run", "check:connectors"], {
  encoding: "utf8",
  timeout: 120_000,
});

const stdout = result.stdout ?? "";
const stderr = result.stderr ?? "";
const output = `${stdout}\n${stderr}`;
const gatedMatch = output.match(/gated native-write dossiers,\s*(\d+)[^:]*:\s*([^\n]+)/i);
const gatedNativeWriteConnectors = gatedMatch
  ? gatedMatch[2]
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean)
  : [];
const requiredGatedNativeWritePresent = requiredGatedNativeWrite.every((id) =>
  gatedNativeWriteConnectors.includes(id),
);
const connectorSource = fs.readFileSync("src/lib/plannedConnectors.ts", "utf8");

function connectorBlock(connectorId) {
  const blockStart = connectorSource.indexOf(`id: "${connectorId}"`);
  if (blockStart < 0) {
    return "";
  }
  const tail = connectorSource.slice(blockStart + 1);
  const nextConnector = tail.search(/\n\s*\{\s*\n\s*id: "/);
  const blockEnd =
    nextConnector >= 0 ? blockStart + 1 + nextConnector : undefined;
  return connectorSource.slice(blockStart, blockEnd);
}

const lifecycleCoverage = Object.fromEntries(
  requiredGatedNativeWrite.map((connectorId) => {
    const block = connectorBlock(connectorId);
    const stages = requiredLifecycleStages.filter((stage) =>
      block.includes(stage),
    );
    return [
      connectorId,
      {
        stages,
        complete: stages.length === requiredLifecycleStages.length,
        automationDisabled: block.includes("automationEnabled: false"),
      },
    ];
  }),
);
const lifecycleCoverageComplete = Object.values(lifecycleCoverage).every(
  (item) => item.complete && item.automationDisabled,
);
const passed =
  result.status === 0 &&
  requiredGatedNativeWritePresent &&
  lifecycleCoverageComplete &&
  output.includes("Connector manifests validated");

const payload = {
  generatedAt,
  kind: "mac_ai_switchboard.local_connector_readiness_validation",
  releaseGateEvidence: false,
  readOnly: true,
  modifiesRepository: false,
  passed,
  requiredCommand: "npm run check:connectors",
  requiredGatedNativeWrite,
  gatedNativeWriteConnectors,
  requiredGatedNativeWritePresent,
  requiredLifecycleStages,
  lifecycleCoverage,
  lifecycleCoverageComplete,
  status: result.status,
  stdoutPreview: stdout.slice(0, 4000),
  stderrPreview: stderr.slice(0, 4000),
};

fs.mkdirSync("dist", { recursive: true });
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

const lifecycleLines = Object.entries(lifecycleCoverage)
  .map(
    ([connectorId, coverage]) =>
      `- ${connectorId}: ${coverage.complete ? "complete" : "missing"} (${coverage.stages.join(", ") || "none"}); automation disabled: ${coverage.automationDisabled ? "yes" : "no"}`,
  )
  .join("\n");

const markdown = `# Local Connector Readiness Validation

Generated: ${generatedAt}

- Release gate evidence: no
- Read-only: yes
- Modifies repository: no
- Required command: \`${payload.requiredCommand}\`
- Required gated native-write dossiers present: ${requiredGatedNativeWritePresent ? "yes" : "no"}
- Required gated native-write dossiers: ${requiredGatedNativeWrite.join(", ")}
- Observed gated native-write dossiers: ${gatedNativeWriteConnectors.join(", ") || "missing"}
- Required lifecycle stages: ${requiredLifecycleStages.join(", ")}
- Lifecycle coverage complete: ${lifecycleCoverageComplete ? "yes" : "no"}
- Overall result: ${passed ? "pass" : "fail"}

## Native-Write Lifecycle Coverage

${lifecycleLines}

## Command Output

\`\`\`text
${stdout.trim() || "(no stdout)"}
${stderr.trim() ? `\n${stderr.trim()}` : ""}
\`\`\`
`;

fs.writeFileSync(summaryPath, markdown);
console.log("Local connector readiness summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!passed) {
  process.exitCode = 1;
}
