#!/usr/bin/env node
import fs from "node:fs";

const proofPath = "dist/public-release-proof-summary.json";
const markdownPath = "dist/public-release-proof-summary.md";
const requiredArtifactKeys = [
  "releaseReadinessReport",
  "installedSmokeSummary",
  "staticSmokeSummary",
  "signedDmg",
  "updaterFeed",
];

function fail(message) {
  console.error(`public release proof check failed: ${message}`);
  process.exitCode = 1;
}

if (!fs.existsSync(proofPath)) {
  fail(`${proofPath} is missing; run npm run release:proof first`);
  process.exit();
}
if (!fs.existsSync(markdownPath)) {
  fail(`${markdownPath} is missing; run npm run release:proof first`);
  process.exit();
}

const proof = JSON.parse(fs.readFileSync(proofPath, "utf8"));
const markdown = fs.readFileSync(markdownPath, "utf8");
const expectedBlockedProofBlockers = ["updater feed"];

if (proof.kind !== "mac_ai_switchboard.public_release_proof") {
  fail("kind must be mac_ai_switchboard.public_release_proof");
}
if (proof.schemaVersion !== 1) {
  fail("schemaVersion must be 1");
}
if (proof.releaseGateEvidence !== proof.proofReady) {
  fail("releaseGateEvidence must match proofReady");
}
if (!Array.isArray(proof.blockers)) {
  fail("blockers must be an array");
}
for (const key of requiredArtifactKeys) {
  if (!proof.requiredArtifacts?.[key]) {
    fail(`requiredArtifacts.${key} is missing`);
  }
}
if (proof.requiredArtifacts.staticSmokeSummary !== "dist/smoke-preflight-summary.md") {
  fail("requiredArtifacts.staticSmokeSummary must be dist/smoke-preflight-summary.md");
}
const excludedLocalEvidence = proof.localOnlyEvidenceExcluded ?? [];
for (const localOnlyArtifact of [
  "dist/local-installed-smoke-summary.md",
  "dist/local-rollback-validation-summary.md",
  "dist/local-doctor-repair-validation-summary.md",
  "dist/local-connector-readiness-summary.md",
  "dist/measured-savings-benchmark.md",
]) {
  if (!excludedLocalEvidence.includes(localOnlyArtifact)) {
    fail(`localOnlyEvidenceExcluded missing ${localOnlyArtifact}`);
  }
}
for (const phrase of [
  "Proof ready:",
  "Signed/notarized release asset present:",
  "Updater feed ready:",
  "Installed app smoke ready:",
  "Local-Only Evidence Excluded",
]) {
  if (!markdown.includes(phrase)) {
    fail(`${markdownPath} must include ${phrase}`);
  }
}

if (proof.proofReady) {
  if (proof.blockers.length !== 0) {
    fail("proofReady cannot be true while blockers are present");
  }
  for (const [key, value] of Object.entries(proof.shareableDmgGate ?? {})) {
    if (typeof value === "boolean" && value !== true) {
      fail(`shareableDmgGate.${key} must be true when proofReady is true`);
    }
  }
} else if (proof.blockers.length === 0) {
  fail("blocked public release proof must list blockers");
}
if (!proof.proofReady) {
  if (!proof.githubRelease?.signedDmgAsset?.url) {
    fail("blocked proof must still record signed/notarized DMG asset evidence when available");
  }
  if (!proof.githubRelease?.checksumAsset?.url) {
    fail("blocked proof must still record public checksum asset evidence when available");
  }
  for (const blocker of expectedBlockedProofBlockers) {
    if (!proof.blockers.includes(blocker)) {
      fail(`blocked public release proof missing blocker: ${blocker}`);
    }
  }
}

if (process.exitCode) {
  process.exit();
}

console.log(`Public release proof summary OK (${proof.proofReady ? "ready" : "blocked"}).`);
