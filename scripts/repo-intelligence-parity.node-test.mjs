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

test("CLI graph resolves one-hop named and wildcard re-exports", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-reexport-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/worker.ts"), "export function runTask() {}\n");
    fs.writeFileSync(path.join(repo, "src/named.ts"), "export { runTask as execute } from './worker';\n");
    fs.writeFileSync(path.join(repo, "src/star.ts"), "export * from './worker';\n");
    fs.writeFileSync(path.join(repo, "src/consumer.ts"), "import { execute } from './named'; import { runTask } from './star'; export function start() { execute(); runTask(); }\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.ok(summary.graph.symbolEdges.some((edge) => edge.to.endsWith("#runTask") && edge.kind === "call_reference"));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI graph leaves dynamic, unresolved, and two-hop re-exports unresolved", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-reexport-negative-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/worker.ts"), "export function runTask() {}\n");
    fs.writeFileSync(path.join(repo, "src/duplicate.ts"), "export function runTask() {}\n");
    fs.writeFileSync(path.join(repo, "src/dynamic.ts"), "const target = './worker'; export { target };\n");
    fs.writeFileSync(path.join(repo, "src/inner.ts"), "export { runTask } from './worker';\n");
    fs.writeFileSync(path.join(repo, "src/outer.ts"), "export { runTask } from './inner';\n");
    fs.writeFileSync(path.join(repo, "src/consumer.ts"), "import { runTask as dynamic } from './dynamic'; import { runTask as chained } from './outer'; export function start() { dynamic(); chained(); }\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.ok(!summary.graph.symbolEdges.some((edge) => edge.to === "src/worker.ts#runTask" && edge.kind === "call_reference"));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI graph rejects ambiguous wildcards and private named re-exports", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-reexport-visibility-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/a.ts"), "export function runTask() {}\nfunction hidden() {}\n");
    fs.writeFileSync(path.join(repo, "src/b.ts"), "export function runTask() {}\n");
    fs.writeFileSync(path.join(repo, "src/duplicate-hidden.ts"), "function hidden() {}\n");
    fs.writeFileSync(path.join(repo, "src/barrel.ts"), "export * from './a'; export * from './b'; export { hidden } from './a';\n");
    fs.writeFileSync(path.join(repo, "src/consumer.ts"), "import { runTask, hidden } from './barrel'; export function start() { runTask(); hidden(); }\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.ok(!summary.graph.symbolEdges.some((edge) => edge.kind === "call_reference" && (edge.to === "src/a.ts#runTask" || edge.to === "src/b.ts#runTask" || edge.to === "src/a.ts#hidden")));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI graph resolves exported namespace members and rejects private members", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-namespace-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/utils.ts"), "export function normalize() {}\nfunction hidden() {}\n");
    fs.writeFileSync(path.join(repo, "src/consumer.ts"), "import * as utils from './utils'; export function start() { utils.normalize(); utils.hidden(); }\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    const callEdges = summary.graph.symbolEdges.filter((edge) => edge.kind === "call_reference");
    assert.ok(callEdges.some((edge) => edge.to === "src/utils.ts#normalize"));
    assert.ok(!callEdges.some((edge) => edge.to === "src/utils.ts#hidden"));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI default indexing excludes unknown files like the native and frontend indexers", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-classification-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/app.ts"), "export function app() {}\n");
    fs.writeFileSync(path.join(repo, "notes.weird"), "unknown file\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.equal(summary.indexedFiles, 1);
    assert.equal(summary.skippedFiles, 1);
    assert.equal(summary.indexerVersion, "path-graph-v12");
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI skips oversized source files and ignored dependency directories deterministically", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-size-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.mkdirSync(path.join(repo, "vendor", "nested"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/small.ts"), "export const ok = true;\n");
    fs.writeFileSync(path.join(repo, "src/large.ts"), "x".repeat(1_000_001));
    fs.writeFileSync(path.join(repo, "vendor", "nested", "ignored.ts"), "export const no = true;\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.equal(summary.indexedFiles, 1);
    assert.equal(summary.skippedFiles, 1);
    assert.equal(summary.roleCounts.generated, 1);
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI default indexing excludes singular secret directories", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-secret-classification-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.mkdirSync(path.join(repo, "secret"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/app.ts"), "export function app() {}\n");
    fs.writeFileSync(path.join(repo, "secret/api.ts"), "export const token = 'redacted';\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.equal(summary.indexedFiles, 1);
    assert.equal(summary.skippedFiles, 1);
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});
