import assert from "node:assert/strict";
import { chmodSync, existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, delimiter, dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { test } from "node:test";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const switchboard = resolve(repoRoot, "bin/switchboard.mjs");
const NATIVE_SHAPE_ERROR =
  "--native is supported only as the single final argument for harness status, router endpoint plan, and Workbench session serialize.\n";
const WORKBENCH_NATIVE_REQUIRED_ERROR =
  "workbench session serialize requires --native as the final argument.\n";
const HARNESS_STATUS_TRAILING_ERROR =
  "harness status accepts no trailing arguments.\n";

function nativeStub() {
  const directory = mkdtempSync(join(tmpdir(), "switchboard-native-bridge-"));
  const script = join(directory, "native.mjs");
  writeFileSync(
    script,
    [
      "import fs from 'node:fs';",
      "const input = fs.readFileSync(0, 'utf8');",
      "if (process.env.SWITCHBOARD_NATIVE_SENTINEL) fs.writeFileSync(process.env.SWITCHBOARD_NATIVE_SENTINEL, 'invoked');",
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
      ["router", "endpoint", "plan", "--native", "--native"],
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

test("malformed harness and Workbench native shapes fail before dispatch", () => {
  const stub = nativeStub();
  const sentinel = join(stub.directory, "invoked.txt");
  const privateInput = "private-shape-input-must-not-be-echoed\n";
  const rejected = [
    { args: ["harness", "--native", "status"], error: NATIVE_SHAPE_ERROR },
    { args: ["--native", "harness", "status"], error: NATIVE_SHAPE_ERROR },
    { args: ["harness", "status", "--native", "--native"], error: NATIVE_SHAPE_ERROR },
    { args: ["harness", "status", "--native", "extra"], error: NATIVE_SHAPE_ERROR },
    {
      args: ["workbench", "--native", "session", "serialize"],
      error: WORKBENCH_NATIVE_REQUIRED_ERROR,
    },
    {
      args: ["--native", "workbench", "session", "serialize"],
      error: WORKBENCH_NATIVE_REQUIRED_ERROR,
    },
    {
      args: ["workbench", "session", "serialize", "--native", "--native"],
      error: WORKBENCH_NATIVE_REQUIRED_ERROR,
    },
    {
      args: ["workbench", "session", "serialize", "--native", "extra"],
      error: NATIVE_SHAPE_ERROR,
    },
  ];

  try {
    for (const rejectedCase of rejected) {
      rmSync(sentinel, { force: true });
      const result = run(
        rejectedCase.args,
        {
          SWITCHBOARD_NATIVE_CLI: stub.executable,
          SWITCHBOARD_NATIVE_SENTINEL: sentinel,
        },
        privateInput,
      );

      assert.equal(result.status, 2);
      assert.equal(result.stdout, "");
      assert.equal(result.stderr, rejectedCase.error);
      assert.equal(existsSync(sentinel), false);
      assert.doesNotMatch(result.stderr, /private-shape-input/);
    }
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("unsupported commands cannot consume the native flag", () => {
  const stub = nativeStub();
  const sentinel = join(stub.directory, "invoked.txt");
  const rejected = [
    ["--native"],
    ["help", "--native"],
    ["version", "--native"],
    ["repo", ".", "--list-agents", "--native"],
    ["repo-intelligence", ".", "--list-agents", "--native"],
    ["intelligence", ".", "--list-agents", "--native"],
    ["harness", "unknown", "--native"],
    ["unsupported", "--native"],
  ];

  try {
    for (const args of rejected) {
      rmSync(sentinel, { force: true });
      const result = run(args, {
        SWITCHBOARD_NATIVE_CLI: stub.executable,
        SWITCHBOARD_NATIVE_SENTINEL: sentinel,
      });

      assert.equal(result.status, 2);
      assert.equal(result.stdout, "");
      assert.equal(result.stderr, NATIVE_SHAPE_ERROR);
      assert.equal(existsSync(sentinel), false);
    }
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("Workbench serialization requires the native flag without reflecting input", () => {
  const privateInput = "private-workbench-input-must-not-be-echoed\n";
  const result = run(
    ["workbench", "session", "serialize"],
    {},
    privateInput,
  );

  assert.equal(result.status, 2);
  assert.equal(result.stdout, "");
  assert.equal(result.stderr, WORKBENCH_NATIVE_REQUIRED_ERROR);
  assert.doesNotMatch(result.stderr, /private-workbench-input/);
});

test("plain harness status rejects trailing arguments", () => {
  for (const args of [
    ["harness", "status", "extra"],
    ["harness", "status", "extra", "more"],
  ]) {
    const result = run(args);
    assert.equal(result.status, 2);
    assert.equal(result.stdout, "");
    assert.equal(result.stderr, HARNESS_STATUS_TRAILING_ERROR);
  }
});

test("native bridge rejects bare and relative executable values before invocation", () => {
  const stub = nativeStub();
  const sentinel = join(stub.directory, "invoked.txt");
  const path = [stub.directory, process.env.PATH].filter(Boolean).join(delimiter);
  const rejected = [
    basename(stub.executable),
    relative(repoRoot, stub.executable),
  ];

  try {
    for (const configuredExecutable of rejected) {
      rmSync(sentinel, { force: true });
      const result = run(
        ["harness", "status", "--native"],
        {
          PATH: path,
          SWITCHBOARD_NATIVE_CLI: configuredExecutable,
          SWITCHBOARD_NATIVE_SENTINEL: sentinel,
        },
        "private-native-input\n",
      );

      assert.equal(result.status, 1);
      assert.equal(result.stdout, "");
      assert.equal(
        result.stderr,
        "Native CLI bridge requires SWITCHBOARD_NATIVE_CLI to be an absolute executable path.\n",
      );
      assert.equal(existsSync(sentinel), false);
      assert.doesNotMatch(result.stderr, /private-native-input/);
    }
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("native bridge preserves configured absolute executable behavior", () => {
  const stub = nativeStub();
  const sentinel = join(stub.directory, "invoked.txt");
  try {
    const result = run(
      ["harness", "status", "--native"],
      {
        SWITCHBOARD_NATIVE_CLI: stub.executable,
        SWITCHBOARD_NATIVE_SENTINEL: sentinel,
      },
      "absolute-input\n",
    );

    assert.equal(result.status, 0);
    assert.deepEqual(JSON.parse(result.stdout.slice("native:".length)), {
      args: ["harness", "status"],
      input: "absolute-input\n",
    });
    assert.equal(result.stderr, "native stderr\n");
    assert.equal(existsSync(sentinel), true);
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
