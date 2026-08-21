import test from "node:test";
import assert from "node:assert/strict";
import { selectReleaseArtifact } from "./release-artifact-contract.mjs";

test("selects the newest DMG instead of the lexicographically last name", () => {
  const result = selectReleaseArtifact([
    { name: "AI Switchboard_9.9.9.dmg", path: "/old.dmg", mtimeMs: 10 },
    { name: "AI Switchboard_1.0.0.dmg", path: "/new.dmg", mtimeMs: 20 },
  ]);
  assert.equal(result.candidate.path, "/new.dmg");
});

test("requires the expected version when one is supplied", () => {
  const result = selectReleaseArtifact([
    { name: "AI Switchboard_9.9.9.dmg", path: "/old.dmg", mtimeMs: 20 },
  ], { expectedVersion: "1.0.0" });
  assert.equal(result.candidate, null);
  assert.match(result.reason, /expected version/);
});

test("rejects ambiguous newest DMGs", () => {
  const result = selectReleaseArtifact([
    { name: "AI Switchboard_a.dmg", path: "/a.dmg", mtimeMs: 20 },
    { name: "AI Switchboard_b.dmg", path: "/b.dmg", mtimeMs: 20 },
  ]);
  assert.equal(result.candidate, null);
  assert.match(result.reason, /newest modification time/);
});
