import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const root = process.cwd();
const script = path.join(root, "scripts/vercel-ignore-build.mjs");

function run(changedFiles, { omitPrevious = false } = {}) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-vercel-ignore-"));
  const previous = "HEAD^";
  const current = "HEAD";
  const git = path.join(directory, "git");
  fs.writeFileSync(git, `#!/bin/sh\nprintf '%s\\n' ${changedFiles.map((file) => JSON.stringify(file)).join(" ")}\n`);
  fs.chmodSync(git, 0o755);
  const env = {
    ...process.env,
    PATH: `${directory}:${process.env.PATH}`,
    VERCEL_GIT_PREVIOUS_SHA: previous,
    VERCEL_GIT_COMMIT_SHA: current,
  };
  if (omitPrevious) delete env.VERCEL_GIT_PREVIOUS_SHA;
  const result = spawnSync(process.execPath, [script], {
    cwd: root,
    encoding: "utf8",
    env,
  });
  return result;
}

test("skips native-only changes", () => {
  const result = run(["src-tauri/src/lib.rs", "docs/implementation-plan-reconciliation.md"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /skipping/);
});

test("continues for web-shell changes", () => {
  const result = run(["src/App.tsx", "src-tauri/src/lib.rs"]);
  assert.equal(result.status, 1, result.stderr);
  assert.match(result.stdout, /continuing/);
});

test("uses HEAD^ when Vercel omits the previous-commit variable", () => {
  const result = run(["src-tauri/src/lib.rs"], { omitPrevious: true });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /skipping/);
});
