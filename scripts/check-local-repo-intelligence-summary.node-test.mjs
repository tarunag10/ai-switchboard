import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const checker = path.resolve("scripts/check-local-repo-intelligence-summary.mjs");

test("local Repo Intelligence summary checker rejects corrupt JSON concisely", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-repo-summary-corrupt-"));
  try {
    fs.mkdirSync(path.join(tempDir, "dist"), { recursive: true });
    fs.writeFileSync(
      path.join(tempDir, "dist/local-repo-intelligence-validation-summary.json"),
      "{\"schemaVersion\":1,",
    );
    const result = spawnSync(process.execPath, [checker], { cwd: tempDir, encoding: "utf8" });
    assert.equal(result.status, 1);
    assert.equal(result.stdout, "");
    assert.match(result.stderr, /repo intelligence summary check failed: .* contains invalid JSON/);
    assert.doesNotMatch(result.stderr, /SyntaxError|at JSON\.parse/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});
