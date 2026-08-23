#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

const root = process.cwd();
const inventoryPath = path.join(root, "third_party/oss-integrations.json");
const noticesPath = path.join(root, "THIRD_PARTY_NOTICES.md");

function fail(message) {
  console.error(`self-contained OSS inventory check failed: ${message}`);
  process.exit(1);
}

function sha256File(relativePath) {
  return crypto
    .createHash("sha256")
    .update(fs.readFileSync(path.join(root, relativePath)))
    .digest("hex");
}

if (!fs.existsSync(inventoryPath)) fail("missing third_party/oss-integrations.json");
if (!fs.existsSync(noticesPath)) fail("missing THIRD_PARTY_NOTICES.md");

const inventory = JSON.parse(fs.readFileSync(inventoryPath, "utf8"));
if (inventory.schemaVersion !== 1) fail("schemaVersion must be 1");
if (inventory.projectIntent?.use !== "private_research") fail("project intent must be private_research");
if (inventory.projectIntent?.commercialOrMonetaryUseIntended !== false) {
  fail("commercialOrMonetaryUseIntended must be false");
}
if (inventory.projectIntent?.upstreamLicenseTermsStillApply !== true) {
  fail("research intent must preserve upstream licence terms");
}
if (inventory.policy?.runtimeDownloadsAllowedAtTarget !== false) {
  fail("self-contained target must reject runtime downloads");
}
if (inventory.policy?.mutableVersionsAllowedAtTarget !== false) {
  fail("self-contained target must reject mutable versions");
}
if (inventory.policy?.upstreamAttributionRemoved !== false) {
  fail("self-contained target must preserve upstream attribution");
}

const requiredIds = new Set([
  "headroom",
  "caveman",
  "switchboard-pack-compaction",
  "chonkify-upstream",
  "deepseek-harness",
  "nvidia-switchyard",
  "ponytail",
  "markitdown",
  "leanctx",
  "rtk",
  "jcode",
]);
const allowedLicenses = new Set([
  "MIT",
  "Apache-2.0",
  "AI-Switchboard-MIT",
  "Proprietary-Evaluation",
  "Unresolved",
]);
const seen = new Set();
for (const entry of inventory.integrations ?? []) {
  if (!requiredIds.has(entry.id)) fail(`unexpected integration ${entry.id}`);
  if (seen.has(entry.id)) fail(`duplicate integration ${entry.id}`);
  seen.add(entry.id);
  if (!allowedLicenses.has(entry.license)) fail(`${entry.id} has an unknown licence classification`);
  if (!entry.currentDelivery || !entry.targetDelivery || !entry.migrationStatus) {
    fail(`${entry.id} is missing delivery or migration state`);
  }
  if (!Array.isArray(entry.copiedPaths)) fail(`${entry.id} copiedPaths must be an array`);
  if (entry.upstreamCodeEmbedded && !entry.sourceRepository) {
    fail(`${entry.id} embeds upstream code without a source repository`);
  }
  if (entry.upstreamCodeEmbedded && !entry.notice) {
    fail(`${entry.id} embeds upstream code without a notice target`);
  }
  if (entry.currentVersion === "latest" && entry.migrationStatus === "complete") {
    fail(`${entry.id} cannot complete migration with a mutable latest version`);
  }
  if (entry.notice && !/^https?:/.test(entry.notice) && !fs.existsSync(path.join(root, entry.notice))) {
    fail(`${entry.id} notice target does not exist: ${entry.notice}`);
  }
  if (entry.migrationStatus === "complete") {
    if (entry.externalRuntimeRequired || entry.runtimeDownloadRequired) {
      fail(`${entry.id} is marked complete but still requires an external runtime or download`);
    }
    if (!String(entry.currentDelivery).includes("switchboard_native")) {
      fail(`${entry.id} is marked complete without Switchboard-native delivery`);
    }
    if (entry.upstreamCodeEmbedded) {
      if (!/^[0-9a-f]{40}$/.test(entry.exactRevision ?? "")) {
        fail(`${entry.id} embeds upstream code without an exact commit`);
      }
      if (!entry.sourceManifest || !entry.sourceManifestSha256) {
        fail(`${entry.id} embeds upstream code without a source manifest and digest`);
      }
      if (!fs.existsSync(path.join(root, entry.sourceManifest))) {
        fail(`${entry.id} source manifest does not exist`);
      }
      if (sha256File(entry.sourceManifest) !== entry.sourceManifestSha256) {
        fail(`${entry.id} source manifest digest does not match`);
      }
      const source = JSON.parse(fs.readFileSync(path.join(root, entry.sourceManifest), "utf8"));
      if (source.commit !== entry.exactRevision || source.packageVersion !== entry.currentVersion) {
        fail(`${entry.id} source manifest does not match inventory provenance`);
      }
      const copied = new Set(entry.copiedPaths);
      const manifested = new Set(source.files?.map((file) => file.localPath) ?? []);
      if (copied.size !== manifested.size || [...copied].some((file) => !manifested.has(file))) {
        fail(`${entry.id} copied paths do not match its source manifest`);
      }
      for (const file of source.files ?? []) {
        if (file.modified !== false) fail(`${entry.id} source manifest must record copied-file modifications`);
        if (!fs.existsSync(path.join(root, file.localPath))) {
          fail(`${entry.id} copied file does not exist: ${file.localPath}`);
        }
        if (sha256File(file.localPath) !== file.sha256) {
          fail(`${entry.id} copied file hash does not match: ${file.localPath}`);
        }
      }
    }
  }
}
for (const id of requiredIds) {
  if (!seen.has(id)) fail(`missing integration ${id}`);
}

const notices = fs.readFileSync(noticesPath, "utf8");
for (const phrase of [
  "private research",
  "upstream licence",
  "Headroom",
  "DeepSeek Harness",
  "NVIDIA NeMo Switchyard",
  "Ponytail",
  "MarkItDown",
  "leanctx",
  "RTK",
  "JCode",
  "Switchboard Pack Compaction",
]) {
  if (!notices.includes(phrase)) fail(`THIRD_PARTY_NOTICES.md is missing ${phrase}`);
}

console.log(JSON.stringify({
  ok: true,
  integrations: seen.size,
  complete: inventory.integrations.filter((entry) => entry.migrationStatus === "complete").length,
  partial: inventory.integrations.filter((entry) => entry.migrationStatus === "partial").length,
  pending: inventory.integrations.filter((entry) => entry.migrationStatus === "pending").length,
  blocked: inventory.integrations.filter((entry) => entry.migrationStatus === "blocked").length,
  runtimeDownloadsAllowedAtTarget: false,
}, null, 2));
