import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { test } from "node:test";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const switchboard = resolve(repoRoot, "bin/switchboard.mjs");

function nativeStub() {
  const directory = mkdtempSync(join(tmpdir(), "switchboard-native-bridge-"));
  const script = join(directory, "native.mjs");
  writeFileSync(
    script,
    [
      "import fs from 'node:fs';",
      "const input = fs.readFileSync(0, 'utf8');",
      "process.stdout.write(`native:${JSON.stringify({ args: process.argv.slice(2), input })}\\n`);",
      "process.stderr.write('native stderr\\n');",
      "",
    ].join("\n"),
  );

  let executable = script;
  if (process.platform === "win32") {
    executable = join(directory, "native.cmd");
    writeFileSync(executable, `@echo off\r\n"${process.execPath}" "%~dp0native.mjs" %*\r\n`);
  } else {
    executable = join(directory, "native");
    writeFileSync(executable, `#!/bin/sh\nexec "${process.execPath}" "${script}" "$@"\n`);
    chmodSync(executable, 0o755);
  }
  return { directory, executable };
}

function run(args, env = {}, input = "") {
  return spawnSync(process.execPath, [switchboard, ...args], {
    cwd: repoRoot,
    env: { ...process.env, ...env },
    encoding: "utf8",
    input,
  });
}

test("native bridge delegates status and forwards streams", () => {
  const stub = nativeStub();
  try {
    const result = run(["harness", "status", "--native"], {
      SWITCHBOARD_NATIVE_CLI: stub.executable,
    }, "stdin-payload\n");
    assert.equal(result.status, 0);
    assert.deepEqual(JSON.parse(result.stdout.slice("native:".length)), {
      args: ["harness", "status"],
      input: "stdin-payload\n",
    });
    assert.equal(result.stderr, "native stderr\n");
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("native bridge delegates Workbench serialization only when opted in", () => {
  const stub = nativeStub();
  try {
    const result = run(["workbench", "session", "serialize", "--native"], {
      SWITCHBOARD_NATIVE_CLI: stub.executable,
    }, "session-json\n");
    assert.equal(result.status, 0);
    assert.deepEqual(JSON.parse(result.stdout.slice("native:".length)), {
      args: ["workbench", "session", "serialize"],
      input: "session-json\n",
    });
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("native bridge fails closed when unset or unusable", () => {
  const stub = nativeStub();
  try {
    const unset = run(["harness", "status", "--native"], {
      SWITCHBOARD_NATIVE_CLI: "",
    });
    assert.equal(unset.status, 1);
    assert.match(unset.stderr, /Set SWITCHBOARD_NATIVE_CLI to an executable native CLI/);

    const unusable = run(["harness", "status", "--native"], {
      SWITCHBOARD_NATIVE_CLI: join(tmpdir(), "switchboard-native-does-not-exist"),
    });
    assert.equal(unusable.status, 1);
    assert.match(unusable.stderr, /could not start the configured native CLI/);
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("native opt-in is rejected for router and optimize", () => {
  for (const command of ["router", "optimize"]) {
    const result = run([command, ".", "--native"], {
      SWITCHBOARD_NATIVE_CLI: "/path/that/must/not/be/used",
    });
    assert.equal(result.status, 2);
    assert.match(result.stderr, /supported only for harness status/);
  }
});
