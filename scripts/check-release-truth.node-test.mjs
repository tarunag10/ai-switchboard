import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import path from "node:path";

test("release truth contract is internally consistent", () => {
  const root = process.cwd();
  const output = execFileSync(process.execPath, [path.join(root, "scripts/check-release-truth.mjs")], { cwd: root, encoding: "utf8" });
  assert.match(output, /release truth ok/);
});
