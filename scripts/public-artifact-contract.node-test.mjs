import test from "node:test";
import assert from "node:assert/strict";
import { inspectPublicArtifact } from "./public-artifact-contract.mjs";

const files = {
  "/tmp/release.dmg": { isFile: () => true },
  "/tmp/not-a-file.dmg": { isFile: () => false },
};
const fileSystem = { statSync: (filePath) => {
  const value = files[filePath];
  if (!value) throw new Error("missing");
  return value;
} };

test("accepts an absolute regular DMG path", () => {
  assert.deepEqual(inspectPublicArtifact("/tmp/release.dmg", fileSystem), {
    provided: true, ok: true, path: "/tmp/release.dmg", reason: null,
  });
});

test("rejects relative, missing, non-DMG, and non-file artifacts", () => {
  for (const [filePath, reason] of [
    ["release.dmg", "public artifact path must be absolute"],
    ["/tmp/missing.dmg", "public artifact file does not exist"],
    ["/tmp/release.zip", "public artifact path must point to a .dmg file"],
    ["/tmp/not-a-file.dmg", "public artifact path must point to a regular file"],
  ]) {
    const result = inspectPublicArtifact(filePath, fileSystem);
    assert.equal(result.ok, false);
    assert.equal(result.reason, reason);
  }
});

test("allows the optional artifact to be omitted", () => {
  assert.deepEqual(inspectPublicArtifact("", fileSystem), {
    provided: false, ok: true, path: null, reason: null,
  });
});
