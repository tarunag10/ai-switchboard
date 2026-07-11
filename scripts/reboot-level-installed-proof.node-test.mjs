import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const root = process.cwd();
const armScript = path.join(root, "scripts/arm-reboot-level-installed-proof.mjs");
const markerScript = path.join(root, "scripts/record-reboot-level-installed-proof-marker.mjs");

function run(script, env) {
  return spawnSync(process.execPath, [script], {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
}

test("post-reboot recorder refuses a missing arm receipt without writing a marker", () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-reboot-proof-"));
  const armPath = path.join(directory, "missing-arm.json");
  const markerPath = path.join(directory, "marker.json");
  const result = run(markerScript, {
    MAC_AI_SWITCHBOARD_REBOOT_ARM_PATH: armPath,
    MAC_AI_SWITCHBOARD_REBOOT_MARKER_PATH: markerPath,
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /marker was NOT written/);
  assert.equal(fs.existsSync(markerPath), false);
});

test("arm receipt records a concrete macOS boot-session baseline", () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-reboot-proof-"));
  const armPath = path.join(directory, "arm.json");
  const result = run(armScript, { MAC_AI_SWITCHBOARD_REBOOT_ARM_PATH: armPath });
  assert.equal(result.status, 0, result.stderr);
  const arm = JSON.parse(fs.readFileSync(armPath, "utf8"));
  assert.equal(arm.kind, "mac_ai_switchboard.reboot_level_installed_proof_arm");
  assert.equal(typeof arm.nonce, "string");
  assert.equal(Number.isInteger(arm.bootSession.bootTimeUnixSeconds), true);
});
