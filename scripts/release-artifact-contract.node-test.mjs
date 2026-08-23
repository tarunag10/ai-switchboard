import test from "node:test";
import assert from "node:assert/strict";
import { selectReleaseArtifact } from "./release-artifact-contract.mjs";

test("selects the newest DMG instead of the lexicographically last name", () => {
  const result = selectReleaseArtifact([
    { name: "AI Switchboard_9.9.9.dmg", path: "/old.dmg", regularFile: true, mtimeMs: 10 },
    { name: "AI Switchboard_1.0.0.dmg", path: "/new.dmg", regularFile: true, mtimeMs: 20 },
  ]);
  assert.equal(result.candidate.path, "/new.dmg");
});

test("requires the expected version when one is supplied", () => {
  const result = selectReleaseArtifact([
    { name: "AI Switchboard_9.9.9.dmg", path: "/old.dmg", regularFile: true, mtimeMs: 20 },
  ], { expectedVersion: "1.0.0" });
  assert.equal(result.candidate, null);
  assert.match(result.reason, /expected version/);
});

test("rejects ambiguous newest DMGs", () => {
  const result = selectReleaseArtifact([
    { name: "AI Switchboard_1.0.0_a.dmg", path: "/a.dmg", regularFile: true, mtimeMs: 20 },
    { name: "AI Switchboard_1.0.1_b.dmg", path: "/b.dmg", regularFile: true, mtimeMs: 20 },
  ]);
  assert.equal(result.candidate, null);
  assert.match(result.reason, /newest modification time/);
});

test("rejects a same-version DMG for an unrelated product", () => {
  const result = selectReleaseArtifact([
    { name: "Other Product_1.0.0.dmg", path: "/other.dmg", regularFile: true, mtimeMs: 30 },
  ], { expectedVersion: "1.0.0" });
  assert.equal(result.candidate, null);
  assert.match(result.reason, /no DMG candidates found/);
});

test("matches exact versions instead of numeric substrings", () => {
  const result = selectReleaseArtifact([
    { name: "AI Switchboard_1.0.20.dmg", path: "/newer.dmg", regularFile: true, mtimeMs: 30 },
    { name: "AI Switchboard_1.0.2.dmg", path: "/exact.dmg", regularFile: true, mtimeMs: 20 },
  ], { expectedVersion: "1.0.2" });
  assert.equal(result.candidate.path, "/exact.dmg");
});

test("rejects a directory-shaped or duplicate exact-version candidate", () => {
  const directory = selectReleaseArtifact([
    { name: "AI Switchboard_1.0.0.dmg", path: "/dir.dmg", regularFile: false, mtimeMs: 30 },
  ], { expectedVersion: "1.0.0" });
  assert.equal(directory.candidate, null);

  const duplicate = selectReleaseArtifact([
    { name: "AI Switchboard_1.0.0_aarch64.dmg", path: "/a.dmg", regularFile: true, mtimeMs: 30 },
    { name: "Mac-AI-Switchboard_1.0.0.dmg", path: "/b.dmg", regularFile: true, mtimeMs: 20 },
  ], { expectedVersion: "1.0.0" });
  assert.equal(duplicate.candidate, null);
  assert.match(duplicate.reason, /multiple DMG candidates/);
});
