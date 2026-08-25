#!/usr/bin/env node
// Deterministic Class C task benchmark runner.
// Validates benchmarks/tasks/*.json against benchmarks/tasks/schema.json,
// ingests optional operator/CI-produced run results from
// benchmarks/tasks/results/*.json, and writes:
//   benchmarks/results/class-c-summary.json
//   benchmarks/results/class-c-summary.md
// No network, no clock reads: identical inputs produce byte-identical outputs.

import fs from "node:fs";
import path from "node:path";
import { buildSummary, renderMarkdown } from "./lib/class-c-tasks.mjs";

const root = process.cwd();
const args = process.argv.slice(2);
const optionValue = (name) => {
  const index = args.indexOf(name);
  return index === -1 ? undefined : args[index + 1];
};

try {
  const summary = buildSummary(root, {
    minimumSamples: Number(optionValue("--minimum-samples")) || undefined,
  });
  if (!args.includes("--no-write")) {
    const outputDir = path.join(root, "benchmarks/results");
    fs.mkdirSync(outputDir, { recursive: true });
    fs.writeFileSync(path.join(outputDir, "class-c-summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
    fs.writeFileSync(path.join(outputDir, "class-c-summary.md"), renderMarkdown(summary));
  }
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
} catch (error) {
  console.error(`class-c runner failed: ${error.message}`);
  process.exit(1);
}
