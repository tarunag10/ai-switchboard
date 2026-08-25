import assert from "node:assert/strict";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { test } from "node:test";
import packageJson from "../package.json" with { type: "json" };

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const expectedFiles = [
  "README.md",
  "LICENSE",
  "NOTICE",
  "bin/",
  "scripts/chonkify-adapter.mjs",
  "scripts/repo-intelligence.mjs",
];

const requiredPackFiles = [
  "bin/switchboard.mjs",
  "scripts/repo-intelligence.mjs",
  "scripts/chonkify-adapter.mjs",
];

const forbiddenPrefixes = [
  ".claude/",
  ".codex/",
  ".github/",
  ".playwright-mcp/",
  ".secrets/",
  "dist/",
  "build/",
  "coverage/",
  "target/",
  "node_modules/",
];

const forbiddenExact = new Set([
  ".env",
  ".env.local",
  "console-errors.md",
  "mac-ai-switchboard-audit.md",
]);

const NATIVE_DISABLED_ERROR =
  "Native CLI bridge is disabled. Set SWITCHBOARD_NATIVE_CLI to an executable native CLI, then retry with --native.\n";

function runNpm(args, options) {
  const npmExecPath = process.env.npm_execpath;
  if (npmExecPath) {
    return spawnSync(process.execPath, [npmExecPath, ...args], options);
  }
  return spawnSync(process.platform === "win32" ? "npm.cmd" : "npm", args, options);
}

function packDryRun() {
  const cacheDir = mkdtempSync(join(tmpdir(), "switchboard-npm-pack-cache-"));
  try {
    return runNpm(
      ["pack", "--dry-run", "--json", "--ignore-scripts"],
      {
        cwd: repoRoot,
        env: {
          ...process.env,
          NPM_CONFIG_CACHE: cacheDir,
          NPM_CONFIG_OFFLINE: "true",
        },
        encoding: "utf8",
        maxBuffer: 8 * 1024 * 1024,
      },
    );
  } finally {
    rmSync(cacheDir, { recursive: true, force: true });
  }
}

function buildAndExtractPack(directory) {
  const packDirectory = join(directory, "pack");
  const extractDirectory = join(directory, "extract");
  const cacheDirectory = join(directory, "npm-cache");
  mkdirSync(packDirectory);
  mkdirSync(extractDirectory);
  mkdirSync(cacheDirectory);

  const packResult = runNpm(
    [
      "pack",
      "--json",
      "--ignore-scripts",
      "--pack-destination",
      packDirectory,
    ],
    {
      cwd: repoRoot,
      env: {
        ...process.env,
        NPM_CONFIG_CACHE: cacheDirectory,
        NPM_CONFIG_OFFLINE: "true",
      },
      encoding: "utf8",
      maxBuffer: 8 * 1024 * 1024,
    },
  );
  assert.equal(packResult.error, undefined);
  assert.equal(packResult.status, 0, packResult.stderr);

  const [pack] = JSON.parse(packResult.stdout);
  assert.ok(pack?.filename, "npm pack should produce one named archive");
  const archivePath = join(packDirectory, pack.filename);
  const extractResult = spawnSync(
    "tar",
    ["-xzf", archivePath, "-C", extractDirectory],
    { encoding: "utf8" },
  );
  assert.equal(extractResult.error, undefined);
  assert.equal(extractResult.status, 0, extractResult.stderr);

  return {
    pack,
    packedRoot: join(extractDirectory, "package"),
  };
}

function runPackedCli(packedRoot, args, { env = {}, input = "" } = {}) {
  const childEnv = { ...process.env };
  delete childEnv.SWITCHBOARD_NATIVE_CLI;
  for (const [name, value] of Object.entries(env)) {
    if (value === undefined) {
      delete childEnv[name];
    } else {
      childEnv[name] = value;
    }
  }

  return spawnSync(
    process.execPath,
    [join(packedRoot, "bin", "switchboard.mjs"), ...args],
    {
      cwd: packedRoot,
      env: childEnv,
      input,
      encoding: "utf8",
      maxBuffer: 8 * 1024 * 1024,
    },
  );
}

