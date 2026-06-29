import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoIntelligenceScript = path.join(scriptDir, "repo-intelligence.mjs");

const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-mcp-repo-"));
fs.mkdirSync(path.join(repoRoot, "src"), { recursive: true });
fs.writeFileSync(
  path.join(repoRoot, "src", "index.ts"),
  "export function main() { return 'ok'; }\n",
);
fs.writeFileSync(path.join(repoRoot, ".env.local"), "SECRET=value\n");

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

send({ jsonrpc: "2.0", id: 1, method: "initialize" });
send({ jsonrpc: "2.0", id: 2, method: "tools/list" });
send({
  jsonrpc: "2.0",
  id: 3,
  method: "tools/call",
  params: {
    name: "repo_context_pack",
    arguments: { packId: "implementation" },
  },
});

const timeout = setTimeout(() => {
  child.kill();
  throw new Error(`repo-memory MCP smoke timed out. stderr=${stderr}`);
}, 5_000);

function finish() {
  const lines = stdout
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  const listResponse = lines.find((line) => line.id === 2);
  const packResponse = lines.find((line) => line.id === 3);
  const tools = listResponse?.result?.tools ?? [];
  const names = tools.map((tool) => tool.name).sort();

  for (const expected of [
    "repo_context_pack",
    "repo_dependents_of",
    "repo_symbol_lookup",
  ]) {
    if (!names.includes(expected)) {
      throw new Error(`repo-memory MCP tool missing: ${expected}`);
    }
  }
  for (const tool of tools) {
    if (tool.annotations?.readOnlyHint !== true) {
      throw new Error(`repo-memory MCP tool is not read-only: ${tool.name}`);
    }
    if (!/read-only/i.test(tool.description)) {
      throw new Error(`repo-memory MCP tool lacks read-only description: ${tool.name}`);
    }
  }
  const packText = packResponse?.result?.content?.[0]?.text ?? "";
  if (!packText.includes("Safety: read-only context pack")) {
    throw new Error("repo_context_pack response is missing read-only safety text");
  }
  if (packText.includes(".env.local") || packText.includes("SECRET=value")) {
    throw new Error("repo_context_pack response leaked secret-like file content");
  }

  clearTimeout(timeout);
  child.kill();
  fs.rmSync(repoRoot, { recursive: true, force: true });
  console.log(`Repo-memory MCP tools are read-only (${names.join(", ")}).`);
}

setTimeout(finish, 250);
