import test from "node:test";
import assert from "node:assert/strict";
import { validateChecksumAssetEvidence, verifyChecksumText } from "./public-release-proof-contract.mjs";

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
