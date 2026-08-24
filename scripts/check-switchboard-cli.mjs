#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import packageJson from "../package.json" with { type: "json" };
import { runSwitchboardContracts } from "./check-switchboard-contracts.mjs";

function fail(message) {
  console.error(`switchboard cli check failed: ${message}`);
  process.exit(1);
}

function run(args) {
  const result = spawnSync(process.execPath, ["bin/switchboard.mjs", ...args], {
    cwd: process.cwd(),
    encoding: "utf8",
    maxBuffer: 4 * 1024 * 1024,
  });

  if (result.status !== 0) {
    fail(`switchboard ${args.join(" ")} exited ${result.status}: ${result.stderr || result.stdout}`);
  }

  return result.stdout;
}

if (packageJson.bin?.switchboard !== "./bin/switchboard.mjs") {
  fail("package.json must expose bin.switchboard");
}

if (!packageJson.scripts?.switchboard?.includes("bin/switchboard.mjs")) {
  fail("package.json must expose npm run switchboard");
}

const help = run(["--help"]);
if (!help.includes("switchboard repo-intelligence") || !help.includes("switchboard harness status") || !help.includes("Legacy Mac AI Switchboard paths")) {
  fail("help output must document Repo Intelligence, harness status, and legacy compatibility");
}

const version = run(["--version"]).trim();
if (version !== packageJson.version) {
  fail(`version mismatch: ${version}`);
}

const agents = run(["repo-intelligence", ".", "--list-agents"]);
if (!agents.includes("codex") || !agents.includes("gemini")) {
  fail("repo-intelligence wrapper did not return expected agent ids");
}

const session = run([
  "repo-intelligence",
  ".",
  "--start-session",
  "--agent",
  "codex",
  "--budget",
  "1200",
]);
if (!session.includes('"kind": "mac_ai_switchboard.agent_session_preparation"')) {
  fail("--start-session alias did not return an agent session preparation");
}

const harnessStatus = JSON.parse(run(["harness", "status"]));
if (harnessStatus.router?.available !== true || harnessStatus.workbench?.available !== true || harnessStatus.router?.providerTraffic !== false) {
  fail("harness status must expose provider-neutral router and Workbench contracts");
}

const harnessSession = run(["harness", "session", ".", "--agent", "codex", "--budget", "1200"]);
if (!harnessSession.includes('"kind": "mac_ai_switchboard.agent_session_preparation"')) {
  fail("harness session did not return an agent session preparation");
}

const routerSession = run(["router", ".", "--agent", "codex", "--budget", "1200"]);
if (!routerSession.includes('"kind": "mac_ai_switchboard.agent_session_preparation"')) {
  fail("router alias did not return an agent session preparation");
}

const optimizeSession = run(["optimize", ".", "--agent", "codex", "--budget", "1200"]);
if (!optimizeSession.includes('"kind": "mac_ai_switchboard.agent_session_preparation"')) {
  fail("optimize alias did not return an agent session preparation");
}

const directAgents = spawnSync(
  process.execPath,
  ["scripts/repo-intelligence.mjs", ".", "--list-agents"],
  {
    cwd: process.cwd(),
    encoding: "utf8",
    maxBuffer: 4 * 1024 * 1024,
  },
);
if (directAgents.status !== 0) {
  fail(
    `repo:intelligence --list-agents exited ${directAgents.status}: ${directAgents.stderr || directAgents.stdout}`,
  );
}
if (!directAgents.stdout.includes("codex") || !directAgents.stdout.includes("gemini")) {
  fail("npm run repo:intelligence compatibility path did not return expected agent ids");
}

const contractsResult = runSwitchboardContracts();
if (!contractsResult.ok) {
  fail(contractsResult.message);
}

const readme = readFileSync("README.md", "utf8");
const platform = readFileSync("docs/platform-support.md", "utf8");

if (!readme.includes("npm run switchboard -- repo-intelligence") || !readme.includes("docs/platform-support.md")) {
  fail("README must show Switchboard CLI usage and platform support");
}

if (!platform.includes("Linux") || !platform.includes("Windows") || !platform.includes("macOS")) {
  fail("platform support doc must cover macOS, Linux, and Windows");
}

console.log("Switchboard CLI check passed.");
