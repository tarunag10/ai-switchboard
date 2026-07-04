#!/usr/bin/env node
import fs from "node:fs";

const requiredFiles = [
  "dist/nightly-golden-path-proof.md",
  "dist/nightly-optimization-telemetry-proof.json",
  "dist/nightly-optimization-telemetry.sqlite",
  "dist/nightly-config-byte-proof.json",
];

function fail(message) {
  console.error(`nightly proof failed: ${message}`);
  process.exit(1);
}

for (const file of requiredFiles) {
  if (!fs.existsSync(file)) {
    fail(`${file} missing`);
  }
}

const report = fs.readFileSync("dist/nightly-golden-path-proof.md", "utf8");
for (const needle of [
  "prompt_cache_events",
  "compaction_decisions",
  "routing_decisions",
  "token_xray_bucket_events",
  "redundancy_hash_events",
  "rtk_preset_metadata_events",
  "config_byte_proof",
]) {
  if (!report.includes(needle)) {
    fail(`summary missing ${needle}`);
  }
}

const optimization = JSON.parse(
  fs.readFileSync("dist/nightly-optimization-telemetry-proof.json", "utf8"),
);
if (optimization.tokenXray?.totalTokens !== 1000) {
  fail("optimization token x-ray proof missing");
}
if (optimization.cacheEfficiency?.uncachedInputTokens !== 400) {
  fail("optimization cache proof missing");
}

const config = JSON.parse(fs.readFileSync("dist/nightly-config-byte-proof.json", "utf8"));
if (config.kind !== "ai_switchboard.nightly_config_byte_proof") {
  fail("config proof kind mismatch");
}
if (config.passed !== true) {
  fail("config byte proof did not pass");
}
for (const fixture of config.fixtures ?? []) {
  if (fixture.byteIdentical !== true) {
    fail(`${fixture.client} config was not byte-identical`);
  }
}

if (!report.includes("Clean install and uninstall in an isolated macOS user profile.")) {
  fail("live install/uninstall prerequisite is not explicit");
}

console.log("Nightly golden-path proof OK.");
