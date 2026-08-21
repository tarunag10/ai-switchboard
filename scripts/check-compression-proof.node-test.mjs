import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import path from "node:path";

test("compression evidence is complete and cannot authorize automatic routing", () => {
  const root = process.cwd();
  const output = execFileSync(process.execPath, [path.join(root, "scripts/check-compression-proof.mjs")], { cwd: root, encoding: "utf8" });
  assert.match(output, /compression proof ok/);
});
