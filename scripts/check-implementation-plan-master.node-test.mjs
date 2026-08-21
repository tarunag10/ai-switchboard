import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { checkMasterPlan } from "./check-implementation-plan-master.mjs";

test("current master plan has all required evidence boundaries", () => {
  assert.deepEqual(checkMasterPlan(), []);
});

test("missing plan paths fail instead of silently passing", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-master-plan-"));
  try {
    fs.writeFileSync(path.join(tempDir, "plan.md"), "Remaining build work");
    assert.match(checkMasterPlan(tempDir, "plan.md")[0], /missing referenced evidence path|missing required boundary|cannot read package/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("current package exposes every operational gate named by the roadmap", () => {
  const failures = checkMasterPlan();
  assert.equal(failures.some((failure) => failure.includes("missing operational package script")), false, failures.join("\n"));
});
