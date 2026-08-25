#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, isAbsolute, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import packageJson from "../package.json" with { type: "json" };

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const nativeRequested = args.includes("--native");
const commandArgs = args.filter((arg) => arg !== "--native");
const command = commandArgs[0];
const NATIVE_SHAPE_ERROR =
  "--native is supported only as the single final argument for harness status, router endpoint plan, and Workbench session serialize.";
const WORKBENCH_NATIVE_REQUIRED_ERROR =
  "workbench session serialize requires --native as the final argument.";
const HARNESS_STATUS_TRAILING_ERROR =
  "harness status accepts no trailing arguments.";

function hasExactArgs(expected) {
  return (
    args.length === expected.length &&
    args.every((arg, index) => arg === expected[index])
  );
}

const plainHarnessStatus = hasExactArgs(["harness", "status"]);
const nativeHarnessStatus = hasExactArgs([
  "harness",
  "status",
  "--native",
]);
const routerEndpointPlan =
  commandArgs.length === 3 &&
  commandArgs[0] === "router" &&
  commandArgs[1] === "endpoint" &&
  commandArgs[2] === "plan";
const nativeRouterEndpointPlan = hasExactArgs([
  "router",
  "endpoint",
  "plan",
  "--native",
]);
const workbenchSessionSerialize =
  commandArgs.length === 3 &&
  commandArgs[0] === "workbench" &&
  commandArgs[1] === "session" &&
  commandArgs[2] === "serialize";
const nativeWorkbenchSessionSerialize = hasExactArgs([
  "workbench",
  "session",
  "serialize",
  "--native",
]);

function runNativeCli(nativeArgs) {
  const nativeCli = process.env.SWITCHBOARD_NATIVE_CLI?.trim();
  if (!nativeCli) {
    console.error(
      "Native CLI bridge is disabled. Set SWITCHBOARD_NATIVE_CLI to an executable native CLI, then retry with --native.",
    );
    process.exit(1);
  }
  if (!isAbsolute(nativeCli)) {
    console.error(
      "Native CLI bridge requires SWITCHBOARD_NATIVE_CLI to be an absolute executable path.",
    );
    process.exit(1);
  }

  const result = spawnSync(nativeCli, nativeArgs, {
    cwd: repoRoot,
    stdio: "inherit",
    shell: false,
  });

  if (result.error) {
    console.error("Native CLI bridge could not start the configured native CLI.");
    process.exit(1);
  }
  if (typeof result.status === "number") {
    process.exit(result.status);
  }
  console.error("Native CLI bridge terminated without an exit status.");
  process.exit(1);
}

function printHelp() {
  console.log(`Switchboard CLI ${packageJson.version}

Usage:
  switchboard repo-intelligence <repo-path> [options]
  switchboard repo <repo-path> [options]
  switchboard harness status [--native]
  switchboard harness session <repo-path> [options]
  switchboard workbench session serialize --native
  switchboard router <repo-path> [options]
  switchboard router endpoint plan --native
  switchboard optimize <repo-path> [options]
  switchboard --version

Compatibility:
  npm run repo:intelligence -- <repo-path> [options]

Notes:
  Node harness, router, and Repo Intelligence commands are local, provider-neutral planning surfaces.
  The Node package does not bundle a native executable.
  Native status, endpoint planning, and Workbench serialization require --native and an external
  native CLI named by an absolute SWITCHBOARD_NATIVE_CLI path.
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

if (routerEndpointPlan && !nativeRouterEndpointPlan) {
  console.error(
    "router endpoint plan requires --native as the final argument.",
  );
  process.exit(2);
}

if (workbenchSessionSerialize && !nativeWorkbenchSessionSerialize) {
  console.error(WORKBENCH_NATIVE_REQUIRED_ERROR);
  process.exit(2);
}

if (nativeRequested && command === "router" && !nativeRouterEndpointPlan) {
  console.error(
    "--native is supported only for router endpoint plan, harness status, and Workbench session serialize.",
  );
  process.exit(2);
}

if (nativeRequested && command === "optimize") {
  console.error(
    "--native is not supported for optimize.",
  );
  process.exit(2);
}

if (
  nativeRequested &&
  !nativeHarnessStatus &&
  !nativeRouterEndpointPlan &&
  !nativeWorkbenchSessionSerialize
) {
  console.error(NATIVE_SHAPE_ERROR);
  process.exit(2);
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
    if (nativeHarnessStatus) {
      runNativeCli(commandArgs);
    }
    if (!plainHarnessStatus) {
      console.error(HARNESS_STATUS_TRAILING_ERROR);
      process.exit(2);
    }
    console.log(JSON.stringify({
      version: packageJson.version,
      surface: "switchboard-harness",
      platform: process.platform,
      mode: "local-preview",
      cli: { available: true, binary: "switchboard" },
      router: { available: true, execution: "observe-only", providerTraffic: false },
      workbench: {
        contractAvailable: true,
        bundledNativeExecutable: false,
        execution: "external-native-cli-required",
        externalNativeCli: {
          required: true,
          environmentVariable: "SWITCHBOARD_NATIVE_CLI",
          pathRequirement: "absolute",
        },
      },
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

if (nativeWorkbenchSessionSerialize) {
  runNativeCli(commandArgs);
}

if (nativeRouterEndpointPlan) {
  runNativeCli(commandArgs);
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
