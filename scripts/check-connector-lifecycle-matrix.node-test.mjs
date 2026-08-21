import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import path from "node:path";

test("every managed connector has explicit lifecycle evidence", () => {
  const root = process.cwd();
  const output = execFileSync(process.execPath, [path.join(root, "scripts/check-connector-lifecycle-matrix.mjs")], { cwd: root, encoding: "utf8" });
  assert.match(output, /connector lifecycle matrix ok/);
});
