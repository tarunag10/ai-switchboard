#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import packageJson from "../package.json" with { type: "json" };

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
if (!help.includes("switchboard repo-intelligence") || !help.includes("Legacy Mac AI Switchboard paths")) {
  fail("help output must document repo-intelligence and legacy compatibility");
}

const version = run(["--version"]).trim();
if (version !== packageJson.version) {
  fail(`version mismatch: ${version}`);
}

const agents = run(["repo-intelligence", ".", "--list-agents"]);
if (!agents.includes("codex") || !agents.includes("gemini")) {
  fail("repo-intelligence wrapper did not return expected agent ids");
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
