import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { test } from "node:test";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const helper = resolve(repoRoot, "scripts/build-native-cli.mjs");

function cargoStub() {
  const directory = mkdtempSync(join(tmpdir(), "switchboard-cargo-stub-"));
  const script = join(directory, "cargo-stub.mjs");
  const record = join(directory, "record.json");
  writeFileSync(
    script,
    [
      "import fs from 'node:fs';",
      "fs.writeFileSync(process.env.SWITCHBOARD_CARGO_RECORD, JSON.stringify({ args: process.argv.slice(2), cwd: process.cwd() }));",
      "process.exit(Number(process.env.SWITCHBOARD_CARGO_EXIT || 0));",
      "",
    ].join("\n"),
  );

  let executable = join(directory, "cargo");
  if (process.platform === "win32") {
    executable = join(directory, "cargo.cmd");
    writeFileSync(executable, `@echo off\r\n"${process.execPath}" "%~dp0cargo-stub.mjs" %*\r\n`);
  } else {
    writeFileSync(executable, `#!/bin/sh\nexec "${process.execPath}" "${script}" "$@"\n`);
    chmodSync(executable, 0o755);
  }
  return { directory, executable, record };
}

function run(env = {}) {
  return spawnSync(process.execPath, [helper], {
    cwd: join(repoRoot, "crates"),
    env: { ...process.env, ...env },
    encoding: "utf8",
  });
}

test("native CLI build invokes Cargo from the repository root with exact locked args", () => {
  const stub = cargoStub();
  try {
    const result = run({
      PATH: `${dirname(stub.executable)}${process.platform === "win32" ? ";" : ":"}${process.env.PATH ?? ""}`,
      SWITCHBOARD_CARGO_RECORD: stub.record,
    });
    assert.equal(result.status, 0);
    assert.deepEqual(JSON.parse(readFileSync(stub.record, "utf8")), {
      args: ["build", "--manifest-path", "crates/switchboard-cli/Cargo.toml", "--locked"],
      cwd: repoRoot,
    });
    assert.match(readFileSync(helper, "utf8"), /shell: false/);
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("native CLI build propagates Cargo failures with an actionable error", () => {
  const stub = cargoStub();
  try {
    const result = run({
      PATH: `${dirname(stub.executable)}${process.platform === "win32" ? ";" : ":"}${process.env.PATH ?? ""}`,
      SWITCHBOARD_CARGO_RECORD: stub.record,
      SWITCHBOARD_CARGO_EXIT: "17",
    });
    assert.equal(result.status, 17);
    assert.match(result.stderr, /Native CLI source build failed/);
  } finally {
    rmSync(stub.directory, { recursive: true, force: true });
  }
});

test("native CLI build fails clearly when Cargo is missing", () => {
  const directory = mkdtempSync(join(tmpdir(), "switchboard-no-cargo-"));
  try {
    const result = run({
      PATH: directory,
    });
    assert.equal(result.status, 1);
    assert.match(result.stderr, /requires Cargo/);
    assert.match(result.stderr, /npm run build:native-cli/);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
