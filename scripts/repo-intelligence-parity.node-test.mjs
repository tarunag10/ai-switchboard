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

test("CLI graph resolves named default imports without global-name ambiguity", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-default-import-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/worker.ts"), "export default function runTask() {}\n");
    fs.writeFileSync(path.join(repo, "src/other.ts"), "export function runTask() {}\n");
    fs.writeFileSync(path.join(repo, "src/consumer.ts"), "import runTask from './worker'; export function start() { runTask(); }\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.ok(summary.graph.symbolEdges.some((edge) => edge.to === "src/worker.ts#runTask" && edge.kind === "call_reference"));
    assert.ok(!summary.graph.symbolEdges.some((edge) => edge.to === "src/other.ts#runTask" && edge.kind === "call_reference"));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI graph resolves same-file identifier-form default exports", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-default-identifier-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/worker.ts"), "function runTask() {}\nexport default runTask;\n");
    fs.writeFileSync(path.join(repo, "src/consumer.ts"), "import runTask from './worker'; export function start() { runTask(); }\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.ok(summary.graph.symbolEdges.some((edge) => edge.to === "src/worker.ts#runTask" && edge.kind === "call_reference"));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI graph resolves same-file aliased named exports", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-local-alias-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/worker.ts"), "function runTask() {}\nexport { runTask as execute };\n");
    fs.writeFileSync(path.join(repo, "src/consumer.ts"), "import { execute } from './worker'; export function start() { execute(); }\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.ok(summary.graph.symbolEdges.some((edge) => edge.to === "src/worker.ts#runTask" && edge.kind === "call_reference"));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI default re-exports stay one-hop and anonymous defaults stay unresolved", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-default-reexport-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/worker.ts"), "export function runTask() {}\n");
    fs.writeFileSync(path.join(repo, "src/barrel.ts"), "export { runTask as default } from './worker';\n");
    fs.writeFileSync(path.join(repo, "src/anonymous.ts"), "export default function () {}\n");
    fs.writeFileSync(path.join(repo, "src/consumer.ts"), "import runTask from './barrel'; import anonymous from './anonymous'; export function start() { runTask(); anonymous(); }\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    const callEdges = summary.graph.symbolEdges.filter((edge) => edge.kind === "call_reference");
    assert.ok(callEdges.some((edge) => edge.to === "src/worker.ts#runTask"));
    assert.ok(!callEdges.some((edge) => edge.to.includes("anonymous")));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI CommonJS require ignores later string literals on the same line", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-commonjs-"));
  try {
    fs.mkdirSync(path.join(repo, "src"), { recursive: true });
    fs.writeFileSync(path.join(repo, "src/worker.js"), "module.exports = {}\n");
    fs.writeFileSync(path.join(repo, "src/loaded.js"), "module.exports = {}\n");
    fs.writeFileSync(path.join(repo, "src/consumer.js"), "const worker = require(\"./worker\"); console.log(\"loaded\");\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    const imports = summary.graph.importEdges.filter((edge) => edge.from === "src/consumer.js");
    assert.ok(imports.some((edge) => edge.to === "src/worker.js"));
    assert.ok(!imports.some((edge) => edge.to === "src/loaded.js"));
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

test("CLI classification keeps secret precedence, shell source, and nested docs parity", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-role-matrix-"));
  try {
    fs.mkdirSync(path.join(repo, "src", "docs"), { recursive: true });
    fs.mkdirSync(path.join(repo, "Secret"), { recursive: true });
    fs.mkdirSync(path.join(repo, ".secret"), { recursive: true });
    fs.mkdirSync(path.join(repo, "private_key"), { recursive: true });
    fs.writeFileSync(path.join(repo, "scripts.sh"), "echo ok\n");
    fs.writeFileSync(path.join(repo, "src", "docs", "guide.ts"), "export const guide = true;\n");
    fs.writeFileSync(path.join(repo, "Secret", "api.ts"), "export const token = true;\n");
    fs.writeFileSync(path.join(repo, ".secret", "api.ts"), "export const token = true;\n");
    fs.writeFileSync(path.join(repo, "private_key", "api.ts"), "export const token = true;\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.equal(summary.indexedFiles, 2);
    assert.equal(summary.roleCounts.generated, 3);
    assert.equal(summary.roleCounts.source, 1);
    assert.equal(summary.roleCounts.docs, 1);
    assert.ok(summary.indexMetadata.fileFingerprints.some((entry) => entry.path === "scripts.sh"));
    assert.ok(summary.indexMetadata.fileFingerprints.some((entry) => entry.path === "src/docs/guide.ts"));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("CLI applies the file cap after global path sorting", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-cap-"));
  try {
    fs.mkdirSync(path.join(repo, "a"), { recursive: true });
    for (let index = 0; index < 2_500; index += 1) {
      fs.writeFileSync(
        path.join(repo, "a", `${String(index).padStart(4, "0")}.ts`),
        "export const value = 1;\n",
      );
    }
    fs.writeFileSync(path.join(repo, "a-file.ts"), "export const root = 1;\n");
    const summary = JSON.parse(execFileSync(process.execPath, ["scripts/repo-intelligence.mjs", repo, "--format", "json"], { encoding: "utf8" }));
    assert.equal(summary.indexedFiles, 2_500);
    const indexedPaths = summary.indexMetadata.fileFingerprints.map(
      (entry) => entry.path,
    );
    assert.ok(indexedPaths.includes("a-file.ts"));
    assert.ok(!indexedPaths.includes("a/2499.ts"));
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});
