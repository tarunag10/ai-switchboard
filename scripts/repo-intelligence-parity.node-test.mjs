import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";

test("CLI graph suppresses ambiguous and receiver-qualified fallback calls", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-graph-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/one.ts"), "export function run() {}\n");
    fs.writeFileSync(path.join(repo, "src/two.ts"), "export function run() {}\n");
    fs.writeFileSync(path.join(repo, "src/caller.ts"), "run(); client.run();\n");
    fs.writeFileSync(path.join(repo, "src/unique.ts"), "export function unique() {}\n");
    fs.writeFileSync(path.join(repo, "src/unique-caller.ts"), "unique();\n");

    const output = execFileSync(
      process.execPath,
      ["scripts/repo-intelligence.mjs", repo, "--format", "json"],
      { encoding: "utf8" },
    );
    const summary = JSON.parse(output);
    const callEdges = summary.graph.symbolEdges.filter((edge) => edge.kind === "call_reference");
    assert.ok(!callEdges.some((edge) => edge.to.endsWith("#run")));
    assert.ok(callEdges.some((edge) => edge.to.endsWith("#unique")));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});
