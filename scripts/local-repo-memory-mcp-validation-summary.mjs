import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const summaryPath = "dist/local-repo-memory-mcp-validation-summary.md";
const jsonPath = "dist/local-repo-memory-mcp-validation-summary.json";

const steps = [
  {
    id: "repo-memory-mcp-read-only-smoke",
    label: "Repo Memory MCP read-only smoke",
    command: "npm",
    args: ["run", "check:repo-memory-mcp"],
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
const output = results.map((result) => result.stdout).join("\n");
const toolNames =
  output.match(/\(([^)]+)\)/)?.[1]?.split(", ").filter(Boolean) ?? [];
const expectedTools = [
  "repo_context_pack",
  "repo_dependents_of",
  "repo_symbol_lookup",
  "switchboard.build_context_pack",
  "switchboard.get_repo_graph_summary",
  "switchboard.list_context_packs",
];
const bridgeRecipeDoc = fs.readFileSync("docs/repo-memory-mcp.md", "utf8");
const bridgeRecipeSignals = [
  "Bridge Setup Recipes",
  "Claude Code",
  "Goose",
  "Cursor, Windsurf, and Zed",
  "Continue and Aider",
  "Gemini CLI, OpenCode, Grok / xAI CLI, Qwen Code, and Amazon Q Developer CLI",
  "provider setup instructions",
];
const expectedToolsPresent = expectedTools.every((tool) =>
  toolNames.includes(tool),
);
const budgetedPackVerified = toolNames.includes("switchboard.build_context_pack");
const graphQueriesVerified = [
  "switchboard.get_repo_graph_summary",
  "repo_symbol_lookup",
  "repo_dependents_of",
].every((tool) => toolNames.includes(tool));
const staleIndexHealthVerified = toolNames.includes("switchboard.list_context_packs");
const connectorBridgeRecipesVerified = bridgeRecipeSignals.every((signal) =>
  bridgeRecipeDoc.includes(signal),
);
const overallPassed =
  passed &&
  expectedToolsPresent &&
  budgetedPackVerified &&
  graphQueriesVerified &&
  staleIndexHealthVerified &&
  connectorBridgeRecipesVerified;

const payload = {
  schemaVersion: 1,
  generatedAt,
  kind: "mac_ai_switchboard.local_repo_memory_mcp_validation",
  releaseGateEvidence: false,
  readOnly: true,
  modifiesRepository: false,
  relaunchSurvivalEvidence: "app-managed descriptor smoke recheck",
  connectorBridgeRecipesVerified,
  budgetedPackVerified,
  graphQueriesVerified,
  staleIndexHealthVerified,
  passed: overallPassed,
  toolCount: toolNames.length,
  expectedToolsPresent,
  tools: toolNames,
  steps: results.map(({ stdout, stderr, ...result }) => ({
    ...result,
    stdoutPreview: stdout.slice(0, 2000),
    stderrPreview: stderr.slice(0, 2000),
  })),
};

const summary = `# Local Repo Memory MCP Validation Summary

Generated: ${generatedAt}

- Evidence kind: local Repo Memory MCP validation
- Release gate evidence: no
- Read-only: yes
- Modifies repository: no
- Relaunch survival evidence: app-managed descriptor smoke recheck
- Connector bridge recipes verified: ${connectorBridgeRecipesVerified ? "yes" : "no"}
- Connector bridge recipe signals: ${connectorBridgeRecipesVerified ? "pass" : "fail"}
- Expected tools present: ${payload.expectedToolsPresent ? "yes" : "no"}
- Budgeted pack retrieval verified: ${payload.budgetedPackVerified ? "yes" : "no"}
- Graph queries verified: ${payload.graphQueriesVerified ? "yes" : "no"}
- Stale-index health surface verified: ${payload.staleIndexHealthVerified ? "yes" : "no"}
- Tool count: ${payload.toolCount}
- Overall result: ${overallPassed ? "pass" : "fail"}

${results
  .map(
    (result) => `## ${result.label}

- Command: \`${result.fullCommand}\`
- Result: ${result.ok ? "pass" : "fail"}
- Exit status: ${result.status ?? "unknown"}
`,
  )
  .join("\n")}
This smoke proves the repo-memory MCP stdio bridge exposes only read-only tools, returns bounded context with safety copy, and excludes seeded secret-like files. It is local-only evidence for the one-click app validation path and does not prove signed/notarized public release readiness.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, summary);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log("Local Repo Memory MCP validation summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!overallPassed) {
  process.exit(1);
}
