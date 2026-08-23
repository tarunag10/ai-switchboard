import test from "node:test";
import assert from "node:assert/strict";
import { selectReleaseArtifactFromDirectory } from "./release-artifact-cli.mjs";

function fakeFileSystem(entries) {
  return {
    readdirSync: () => Object.keys(entries),
    statSync: (filePath) => entries[filePath.split("/").pop()],
  };
}

test("selects only the exact-version regular DMG for local installation", () => {
  const path = selectReleaseArtifactFromDirectory(
    "/dmg",
    "1.0.2",
    fakeFileSystem({
      "AI Switchboard_1.0.20.dmg": { isFile: () => true, mtimeMs: 30 },
      "Mac-AI-Switchboard_1.0.2-local-unsigned-aarch64.dmg": {
        isFile: () => true,
        mtimeMs: 20,
      },
    }),
  );
  assert.equal(path, "/dmg/Mac-AI-Switchboard_1.0.2-local-unsigned-aarch64.dmg");
});

test("fails closed for an ambiguous or missing local artifact", () => {
  assert.throws(
    () =>
      selectReleaseArtifactFromDirectory(
        "/dmg",
        "1.0.2",
        fakeFileSystem({
          "AI Switchboard_1.0.2.dmg": { isFile: () => true, mtimeMs: 30 },
          "Mac-AI-Switchboard_1.0.2.dmg": { isFile: () => true, mtimeMs: 20 },
        }),
      ),
    /multiple DMG candidates/,
  );
  assert.throws(
    () =>
      selectReleaseArtifactFromDirectory(
        "/dmg",
        "1.0.2",
        fakeFileSystem({
          "AI Switchboard_1.0.20.dmg": { isFile: () => true, mtimeMs: 30 },
        }),
      ),
    /expected version/,
  );
});
