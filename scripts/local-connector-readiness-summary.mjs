#!/usr/bin/env node

import fs from "node:fs";
import { spawnSync } from "node:child_process";

const summaryPath = "dist/local-connector-readiness-summary.md";
const jsonPath = "dist/local-connector-readiness-summary.json";
const requiredGatedNativeWrite = ["aider", "amazon_q", "cursor"];
const requiredLifecycleStages = ["detect", "dryRunDiff", "backup", "apply", "verify", "rollback", "offCleanup"];
const generatedAt = new Date().toISOString();

const result = spawnSync("npm", ["run", "check:connectors"], {
  cwd: process.cwd(),
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

function sourceHasConnectorDossier(connectorId) {
  return connectorSource.includes(`connectorId: "${connectorId}"`);
}

function sharedPlanIncludesStage(stage) {
  return connectorSource.includes(`id: "${stage}"`);
}

function promotedNativeAutomationIds() {
  const match = connectorSource.match(/promotedManagedConfigConnectorIds\s*=\s*new Set\(\[([\s\S]*?)\]\)/);
  if (!match) return [];
  return [...match[1].matchAll(/"([^"]+)"/g)].map((item) => item[1]);
}

const promotedIds = promotedNativeAutomationIds();
const sharedStages = requiredLifecycleStages.filter(sharedPlanIncludesStage);
const lifecycleCoverage = Object.fromEntries(
  requiredGatedNativeWrite.map((connectorId) => {
    const hasDossier = sourceHasConnectorDossier(connectorId);
    const automationDisabled = !promotedIds.includes(connectorId);
    return [
      connectorId,
      {
        stages: hasDossier ? sharedStages : [],
        complete: hasDossier && sharedStages.length === requiredLifecycleStages.length,
        automationDisabled,
      },
    ];
  }),
);

const lifecycleCoverageComplete = Object.values(lifecycleCoverage).every(
  (item) => item.complete && item.automationDisabled,
);
const ready = result.status === 0 && requiredGatedNativeWritePresent && lifecycleCoverageComplete;

const payload = {
  schemaVersion: 1,
  generatedAt,
  kind: "mac_ai_switchboard.local_connector_readiness_validation",
  releaseGateEvidence: false,
  readOnly: false,
  ready,
  requiredGatedNativeWrite,
  gatedNativeWriteConnectors,
  requiredGatedNativeWritePresent,
  requiredLifecycleStages,
  sharedStages,
  promotedNativeAutomationIds: promotedIds,
  lifecycleCoverage,
  lifecycleCoverageComplete,
  status: result.status,
  stdoutPreview: stdout.slice(0, 4000),
  stderrPreview: stderr.slice(0, 4000),
};

fs.mkdirSync("dist", { recursive: true });
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

const lifecycleLines = Object.entries(lifecycleCoverage).map(([connectorId, coverage]) => {
  return `- ${connectorId}: ${coverage.complete ? "complete" : "incomplete"}; stages: ${
    coverage.stages.join(", ") || "none"
  }; automation disabled: ${coverage.automationDisabled ? "yes" : "no"}`;
});

const markdown = `# Local Connector Readiness Validation

Generated: ${generatedAt}

- Ready: ${ready ? "yes" : "no"}
- Read-only: no
- Release gate evidence: no
- Required gated native-write dossiers present: ${requiredGatedNativeWritePresent ? "yes" : "no"}
- Required gated native-write dossiers: ${requiredGatedNativeWrite.join(", ")}
- Observed gated native-write dossiers: ${gatedNativeWriteConnectors.join(", ") || "missing"}
- Required lifecycle stages: ${requiredLifecycleStages.join(", ")}
- Shared lifecycle stages found: ${sharedStages.join(", ") || "missing"}
- Lifecycle coverage complete and automation disabled: ${lifecycleCoverageComplete ? "yes" : "no"}
- Promoted native automation ids: ${promotedIds.join(", ") || "none"}

## Native-Write Lifecycle Coverage

${lifecycleLines.join("\n")}

## Command Output Preview

\`\`\`
${stdout.slice(0, 2000).trim() || "(no stdout)"}
${stderr.trim() ? `\nSTDERR:\n${stderr.slice(0, 2000).trim()}` : ""}
\`\`\`
`;

fs.writeFileSync(summaryPath, markdown);

console.log("Local connector readiness summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!ready) {
  process.exitCode = 1;
}
