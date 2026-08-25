#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "..");

function runCargoTest(manifestPath) {
  return spawnSync(
    "cargo",
    ["test", "--locked", "--manifest-path", manifestPath],
    {
      cwd: REPO_ROOT,
      shell: false,
      stdio: "inherit",
    },
  );
}

function runSwitchboardContracts() {
  for (const manifestPath of [
    "crates/switchboard-core/Cargo.toml",
    "crates/switchboard-runtime/Cargo.toml",
    "crates/switchboard-cli/Cargo.toml",
  ]) {
    const result = runCargoTest(manifestPath);
    if (result.error?.code === "ENOENT") {
      return {
        ok: false,
        code: 1,
        message:
          "cargo not found on PATH; install Rust or ensure the cargo executable is available",
      };
    }
    if (result.error) {
      throw result.error;
    }
    if (result.status !== 0) {
      return {
        ok: false,
        code: result.status ?? 1,
        message: `cargo test failed for ${manifestPath}`,
      };
    }
  }

  return { ok: true, code: 0 };
}

function main() {
  const result = runSwitchboardContracts();
  if (!result.ok) {
    console.error(`switchboard contract check failed: ${result.message}`);
    return result.code;
  }

  console.log("Switchboard contract checks passed.");
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main();
}

export { REPO_ROOT, main, runCargoTest, runSwitchboardContracts };