function createNativeStub(directory) {
  const stubDirectory = join(directory, "native-stub");
  mkdirSync(stubDirectory);
  const script = join(stubDirectory, "native.mjs");
  writeFileSync(
    script,
    [
      "#!/usr/bin/env node",
      "process.stdout.write(JSON.stringify({ args: process.argv.slice(2) }));",
      "",
    ].join("\n"),
  );

  if (process.platform === "win32") {
    const command = join(stubDirectory, "native.cmd");
    writeFileSync(command, `@echo off\r\n"${process.execPath}" "%~dp0native.mjs" %*\r\n`);
    return command;
  }

  chmodSync(script, 0o755);
  return script;
}

test("package.json files allowlist stays truthful and conservative", () => {
  assert.deepEqual(packageJson.files, expectedFiles);
});

test("npm pack omits workspace-local, CI, and evidence paths while retaining CLI payload", () => {
  const result = packDryRun();
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);

  const [pack] = JSON.parse(result.stdout);
  assert.ok(pack, "npm pack should return one package record");

  const packedPaths = new Set(pack.files.map((file) => file.path));

  for (const requiredPath of requiredPackFiles) {
    assert.ok(packedPaths.has(requiredPath), `missing required packed file: ${requiredPath}`);
  }

  for (const filePath of packedPaths) {
    for (const prefix of forbiddenPrefixes) {
      assert.ok(
        !filePath.startsWith(prefix),
        `forbidden workspace-local path was packed: ${filePath}`,
      );
    }
    assert.ok(!forbiddenExact.has(filePath), `forbidden workspace-local file was packed: ${filePath}`);
  }
});

test("real packed CLI runs its Node surface and keeps native Workbench external", () => {
  const directory = mkdtempSync(join(tmpdir(), "switchboard-packed-cli-"));
  try {
    const { pack, packedRoot } = buildAndExtractPack(directory);
    const packedBin = pack.files.find((file) => file.path === "bin/switchboard.mjs");
    assert.ok(packedBin, "packed CLI entrypoint should be present");
    if (process.platform !== "win32") {
      assert.notEqual(packedBin.mode & 0o111, 0, "packed CLI entrypoint should be executable");
    }

    const version = runPackedCli(packedRoot, ["--version"]);
    assert.equal(version.error, undefined);
    assert.equal(version.status, 0, version.stderr);
    assert.equal(version.stdout, `${packageJson.version}\n`);

    const help = runPackedCli(packedRoot, ["--help"]);
    assert.equal(help.status, 0, help.stderr);
    assert.match(help.stdout, /does not bundle a native executable/);
    assert.match(help.stdout, /absolute SWITCHBOARD_NATIVE_CLI path/);

    const statusResult = runPackedCli(packedRoot, ["harness", "status"]);
    assert.equal(statusResult.status, 0, statusResult.stderr);
    const status = JSON.parse(statusResult.stdout);
    assert.equal(status.workbench?.contractAvailable, true);
    assert.equal(status.workbench?.bundledNativeExecutable, false);
    assert.equal(status.workbench?.execution, "external-native-cli-required");
    assert.deepEqual(status.workbench?.externalNativeCli, {
      required: true,
      environmentVariable: "SWITCHBOARD_NATIVE_CLI",
      pathRequirement: "absolute",
    });

    const agents = runPackedCli(packedRoot, [
      "repo-intelligence",
      packedRoot,
      "--list-agents",
    ]);
    assert.equal(agents.status, 0, agents.stderr);
    assert.match(agents.stdout, /^codex$/m);
    assert.match(agents.stdout, /^gemini$/m);

    const privateInput = "private-packed-workbench-input\n";
    const nativeUnset = runPackedCli(
      packedRoot,
      ["workbench", "session", "serialize", "--native"],
      { input: privateInput },
    );
    assert.equal(nativeUnset.status, 1);
    assert.equal(nativeUnset.stdout, "");
    assert.equal(nativeUnset.stderr, NATIVE_DISABLED_ERROR);
    assert.doesNotMatch(nativeUnset.stderr, /private-packed-workbench-input/);

    const nativeStub = createNativeStub(directory);
    assert.equal(isAbsolute(nativeStub), true);
    const delegated = runPackedCli(
      packedRoot,
      ["harness", "status", "--native"],
      { env: { SWITCHBOARD_NATIVE_CLI: nativeStub } },
    );
    assert.equal(delegated.status, 0, delegated.stderr);
    assert.deepEqual(JSON.parse(delegated.stdout), {
      args: ["harness", "status"],
    });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
