import test from "node:test";
import assert from "node:assert/strict";
import { isRebootProofReady } from "./reboot-proof-contract.mjs";

const validProof = {
  proofReady: true,
  releaseGateEvidence: true,
  destructive: false,
  blockers: [],
  trust: { ready: true },
  rebootMarker: {
    matchesCurrentBoot: true,
    installedAppTrustVerified: true,
    armPath: "dist/reboot-level-installed-proof-marker.json",
    armedBootTimeUnixSeconds: 100,
    recordedBootTimeUnixSeconds: 200,
  },
};

test("accepts complete current reboot proof", () => {
  assert.equal(isRebootProofReady(validProof), true);
});

for (const [label, mutate] of [
  ["release gate evidence", (proof) => { proof.releaseGateEvidence = false; }],
  ["destructive flag", (proof) => { proof.destructive = true; }],
  ["blockers", (proof) => { proof.blockers = ["stale marker"]; }],
  ["trust", (proof) => { proof.trust.ready = false; }],
  ["stale marker", (proof) => { proof.rebootMarker.matchesCurrentBoot = false; }],
  ["missing arm path", (proof) => { proof.rebootMarker.armPath = ""; }],
  ["same boot session", (proof) => { proof.rebootMarker.recordedBootTimeUnixSeconds = 100; }],
]) {
  test(`rejects ${label}`, () => {
    const proof = structuredClone(validProof);
    mutate(proof);
    assert.equal(isRebootProofReady(proof), false);
  });
}
