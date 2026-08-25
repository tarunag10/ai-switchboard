import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const scriptPath = path.resolve("scripts/run-local-evidence.mjs");

test("verify-only summary ends at release-report and omits public release proof", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-local-evidence-verify-"));
  try {
    const run = spawnSync(process.execPath, [scriptPath, "--verify"], {
      cwd: tempDir,
      encoding: "utf8",
    });

    assert.equal(run.status, 0);
    assert.match(run.stdout, /Local evidence command graph verified: 18 steps\./);

    const summaryPath = path.join(tempDir, "dist", "local-evidence-summary.md");
    const summary = fs.readFileSync(summaryPath, "utf8");
    assert.match(summary, /- Status: verified command graph only/);
    assert.match(
      summary,
      /- pending: Refresh release readiness report \(npm run release:report\)\. Summary: dist\/release-readiness-report\.md\./,
    );
    assert.equal(summary.includes("public-release-proof"), false);

    const lines = summary.trim().split("\n").filter((line) => line.startsWith("- "));
    assert.equal(lines.at(-1)?.includes("Refresh release readiness report"), true);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("local evidence execution stops at release-report", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-local-evidence-run-"));
  const binDir = path.join(tempDir, "bin");
  const logPath = path.join(tempDir, "npm-invocations.log");
  fs.mkdirSync(binDir, { recursive: true });
  fs.writeFileSync(
    path.join(binDir, "npm"),
    [
      "#!/usr/bin/env node",
      "const fs = require('node:fs');",
      "const logPath = process.env.NPM_LOG_PATH;",
      "fs.appendFileSync(logPath, JSON.stringify(process.argv.slice(2)) + '\\n');",
      "process.exit(0);",
      "",
    ].join("\n"),
    { mode: 0o755 },
  );

  try {
    const run = spawnSync(process.execPath, [scriptPath], {
      cwd: tempDir,
      encoding: "utf8",
      env: {
        ...process.env,
        NPM_LOG_PATH: logPath,
        PATH: `${binDir}${path.delimiter}${process.env.PATH ?? ""}`,
      },
    });

    assert.equal(run.status, 0);
    const summaryPath = path.join(tempDir, "dist", "local-evidence-summary.md");
    const summary = fs.readFileSync(summaryPath, "utf8");
    assert.equal(summary.includes("public-release-proof"), false);
    assert.match(summary, /Summary: dist\/release-readiness-report\.md\./);

    const calls = fs.readFileSync(logPath, "utf8").trim().split("\n");
    assert.equal(calls.includes('["run","release:proof"]'), false);
    assert.equal(calls.includes('["run","release:proof:check"]'), false);
    assert.equal(calls.at(-1), '["run","release:report"]');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});
