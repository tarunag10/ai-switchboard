#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cargoArgs = [
  "build",
  "--manifest-path",
  "crates/switchboard-cli/Cargo.toml",
  "--locked",
];

const result = spawnSync("cargo", cargoArgs, {
  cwd: repoRoot,
  stdio: "inherit",
  shell: false,
});

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error(
      "Native CLI source build requires Cargo. Install Rust/Cargo with rustup, then retry npm run build:native-cli.",
    );
  } else {
    console.error("Native CLI source build could not start Cargo.");
  }
  process.exit(1);
}

if (typeof result.status !== "number") {
  console.error("Native CLI source build terminated without an exit status.");
  process.exit(1);
}

if (result.status !== 0) {
  console.error("Native CLI source build failed. Check Cargo output above and retry.");
}

process.exit(result.status);
