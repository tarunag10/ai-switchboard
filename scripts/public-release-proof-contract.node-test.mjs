import test from "node:test";
import assert from "node:assert/strict";
import {
  expectedPublicReleaseProofBlockers,
  hasExactPublicReleaseProofBlockers,
  validateChecksumAssetEvidence,
  verifyChecksumText,
} from "./public-release-proof-contract.mjs";
import { publicReleaseGateBlockers } from "./public-release-proof-gate.mjs";

const digest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

test("accepts an uploaded checksum matching the DMG digest", () => {
  const verification = verifyChecksumText(`${digest}  *AI-Switchboard-signed-notarized-aarch64.dmg\n`, digest, "AI-Switchboard-signed-notarized-aarch64.dmg");
  assert.equal(verification.ok, true);
  assert.equal(validateChecksumAssetEvidence({ state: "uploaded", url: "https://example.test/checksum", verification }).ok, true);
});

test("rejects mismatched and malformed checksum content", () => {
  assert.equal(verifyChecksumText(`${"f".repeat(64)}  app.dmg`, digest, "app.dmg").ok, false);
  assert.equal(verifyChecksumText("not a checksum", digest, "app.dmg").ok, false);
});

test("rejects missing upload state or non-HTTPS asset URLs", () => {
  const verification = { ok: true, digest };
  assert.equal(validateChecksumAssetEvidence({ state: "new", url: "https://example.test/checksum", verification }).ok, false);
  assert.equal(validateChecksumAssetEvidence({ state: "uploaded", url: "http://example.test/checksum", verification }).ok, false);
});

test("does not treat an uploaded but mismatched checksum as proof", () => {
  const verification = verifyChecksumText(`${"f".repeat(64)}  app.dmg`, digest, "app.dmg");
  const result = validateChecksumAssetEvidence({ state: "uploaded", url: "https://example.test/checksum", verification });
  assert.equal(result.ok, false);
});

const readyGate = {
  environmentClear: true,
  signedAndNotarized: true,
  updaterFeedReady: true,
  backendValidationReady: true,
  staticSmokePreflightReady: true,
  installedAppSmokeReady: true,
};

test("derives the canonical blocked proof blocker list", () => {
  assert.deepEqual(expectedPublicReleaseProofBlockers({
    checksumVerificationOk: false,
    updaterEvidence: { blockers: ["updater feed endpoint latest.json"] },
    gate: readyGate,
    rebootProofReady: false,
    publicReleaseGateBlockers,
  }), ["public checksum", "updater feed endpoint latest.json", "reboot-level installed proof"]);
});

test("rejects a stale, duplicate, missing, or extra blocker list by exact comparison", () => {
  const expected = expectedPublicReleaseProofBlockers({
    checksumVerificationOk: false,
    updaterEvidence: { blockers: ["updater feed endpoint latest.json"] },
    gate: readyGate,
    rebootProofReady: false,
    publicReleaseGateBlockers,
  });
  for (const candidate of [
    ["stale blocker"],
    [...expected, ...expected.slice(0, 1)],
    ["public checksum", "reboot-level installed proof"],
    [...expected, "unrelated blocker"],
  ]) {
    assert.equal(hasExactPublicReleaseProofBlockers(candidate, expected), false);
  }
  assert.equal(hasExactPublicReleaseProofBlockers(expected, expected), true);
});

test("returns no blockers only when all proof components are ready", () => {
  assert.deepEqual(expectedPublicReleaseProofBlockers({
    checksumVerificationOk: true,
    updaterEvidence: { blockers: [] },
    gate: readyGate,
    rebootProofReady: true,
    publicReleaseGateBlockers,
  }), []);
});
