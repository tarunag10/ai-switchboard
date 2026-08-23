import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const root = process.cwd();
const checker = path.join(root, "scripts/check-self-contained-oss-inventory.mjs");
const inventory = JSON.parse(
  fs.readFileSync(path.join(root, "third_party/oss-integrations.json"), "utf8"),
);
const notices = fs.readFileSync(path.join(root, "THIRD_PARTY_NOTICES.md"), "utf8");

function runFixture(mutator = () => {}) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-oss-inventory-"));
  try {
    const fixture = structuredClone(inventory);
    mutator(fixture);
    fs.mkdirSync(path.join(directory, "third_party"));
    fs.cpSync(
      path.join(root, "third_party/ponytail"),
      path.join(directory, "third_party/ponytail"),
      { recursive: true },
    );
    fs.writeFileSync(
      path.join(directory, "third_party/oss-integrations.json"),
      `${JSON.stringify(fixture, null, 2)}\n`,
    );
    fs.writeFileSync(path.join(directory, "THIRD_PARTY_NOTICES.md"), notices);
    fs.copyFileSync(path.join(root, "LICENSE"), path.join(directory, "LICENSE"));
    fs.copyFileSync(path.join(root, "NOTICE"), path.join(directory, "NOTICE"));
    return spawnSync(process.execPath, [checker], {
      cwd: directory,
      encoding: "utf8",
    });
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
}

test("accepts the authoritative self-contained migration inventory", () => {
  const result = runFixture();
  assert.equal(result.status, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.equal(report.integrations, 11);
  assert.equal(report.complete, 3);
  assert.equal(report.runtimeDownloadsAllowedAtTarget, false);
});

test("rejects a completed integration that still tracks latest", () => {
  const result = runFixture((fixture) => {
    const ponytail = fixture.integrations.find((entry) => entry.id === "ponytail");
    ponytail.currentVersion = "latest";
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /mutable latest version/);
});

test("rejects drift in a completed embedded source manifest", () => {
  const result = runFixture((fixture) => {
    const ponytail = fixture.integrations.find((entry) => entry.id === "ponytail");
    ponytail.sourceManifestSha256 = "0".repeat(64);
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /source manifest digest does not match/);
});

test("rejects research intent that discards upstream licence obligations", () => {
  const result = runFixture((fixture) => {
    fixture.projectIntent.upstreamLicenseTermsStillApply = false;
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /preserve upstream licence terms/);
});

test("rejects embedded upstream code without a notice target", () => {
  const result = runFixture((fixture) => {
    const deepseek = fixture.integrations.find((entry) => entry.id === "deepseek-harness");
    deepseek.upstreamCodeEmbedded = true;
    deepseek.notice = null;
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /embeds upstream code without a notice target/);
});
