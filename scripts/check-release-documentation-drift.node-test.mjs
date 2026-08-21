import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

test("release documentation claims remain explicitly scoped", () => {
  const output = execFileSync(process.execPath, ["scripts/check-release-documentation-drift.mjs"], { encoding: "utf8" });
  assert.equal(JSON.parse(output).ok, true);
});
