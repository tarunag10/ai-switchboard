import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = process.cwd();
const scriptPath = path.join(repoRoot, "scripts/check-switchboard-contracts.mjs");

function createCargoStub({ mode = "success", stderrMessage = "cargo stub failure\n" } = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-cargo-stub-"));
  const capturePath = path.join(dir, "capture.jsonl");
  const stubScript = path.join(dir, "cargo-stub.mjs");
  const wrapperPath = path.join(dir, process.platform === "win32" ? "cargo.cmd" : "cargo");

  fs.writeFileSync(
    stubScript,
    [
      'import fs from "node:fs";',
      'const capturePath = process.env.SWITCHBOARD_CARGO_CAPTURE_PATH;',
      'fs.appendFileSync(capturePath, `${JSON.stringify({ argv: process.argv.slice(2), cwd: process.cwd(), cargoNetOffline: process.env.CARGO_NET_OFFLINE ?? null })}\\n`);',
      'if (process.env.SWITCHBOARD_CARGO_MODE === "fail") {',
      '  process.stderr.write(process.env.SWITCHBOARD_CARGO_STDERR ?? "cargo stub failure\\n");',
      "  process.exit(17);",
      "}",
      "process.exit(0);",
      "",
    ].join("\n"),
  );

  if (process.platform === "win32") {
    fs.writeFileSync(
      wrapperPath,
      [
        "@echo off",
        `"${process.execPath}" "${stubScript}" %*`,
        "exit /b %ERRORLEVEL%",
        "",
      ].join("\r\n"),
    );
  } else {
    fs.writeFileSync(
      wrapperPath,
      [
        "#!/bin/sh",
        `exec "${process.execPath}" "${stubScript}" "$@"`,
        "",
      ].join("\n"),
    );
    fs.chmodSync(wrapperPath, 0o755);
  }

  return { cargoDir: dir, capturePath, mode, stderrMessage };
}

function runScript({
  cargoDir,
  capturePath,
  mode = "success",
  stderrMessage,
  cwd,
  cargoNetOffline,
}) {
  const env = {
    ...process.env,
    PATH: cargoDir,
    SWITCHBOARD_CARGO_CAPTURE_PATH: capturePath,
    SWITCHBOARD_CARGO_MODE: mode,
    SWITCHBOARD_CARGO_STDERR: stderrMessage,
  };

  if (cargoNetOffline === null) {
    delete env.CARGO_NET_OFFLINE;
  } else if (cargoNetOffline !== undefined) {
    env.CARGO_NET_OFFLINE = cargoNetOffline;
  }

  return spawnSync(process.execPath, [scriptPath], {
    cwd,
    encoding: "utf8",
    env,
  });
}

test("runs cargo test with exact repo-root manifests and argv-only invocation", () => {
  const outsideCwd = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-contracts-outside-"));
  const stub = createCargoStub();
  try {
    const run = runScript({ ...stub, cwd: outsideCwd, cargoNetOffline: null });
    assert.equal(run.status, 0, run.stderr);
    assert.equal(run.stderr, "");
    assert.match(run.stdout, /Switchboard contract checks passed\./);

    const records = fs
      .readFileSync(stub.capturePath, "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));

    assert.deepEqual(records, [
      {
        argv: ["test", "--locked", "--manifest-path", "crates/switchboard-core/Cargo.toml"],
        cwd: repoRoot,
        cargoNetOffline: null,
      },
      {
        argv: ["test", "--locked", "--manifest-path", "crates/switchboard-runtime/Cargo.toml"],
        cwd: repoRoot,
        cargoNetOffline: null,
      },
      {
        argv: ["test", "--locked", "--manifest-path", "crates/switchboard-cli/Cargo.toml"],
        cwd: repoRoot,
        cargoNetOffline: null,
      },
    ]);
  } finally {
    fs.rmSync(outsideCwd, { recursive: true, force: true });
    fs.rmSync(stub.cargoDir, { recursive: true, force: true });
  }
});

test("preserves a caller-supplied Cargo offline setting", () => {
  const outsideCwd = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-contracts-offline-"));
  const stub = createCargoStub();
  try {
    const run = runScript({
      ...stub,
      cwd: outsideCwd,
      cargoNetOffline: "false",
    });
    assert.equal(run.status, 0, run.stderr);

    const records = fs
      .readFileSync(stub.capturePath, "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));

    assert.deepEqual(
      records.map((record) => record.cargoNetOffline),
      ["false", "false", "false"],
    );
  } finally {
    fs.rmSync(outsideCwd, { recursive: true, force: true });
    fs.rmSync(stub.cargoDir, { recursive: true, force: true });
  }
});

test("propagates cargo failures with a nonzero exit status", () => {
  const outsideCwd = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-contracts-fail-"));
  const stub = createCargoStub({ mode: "fail", stderrMessage: "cargo stub failure\n" });
  try {
    const run = runScript({ ...stub, mode: "fail", cwd: outsideCwd });
    assert.equal(run.status, 17);
    assert.match(run.stderr, /cargo stub failure/);
    assert.match(run.stderr, /switchboard contract check failed: cargo test failed for crates\/switchboard-core\/Cargo\.toml/);
  } finally {
    fs.rmSync(outsideCwd, { recursive: true, force: true });
    fs.rmSync(stub.cargoDir, { recursive: true, force: true });
  }
});

test("prints a stable missing-cargo error when cargo is absent", () => {
  const outsideCwd = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-contracts-missing-"));
  const emptyPath = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-contracts-empty-path-"));
  try {
    const run = spawnSync(process.execPath, [scriptPath], {
      cwd: outsideCwd,
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: emptyPath,
      },
    });
    assert.equal(run.status, 1);
    assert.match(
      run.stderr,
      /switchboard contract check failed: cargo not found on PATH; install Rust or ensure the cargo executable is available/,
    );
  } finally {
    fs.rmSync(outsideCwd, { recursive: true, force: true });
    fs.rmSync(emptyPath, { recursive: true, force: true });
  }
});
