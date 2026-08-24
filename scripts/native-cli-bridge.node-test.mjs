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

test("native bridge delegates the exact router endpoint plan command", () => {
  const stub = nativeStub();
  try {
    const result = run(["router", "endpoint", "plan", "--native"], {
      SWITCHBOARD_NATIVE_CLI: stub.executable,
    }, "endpoint-json\n");
    assert.equal(result.status, 0);
    assert.deepEqual(JSON.parse(result.stdout.slice("native:".length)), {
      args: ["router", "endpoint", "plan"],
      input: "endpoint-json\n",
    });
    assert.equal(result.stderr, "native stderr\n");
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("endpoint plan without a final native flag fails closed before router aliases", () => {
  const stub = nativeStub();
  try {
    const privateInput = "private-endpoint-input-must-not-be-echoed\n";
    const rejected = [
      ["router", "endpoint", "plan"],
      ["router", "--native", "endpoint", "plan"],
      ["router", "endpoint", "--native", "plan"],
    ];

    for (const args of rejected) {
      const result = run(
        args,
        { SWITCHBOARD_NATIVE_CLI: stub.executable },
        privateInput,
      );
      assert.equal(result.status, 2);
      assert.equal(result.stdout, "");
      assert.equal(
        result.stderr,
        "router endpoint plan requires --native as the final argument.\n",
      );
      assert.doesNotMatch(result.stderr, /private-endpoint-input/);
    }
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("legacy router repo path remains on the Node compatibility path", () => {
  const result = run(
    ["router", ".", "--list-agents"],
    { SWITCHBOARD_NATIVE_CLI: "/path/that/must/not/be/used" },
  );
  assert.equal(result.status, 0);
  assert.match(result.stdout, /^codex$/m);
  assert.match(result.stdout, /^gemini$/m);
  assert.equal(result.stderr, "");
});

test("native bridge fails closed when unset or unusable", () => {
  const stub = nativeStub();
  try {
    const unset = run(["harness", "status", "--native"], {
      SWITCHBOARD_NATIVE_CLI: "",
    });
    assert.equal(unset.status, 1);
    assert.match(unset.stderr, /Set SWITCHBOARD_NATIVE_CLI to an executable native CLI/);

    const routerUnset = run(["router", "endpoint", "plan", "--native"], {
      SWITCHBOARD_NATIVE_CLI: "",
    }, "endpoint-json\n");
    assert.equal(routerUnset.status, 1);
    assert.match(routerUnset.stderr, /Set SWITCHBOARD_NATIVE_CLI to an executable native CLI/);

    const unusable = run(["harness", "status", "--native"], {
      SWITCHBOARD_NATIVE_CLI: join(tmpdir(), "switchboard-native-does-not-exist"),
    });
    assert.equal(unusable.status, 1);
    assert.match(unusable.stderr, /could not start the configured native CLI/);
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("legacy router native shapes and optimize native are rejected", () => {
  const rejected = [
    ["router", ".", "--native"],
    ["router", "endpoint", "plan", "--native", "extra"],
    ["optimize", ".", "--native"],
  ];
  for (const args of rejected) {
    const result = run(args, {
      SWITCHBOARD_NATIVE_CLI: "/path/that/must/not/be/used",
    });
    assert.equal(result.status, 2);
    assert.match(result.stderr, /--native is (supported only for router endpoint plan|not supported for optimize)/);
  }
});
