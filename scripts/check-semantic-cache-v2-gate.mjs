#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

const requiredSignals = [
  {
    label: "semantic cache v2 policy",
    file: "src/lib/semanticCachePolicy.ts",
    needles: ["semantic-v2", "describeSemanticCacheV2Policy", "canEnableSemanticCacheV2"],
  },
  {
    label: "semantic cache v2 compatibility flag",
    file: "src-tauri/src/semantic_cache.rs",
    // The shipping cache is the exact response cache. Semantic-v2 remains a
    // compatibility/opt-in contract, so the Rust surface carries the legacy
    // serialized field rather than a product-facing `semantic-v2` label.
    needles: ["semantic_v2_enabled", "set_semantic_cache_v2_enabled", "semanticV2Enabled"],
  },
  {
    label: "semantic cache v2 gate fixture",
    file: "fixtures/semantic-cache-v2-gate-evidence.json",
    needles: ["embeddingModelConsent", "localOnlyBoundary"],
  },
];

function fail(message) {
  console.error(`semantic cache v2 gate check failed: ${message}`);
  process.exit(1);
}

for (const signal of requiredSignals) {
  const absolute = path.join(root, signal.file);
  if (!fs.existsSync(absolute)) {
    fail(`missing ${signal.file}`);
  }
  const contents = fs.readFileSync(absolute, "utf8");
  for (const needle of signal.needles) {
    if (!contents.includes(needle)) {
      fail(`${signal.label} missing needle ${needle} in ${signal.file}`);
    }
  }
}

const fixturePath = path.join(root, "fixtures/semantic-cache-v2-gate-evidence.json");
const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
if (!fixture.embeddingModelConsent) {
  fail("semantic cache v2 gate requires explicit embedding model consent in fixture");
}
if (!fixture.localOnlyBoundary) {
  fail("semantic cache v2 gate requires local-only boundary proof in fixture");
}

console.log(
  JSON.stringify(
    {
      ok: true,
      signals: requiredSignals.map((signal) => signal.label),
      policyVersion: fixture.policyVersion,
    },
    null,
    2,
  ),
);
