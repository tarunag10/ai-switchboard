#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

import { evaluateGodFileRegistry, loadGodFileRegistry } from "./lib/god-file-registry.mjs";

const root = process.cwd();
const registry = loadGodFileRegistry(root);

const requiredSplitModules = registry.splitModules ?? [];

function fail(message) {
  console.error(`god file splits check failed: ${message}`);
  process.exit(1);
}

for (const entry of requiredSplitModules) {
  const absolute = path.join(root, entry.path);
  if (!fs.existsSync(absolute)) {
    fail(`missing split module ${entry.path} (${entry.parent})`);
  }
  for (const field of ["id", "path", "parent", "domain"]) {
    if (!(field in entry)) {
      fail(`split module entry missing ${field}`);
    }
  }
}

const report = evaluateGodFileRegistry(root);
for (const entry of report.entries) {
  if (entry.tier !== "god") {
    continue;
  }
  const original = registry.originalBaselines?.[entry.id];
  const hasSplitModules = requiredSplitModules.some(
    (module) => module.parent === entry.id,
  );
  if (!original || !hasSplitModules) {
    continue;
  }
  if (entry.measuredLines >= original.lines) {
    fail(
      `${entry.path} must shrink below original baseline ${original.lines} lines (measured ${entry.measuredLines})`,
    );
  }
}

console.log(
  JSON.stringify(
    {
      ok: true,
      splitModuleCount: requiredSplitModules.length,
      godFiles: report.entries.filter((entry) => entry.tier === "god"),
    },
    null,
    2,
  ),
);
