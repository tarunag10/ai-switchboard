#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { buildManifest, readJson, renderMarkdown } from "./benchmark-manifest.mjs";

const root = process.cwd();
const args = process.argv.slice(2);
const optionValue = (name, fallback) => {
  const index = args.indexOf(name);
  return index === -1 ? fallback : args[index + 1];
};
const outputDirectory = path.resolve(
  root,
  optionValue("--output-dir", "benchmarks/results"),
);
const fixtures = readJson(path.join(root, "benchmarks/fixtures.json"));
const schema = readJson(path.join(root, "benchmarks/schema.json"));
const baseline = readJson(
  path.resolve(root, optionValue("--baseline", "benchmarks/baseline.json")),
);
const manifest = buildManifest({ root, fixtures, schema, baseline });

if (!args.includes("--no-write")) {
  fs.mkdirSync(outputDirectory, { recursive: true });
  fs.writeFileSync(
    path.join(outputDirectory, "manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  fs.writeFileSync(path.join(outputDirectory, "summary.md"), renderMarkdown(manifest));
}

console.log(JSON.stringify(manifest, null, 2));

if (args.includes("--check") && manifest.baselineComparison.status !== "pass") {
  process.exitCode = 1;
}
