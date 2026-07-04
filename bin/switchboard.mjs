#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import packageJson from "../package.json" with { type: "json" };

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const command = args[0];

function printHelp() {
  console.log(`Switchboard CLI ${packageJson.version}

Usage:
  switchboard repo-intelligence <repo-path> [options]
  switchboard repo <repo-path> [options]
  switchboard --version

Compatibility:
  npm run repo:intelligence -- <repo-path> [options]

Notes:
  The macOS app is AI Switchboard for Mac.
  Legacy Mac AI Switchboard paths and package names remain compatible.`);
}

function runNodeScript(scriptPath, scriptArgs) {
  const result = spawnSync(process.execPath, [resolve(repoRoot, scriptPath), ...scriptArgs], {
    cwd: repoRoot,
    stdio: "inherit",
  });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }

  process.exit(result.status ?? 1);
}

if (!command || command === "--help" || command === "-h" || command === "help") {
  printHelp();
  process.exit(0);
}

if (command === "--version" || command === "-v" || command === "version") {
  console.log(packageJson.version);
  process.exit(0);
}

if (["repo-intelligence", "repo", "intelligence"].includes(command)) {
  runNodeScript("scripts/repo-intelligence.mjs", args.slice(1));
}

if (command === "optimize") {
  console.error("switchboard optimize is not available in the repo CLI yet. Use the macOS app modes: Full optimization, Headroom only, RTK only, or Off.");
  process.exit(2);
}

console.error(`Unknown Switchboard CLI command: ${command}`);
printHelp();
process.exit(2);
