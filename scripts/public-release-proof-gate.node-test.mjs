import test from "node:test";
import assert from "node:assert/strict";
import {
  isShareableDmgGateReady,
  publicReleaseGateBlockers,
} from "./public-release-proof-gate.mjs";

const readyGate = {
  ready: true,
  environmentClear: true,
  signedAndNotarized: true,
  updaterFeedReady: true,
  backendValidationReady: true,
  staticSmokePreflightReady: true,
  installedAppSmokeReady: true,
};

test("accepts a fully ready shareable gate", () => {
  assert.equal(isShareableDmgGateReady(readyGate), true);
  assert.deepEqual(publicReleaseGateBlockers(readyGate), []);
});

test("reports every missing public gate component", () => {
  const blockedGate = {
    ...readyGate,
    environmentClear: false,
    backendValidationReady: false,
    staticSmokePreflightReady: false,
    installedAppSmokeReady: false,
  };
  assert.equal(isShareableDmgGateReady(blockedGate), false);
  assert.deepEqual(publicReleaseGateBlockers(blockedGate), [
    "release environment",
    "backend validation",
    "static smoke preflight",
    "public installed-app smoke",
  ]);
});

test("fails closed for absent and non-boolean gate components", () => {
  assert.equal(isShareableDmgGateReady(null), false);
  assert.equal(isShareableDmgGateReady({ environmentClear: true }), false);
  assert.deepEqual(publicReleaseGateBlockers({ environmentClear: true }), [
    "signed/notarized DMG",
    "updater feed",
    "backend validation",
    "static smoke preflight",
    "public installed-app smoke",
  ]);
});
