#!/usr/bin/env node
import fs from "node:fs";

const proofPath = "dist/reboot-level-installed-proof-summary.json";
const markdownPath = "dist/reboot-level-installed-proof-summary.md";

function fail(message) {
  console.error(`reboot-level installed proof check failed: ${message}`);
  process.exitCode = 1;
}

if (!fs.existsSync(proofPath)) {
  fail(`${proofPath} is missing; run npm run smoke:reboot-level:local first`);
  process.exit();
}
if (!fs.existsSync(markdownPath)) {
  fail(`${markdownPath} is missing; run npm run smoke:reboot-level:local first`);
  process.exit();
}

const proof = JSON.parse(fs.readFileSync(proofPath, "utf8"));
const markdown = fs.readFileSync(markdownPath, "utf8");

if (proof.schemaVersion !== 1) {
  fail("summary schemaVersion must be 1");
}
if (proof.kind !== "mac_ai_switchboard.reboot_level_installed_proof") {
  fail("unexpected proof kind");
}
if (proof.releaseGateEvidence !== proof.proofReady) {
  fail("releaseGateEvidence must match proofReady");
}
if (proof.destructive !== false) {
  fail("destructive must remain false");
}
if (!Array.isArray(proof.blockers)) {
  fail("blockers must be an array");
}
if (!proof.proofReady && proof.blockers.length === 0) {
  fail("blocked proof must list blockers");
}
if (proof.proofReady && proof.blockers.length !== 0) {
  fail("ready proof cannot list blockers");
}
if (!proof.rebootMarker?.path) {
  fail("reboot marker path must be recorded");
}
if (proof.proofReady && proof.rebootMarker.matchesCurrentBoot !== true) {
  fail("ready proof requires a marker for the current boot");
}
if (proof.proofReady && proof.rebootMarker.installedAppTrustVerified !== true) {
  fail("ready proof requires the marker's installed-app trust verification");
}
if (proof.proofReady && !proof.rebootMarker.armPath) {
  fail("ready proof requires a recorded pre-reboot arm path");
}
if (
  proof.proofReady &&
  proof.rebootMarker.armedBootTimeUnixSeconds === proof.rebootMarker.recordedBootTimeUnixSeconds
) {
  fail("ready proof requires different armed and recorded boot sessions");
}
if (!Array.isArray(proof.supportingArtifacts)) {
  fail("supportingArtifacts must be an array");
}
for (const id of [
  "installed-smoke",
  "local-doctor-repair",
  "local-rollback",
  "local-uninstall",
]) {
  if (!proof.supportingArtifacts.some((artifact) => artifact.id === id)) {
    fail(`supporting artifact ${id} missing`);
  }
}
for (const phrase of [
  "Release gate evidence:",
  "Proof ready:",
  "Destructive actions: no",
  "Post-reboot marker:",
  "Reboot Marker Requirement",
  "evidence-gated marker",
]) {
  if (!markdown.includes(phrase)) {
    fail(`${markdownPath} must include ${phrase}`);
  }
}

if (process.exitCode) {
  process.exit();
}

console.log(`Reboot-level installed proof summary OK (${proof.proofReady ? "ready" : "blocked"}).`);
