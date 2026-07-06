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
  "updaterSignatureAssets",
  "rebootLevelInstalledProof",
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
const updaterBlockers = {
  releaseAsset: "updater feed release asset latest.json",
  endpoint: "updater feed endpoint latest.json",
  signatureAsset: "updater signature release asset",
  signatureMetadata: "updater feed signature metadata",
};

function requireBoolean(path, value) {
  if (typeof value !== "boolean") {
    fail(`${path} must be boolean`);
  }
}

function expectBlocker(condition, blocker, message) {
  const hasBlocker = proof.blockers.includes(blocker);
  if (condition && !hasBlocker) {
    fail(`${message}: missing blocker "${blocker}"`);
  }
  if (!condition && hasBlocker) {
    fail(`${message}: blocker "${blocker}" is present despite ready evidence`);
  }
}

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
  "Evidence Reconciliation",
  "Updater Evidence",
  "Signed/notarized release asset present:",
  "Updater feed/signature ready:",
  "Remaining updater feed release asset proof:",
  "Remaining updater feed endpoint proof:",
  "Remaining updater signature release-asset proof:",
  "Installed app smoke ready:",
  "Reboot-level installed proof ready:",
  "Local-Only Evidence Excluded",
]) {
  if (!markdown.includes(phrase)) {
    fail(`${markdownPath} must include ${phrase}`);
  }
}
if (proof.evidenceReconciliation?.completedToday?.signedNotarizedDmgAsset !== Boolean(proof.githubRelease?.signedDmgAsset?.url && proof.githubRelease?.checksumAsset?.url)) {
  fail("evidenceReconciliation.completedToday.signedNotarizedDmgAsset must match live DMG/checksum evidence");
}
if (proof.evidenceReconciliation?.completedToday?.publicChecksumAsset !== Boolean(proof.githubRelease?.checksumAsset?.url)) {
  fail("evidenceReconciliation.completedToday.publicChecksumAsset must match checksum evidence");
}
const updaterEvidence = proof.updaterEvidence;
if (!updaterEvidence || typeof updaterEvidence !== "object") {
  fail("updaterEvidence must be present");
}
if (!Array.isArray(updaterEvidence?.blockers)) {
  fail("updaterEvidence.blockers must be an array");
}
if (!Array.isArray(updaterEvidence?.signatureAssets)) {
  fail("updaterEvidence.signatureAssets must be an array");
}
if (!Array.isArray(updaterEvidence?.checkedEndpoints)) {
  fail("updaterEvidence.checkedEndpoints must be an array");
}
requireBoolean("updaterEvidence.ready", updaterEvidence?.ready);
requireBoolean(
  "updaterEvidence.defaultEndpointUsed",
  updaterEvidence?.defaultEndpointUsed,
);
if (typeof updaterEvidence?.defaultEndpoint !== "string") {
  fail("updaterEvidence.defaultEndpoint must be a string");
}
if (updaterEvidence?.defaultEndpointUsed && updaterEvidence.checkedEndpoints.length !== 1) {
  fail("default updater proof must check exactly one default endpoint");
}
for (const [index, endpoint] of (updaterEvidence?.checkedEndpoints ?? []).entries()) {
  if (typeof endpoint.url !== "string" || !endpoint.url.startsWith("https://")) {
    fail(`updaterEvidence.checkedEndpoints[${index}].url must be an HTTPS URL`);
  }
  requireBoolean(`updaterEvidence.checkedEndpoints[${index}].ok`, endpoint.ok);
  requireBoolean(
    `updaterEvidence.checkedEndpoints[${index}].hasSignatureMetadata`,
    endpoint.hasSignatureMetadata,
  );
}
const updaterReleaseAssetMissing = !updaterEvidence?.releaseAsset?.url;
const updaterEndpointMissing = !(updaterEvidence?.checkedEndpoints ?? []).some(
  (endpoint) => endpoint.ok,
);
const updaterSignatureAssetsMissing =
  (updaterEvidence?.signatureAssets ?? []).length === 0;
const updaterSignatureMetadataMissing =
  !updaterEndpointMissing &&
  !(updaterEvidence?.checkedEndpoints ?? []).some(
    (endpoint) => endpoint.ok && endpoint.hasSignatureMetadata,
  );
const expectedUpdaterReady =
  !updaterReleaseAssetMissing &&
  !updaterEndpointMissing &&
  !updaterSignatureAssetsMissing &&
  !updaterSignatureMetadataMissing;
if (updaterEvidence?.ready !== expectedUpdaterReady) {
  fail("updaterEvidence.ready must match live updater asset/endpoint/signature evidence");
}
for (const blocker of Object.values(updaterBlockers)) {
  if (updaterEvidence?.blockers?.includes(blocker) && !proof.blockers.includes(blocker)) {
    fail(`top-level blockers missing updater blocker: ${blocker}`);
  }
}
expectBlocker(
  updaterReleaseAssetMissing,
  updaterBlockers.releaseAsset,
  "updater release asset evidence",
);
expectBlocker(
  updaterEndpointMissing,
  updaterBlockers.endpoint,
  "updater endpoint evidence",
);
expectBlocker(
  updaterSignatureAssetsMissing,
  updaterBlockers.signatureAsset,
  "updater signature asset evidence",
);
expectBlocker(
  updaterSignatureMetadataMissing,
  updaterBlockers.signatureMetadata,
  "updater feed signature metadata evidence",
);
if (proof.blockers.includes("updater feed/signature assets")) {
  fail("generic updater feed/signature assets blocker must be split into exact updater blockers");
}
const remainingProof = proof.evidenceReconciliation?.remainingProof ?? {};
for (const key of [
  "updaterFeedReleaseAssetLatestJson",
  "updaterFeedEndpointLatestJson",
  "updaterSignatureReleaseAssets",
  "updaterFeedSignatureMetadata",
  "rebootLevelInstalledProof",
]) {
  requireBoolean(`evidenceReconciliation.remainingProof.${key}`, remainingProof[key]);
}
if (remainingProof.updaterFeedReleaseAssetLatestJson !== updaterReleaseAssetMissing) {
  fail("remainingProof.updaterFeedReleaseAssetLatestJson must match release asset evidence");
}
if (remainingProof.updaterFeedEndpointLatestJson !== updaterEndpointMissing) {
  fail("remainingProof.updaterFeedEndpointLatestJson must match endpoint evidence");
}
if (remainingProof.updaterSignatureReleaseAssets !== updaterSignatureAssetsMissing) {
  fail("remainingProof.updaterSignatureReleaseAssets must match signature asset evidence");
}
if (remainingProof.updaterFeedSignatureMetadata !== updaterSignatureMetadataMissing) {
  fail("remainingProof.updaterFeedSignatureMetadata must match feed signature metadata evidence");
}
if (
  proof.rebootLevelInstalledProof &&
  proof.rebootLevelInstalledProof.proofReady !== true &&
  !proof.blockers.includes("reboot-level installed proof")
) {
  fail("blocked reboot-level proof artifact must not satisfy public release proof");
}
if (
  proof.rebootLevelInstalledProof?.proofReady === true &&
  proof.rebootLevelInstalledProof?.releaseGateEvidence !== true
) {
  fail("ready reboot-level proof must carry releaseGateEvidence true");
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
  if (
    proof.rebootLevelInstalledProof?.proofReady !== true &&
    !proof.blockers.includes("reboot-level installed proof")
  ) {
    fail("blocked public release proof missing blocker: reboot-level installed proof");
  }
}

if (process.exitCode) {
  process.exit();
}

console.log(`Public release proof summary OK (${proof.proofReady ? "ready" : "blocked"}).`);
