#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import packageJson from "../package.json" with { type: "json" };

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const nativeRequested = args.includes("--native");
const commandArgs = args.filter((arg) => arg !== "--native");
const command = commandArgs[0];

function runNativeCli(nativeArgs) {
  const nativeCli = process.env.SWITCHBOARD_NATIVE_CLI?.trim();
  if (!nativeCli) {
    console.error(
      "Native CLI bridge is disabled. Set SWITCHBOARD_NATIVE_CLI to an executable native CLI, then retry with --native.",
    );
    process.exit(1);
  }

  const result = spawnSync(nativeCli, nativeArgs, {
    cwd: repoRoot,
    stdio: "inherit",
    shell: false,
  });

  if (result.error) {
    console.error(
      `Native CLI bridge could not execute SWITCHBOARD_NATIVE_CLI. Set it to an executable file and retry: ${result.error.message}`,
    );
    process.exit(1);
  }
  if (typeof result.status === "number") {
    process.exit(result.status);
  }
  console.error(
    "Native CLI bridge terminated without an exit status. Retry with an executable SWITCHBOARD_NATIVE_CLI.",
  );
  process.exit(1);
}

function printHelp() {
  console.log(`Switchboard CLI ${packageJson.version}

Usage:
  switchboard repo-intelligence <repo-path> [options]
  switchboard repo <repo-path> [options]
  switchboard harness status [--native]
  switchboard harness session <repo-path> [options]
  switchboard workbench session serialize [--native]
  switchboard router <repo-path> [options]
  switchboard optimize <repo-path> [options]
  switchboard --version

Compatibility:
  npm run repo:intelligence -- <repo-path> [options]

Notes:
  Harness and router commands are local, provider-neutral planning surfaces.
  They prepare Repo Intelligence/session evidence and do not send provider traffic.
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
  runNodeScript("scripts/repo-intelligence.mjs", commandArgs.slice(1));
}

if (command === "harness") {
  const subcommand = commandArgs[1];
  if (!subcommand || subcommand === "--help" || subcommand === "help") {
    printHelp();
    process.exit(0);
  }
  if (subcommand === "status") {
    if (nativeRequested) {
      runNativeCli(commandArgs);
    }
    console.log(JSON.stringify({
      version: packageJson.version,
      surface: "switchboard-harness",
      platform: process.platform,
      mode: "local-preview",
      cli: { available: true, binary: "switchboard" },
      router: { available: true, execution: "observe-only", providerTraffic: false },
      workbench: { available: true, execution: "plan-and-session-evidence" },
      repoIntelligence: { available: true, entrypoint: "scripts/repo-intelligence.mjs" },
      desktopRuntime: { requiredForLiveProcessStart: true },
    }, null, 2));
    process.exit(0);
  }
  if (subcommand === "session") {
    runNodeScript("scripts/repo-intelligence.mjs", [
      commandArgs[2] ?? ".",
      "--start-session",
      ...commandArgs.slice(3),
    ]);
  }
  console.error(`Unknown harness command: ${subcommand}`);
  printHelp();
  process.exit(2);
}

if (
  command === "workbench" &&
  commandArgs[1] === "session" &&
  commandArgs[2] === "serialize" &&
  nativeRequested
) {
  runNativeCli(commandArgs);
}

if (nativeRequested && ["router", "optimize"].includes(command)) {
  console.error("--native is supported only for harness status and Workbench session serialize.");
  process.exit(2);
}

if (command === "router") {
  runNodeScript("scripts/repo-intelligence.mjs", [
    commandArgs[1] ?? ".",
    "--start-session",
    ...commandArgs.slice(2),
  ]);
}

if (command === "optimize") {
  runNodeScript("scripts/repo-intelligence.mjs", [
    commandArgs[1] ?? ".",
    "--start-session",
    ...commandArgs.slice(2),
  ]);
}

console.error(`Unknown Switchboard CLI command: ${command}`);
printHelp();
process.exit(2);
