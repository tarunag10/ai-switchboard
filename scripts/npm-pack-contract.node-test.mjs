import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
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

function packDryRun() {
  const cacheDir = mkdtempSync("/private/tmp/switchboard-npm-pack-cache-");
  try {
    return spawnSync(
      process.env.npm_execpath ?? "npm",
      ["pack", "--dry-run", "--json", "--ignore-scripts"],
      {
        cwd: repoRoot,
        env: {
          ...process.env,
          NPM_CONFIG_CACHE: cacheDir,
        },
        encoding: "utf8",
        maxBuffer: 8 * 1024 * 1024,
      },
    );
  } finally {
    rmSync(cacheDir, { recursive: true, force: true });
  }
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
