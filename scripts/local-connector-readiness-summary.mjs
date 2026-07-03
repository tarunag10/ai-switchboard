#!/usr/bin/env node
import fs from "node:fs";
import { spawnSync } from "node:child_process";

const summaryPath = "dist/local-connector-readiness-summary.md";
const jsonPath = "dist/local-connector-readiness-summary.json";
const requiredGatedNativeWrite = ["aider", "cursor"];
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
const passed =
  result.status === 0 &&
  requiredGatedNativeWritePresent &&
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
  status: result.status,
  stdoutPreview: stdout.slice(0, 4000),
  stderrPreview: stderr.slice(0, 4000),
};

fs.mkdirSync("dist", { recursive: true });
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

const markdown = `# Local Connector Readiness Validation

Generated: ${generatedAt}

- Release gate evidence: no
- Read-only: yes
- Modifies repository: no
- Required command: \`${payload.requiredCommand}\`
- Required gated native-write dossiers present: ${requiredGatedNativeWritePresent ? "yes" : "no"}
- Required gated native-write dossiers: ${requiredGatedNativeWrite.join(", ")}
- Observed gated native-write dossiers: ${gatedNativeWriteConnectors.join(", ") || "missing"}
- Overall result: ${passed ? "pass" : "fail"}

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
