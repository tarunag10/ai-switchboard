import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const checker = path.resolve("scripts/check-public-release-proof-summary.mjs");

test("public proof checker rejects corrupt generated JSON without a stack trace", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-public-proof-corrupt-"));
  try {
    const dist = path.join(tempDir, "dist");
    fs.mkdirSync(dist, { recursive: true });
    const proofPath = path.join(dist, "public-release-proof-summary.json");
    const markdownPath = path.join(dist, "public-release-proof-summary.md");
    const corrupt = "{\"kind\":\"mac_ai_switchboard.public_release_proof\",";
    fs.writeFileSync(proofPath, corrupt);
    fs.writeFileSync(markdownPath, "# Public Release Proof Summary\n");
    const result = spawnSync(process.execPath, [checker], { cwd: tempDir, encoding: "utf8" });
    assert.equal(result.status, 1);
    assert.equal(result.stdout, "");
    assert.match(result.stderr, /public release proof check failed: dist\/public-release-proof-summary\.json contains invalid JSON/);
    assert.doesNotMatch(result.stderr, /SyntaxError|at JSON\.parse/);
    assert.equal(fs.readFileSync(proofPath, "utf8"), corrupt);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});
