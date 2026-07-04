#!/usr/bin/env node

import { spawnSync } from "node:child_process";

function fail(message) {
  console.error(`repo intelligence map context check failed: ${message}`);
  process.exit(1);
}

function runRepoIntelligence(args) {
  const result = spawnSync("npm", ["--silent", "run", "repo:intelligence", "--", ".", ...args], {
    cwd: process.cwd(),
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  });
  if (result.status !== 0) {
    fail(result.stderr || result.stdout || `repo:intelligence exited ${result.status}`);
  }
  try {
    return JSON.parse(result.stdout);
  } catch (error) {
    fail(`invalid JSON from repo:intelligence: ${error.message}`);
  }
}

function assertContext(context, label) {
  if (!context || typeof context !== "object") {
    fail(`${label} missing repoMapContext`);
  }
  if (context.available !== true) {
    fail(`${label} repoMapContext is not available`);
  }
  if (!["fresh", "stale", "expired"].includes(context.freshness)) {
    fail(`${label} repoMapContext freshness missing`);
  }
  if (typeof context.ageHours !== "number" || context.ageHours < 0) {
    fail(`${label} repoMapContext ageHours missing`);
  }
  if (typeof context.estimatedTokensAvoided !== "number" || context.estimatedTokensAvoided <= 0) {
    fail(`${label} repoMapContext missing token savings estimate`);
  }
  if (!String(context.tokenSavingsEvidence || "").includes("Approximate")) {
    fail(`${label} repoMapContext missing token savings evidence method`);
  }
}

const manifest = runRepoIntelligence(["--manifest"]);
assertContext(manifest.repoMapContext, "manifest");

const handoff = runRepoIntelligence(["--agent", "codex", "--format", "json"]);
assertContext(handoff.repoMapContext, "Codex handoff");

console.log(`Repo Intelligence map context OK (${manifest.repoMapContext.freshness}, ${manifest.repoMapContext.ageHours}h old).`);
