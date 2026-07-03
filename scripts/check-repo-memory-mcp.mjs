#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";

const scriptDir = process.cwd();
const repoIntelligenceScript = path.join(scriptDir, "scripts", "repo-intelligence.mjs");
const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-mcp-repo-"));

fs.mkdirSync(path.join(repoRoot, "src"), { recursive: true });
fs.writeFileSync(
  path.join(repoRoot, "package.json"),
  `${JSON.stringify(
    {
      scripts: { test: "vitest" },
      dependencies: { react: "latest" },
      devDependencies: { typescript: "latest" },
    },
    null,
    2,
  )}\n`,
);
fs.writeFileSync(
  path.join(repoRoot, "src", "index.ts"),
  [
    "export function helloRepo(name: string) {",
    '  return `hello ${name}`;',
    "}",
    "",
    'export const greeting = helloRepo("switchboard");',
    "",
  ].join("\n"),
);

const child = spawn(
  process.execPath,
  [repoIntelligenceScript, repoRoot, "--mcp-serve"],
  {
    cwd: scriptDir,
    stdio: ["pipe", "pipe", "pipe"],
  },
);

let stdout = "";
let stderr = "";
child.stdout.setEncoding("utf8");
child.stderr.setEncoding("utf8");
child.stdout.on("data", (chunk) => {
  stdout += chunk;
});
child.stderr.on("data", (chunk) => {
  stderr += chunk;
});

function send(request) {
  child.stdin.write(`${JSON.stringify(request)}\n`);
}

send({ jsonrpc: "2.0", id: 1, method: "initialize", params: {} });
send({ jsonrpc: "2.0", id: 2, method: "tools/list" });
send({
  jsonrpc: "2.0",
  id: 3,
  method: "tools/call",
  params: { name: "repo_context_pack", arguments: { packId: "implementation" } },
});
send({
  jsonrpc: "2.0",
  id: 4,
  method: "tools/call",
  params: {
    name: "switchboard.build_context_pack",
    arguments: { budget_tokens: 120, task: "small implementation fix" },
  },
});
send({
  jsonrpc: "2.0",
  id: 5,
  method: "tools/call",
  params: { name: "switchboard.get_repo_graph_summary", arguments: {} },
});
send({
  jsonrpc: "2.0",
  id: 6,
  method: "tools/call",
  params: { name: "repo_symbol_lookup", arguments: { query: "helloRepo" } },
});
send({
  jsonrpc: "2.0",
  id: 7,
  method: "tools/call",
  params: { name: "repo_dependents_of", arguments: { target: "src/index.ts" } },
});

const timeout = setTimeout(() => {
  child.kill();
  throw new Error(`repo-memory MCP smoke timed out. stderr=${stderr}`);
}, 5_000);

function responseById(responses, id) {
  const response = responses.find((item) => item.id === id);
  if (!response) {
    throw new Error(`repo-memory MCP response missing id=${id}. stdout=${stdout}`);
  }
  if (response.error) {
    throw new Error(
      `repo-memory MCP response id=${id} errored: ${JSON.stringify(response.error)}`,
    );
  }
  return response;
}

function responseText(responses, id) {
  const response = responseById(responses, id);
  const text = response.result?.content?.[0]?.text;
  if (typeof text !== "string" || text.length === 0) {
    throw new Error(`repo-memory MCP response id=${id} did not return text content`);
  }
  return text;
}

function expectText(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label} missing ${expected}. text=${text.slice(0, 500)}`);
  }
}

setTimeout(() => {
  clearTimeout(timeout);
  child.kill();

  const responses = stdout
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  const toolsResponse = responseById(responses, 2);
  const tools = toolsResponse.result?.tools ?? [];
  const names = tools.map((tool) => tool.name);
  const expectedTools = [
    "repo_context_pack",
    "repo_dependents_of",
    "repo_symbol_lookup",
    "switchboard.build_context_pack",
    "switchboard.get_repo_graph_summary",
    "switchboard.list_context_packs",
  ];

  for (const expected of expectedTools) {
    if (!names.includes(expected)) {
      throw new Error(`repo-memory MCP tool missing: ${expected}`);
    }
  }
  for (const tool of tools) {
    if (tool.annotations?.readOnlyHint !== true) {
      throw new Error(`repo-memory MCP tool is not read-only: ${tool.name}`);
    }
    if (!String(tool.description ?? "").includes("read-only")) {
      throw new Error(`repo-memory MCP tool lacks read-only description: ${tool.name}`);
    }
  }

  const legacyPack = responseText(responses, 3);
  const budgetedPack = responseText(responses, 4);
  const graphSummary = responseText(responses, 5);
  const symbolLookup = responseText(responses, 6);
  const dependents = responseText(responses, 7);

  expectText(legacyPack, "Implementation", "legacy context pack");
  expectText(budgetedPack, "Implementation", "budgeted context pack");
  expectText(budgetedPack, repoRoot, "budgeted context pack");
  expectText(graphSummary, "src", "graph summary");
  expectText(symbolLookup, "src/index.ts", "symbol lookup");
  expectText(dependents, "src/index.ts", "dependents query");

  console.log(
    `Repo-memory MCP tools are read-only and queryable (${names.join(", ")}).`,
  );
}, 250);
