import test from "node:test";
import assert from "node:assert/strict";
import { canonicalInstalledAppPath, validateAppIdentity } from "./app-identity-contract.mjs";

const expected = {
  productName: "AI Switchboard",
  bundleIdentifier: "com.tarunagarwal.mac-ai-switchboard",
  version: "0.0.2",
};

test("selects only the canonical installed app path", () => {
  assert.equal(canonicalInstalledAppPath, "/Applications/AI Switchboard.app");
});

test("rejects legacy-name or wrong-bundle metadata", () => {
  assert.deepEqual(validateAppIdentity({
    bundleIdentifier: "com.example.legacy",
    version: "0.0.2",
    displayName: "AI Switchboard for Mac",
    bundleName: "AI Switchboard for Mac",
  }, expected), [
    "bundle identifier does not match the canonical app identity",
    "display or bundle name does not match the canonical app identity",
  ]);
});

test("accepts canonical bundle metadata", () => {
  assert.deepEqual(validateAppIdentity({
    bundleIdentifier: expected.bundleIdentifier,
    version: expected.version,
    displayName: expected.productName,
    bundleName: expected.productName,
  }, expected), []);
});
