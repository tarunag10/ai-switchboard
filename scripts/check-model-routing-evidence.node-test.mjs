import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

test("offline model-routing evidence remains observe-only", () => {
  const output = execFileSync(process.execPath, ["scripts/check-model-routing-evidence.mjs"], { encoding: "utf8" });
  const result = JSON.parse(output);
  assert.equal(result.ok, true);
  assert.equal(result.evidenceClass, "offline_static_fixture");
  assert.equal(result.promotionEligible, false);
  assert.equal(result.automaticRouting, "observe_only");
});
