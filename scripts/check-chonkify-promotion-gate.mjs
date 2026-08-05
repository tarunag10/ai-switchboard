#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

const requiredSignals = [
  {
    label: "repo pack compression seam",
    file: "src/lib/repoPackCompression.ts",
    needles: ["compressRepoPack", "RepoPackChonkifyAdapter"],
  },
  {
    label: "chonkify promotion gate",
    file: "src/lib/chonkifyPromotionGate.ts",
    needles: ["evaluateChonkifyPromotionGate", "canActivateChonkifyRepoPack"],
  },
  {
    label: "cli chonkify adapter",
    file: "scripts/chonkify-adapter.mjs",
    needles: ["chonkifyPackFiles", "estimateChonkifySavings"],
  },
  {
    label: "repo intelligence compression flag",
    file: "scripts/repo-intelligence.mjs",
    needles: ["--compression", "applyPackCompression"],
  },
];

function fail(message) {
  console.error(`chonkify promotion gate check failed: ${message}`);
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

const provenancePath = path.join(root, "fixtures/chonkify-provenance-evidence.json");
if (!fs.existsSync(provenancePath)) {
  fail("missing fixtures/chonkify-provenance-evidence.json");
}
const provenance = JSON.parse(fs.readFileSync(provenancePath, "utf8"));
if (provenance.license !== "MIT") {
  fail("chonkify provenance fixture must declare MIT license");
}

const benchmarkPath = path.join(root, provenance.wrongOmissionFixturesPath ?? "");
if (!benchmarkPath || !fs.existsSync(benchmarkPath)) {
  fail(`missing wrong-omission fixtures at ${provenance.wrongOmissionFixturesPath}`);
}
const benchmark = JSON.parse(fs.readFileSync(benchmarkPath, "utf8"));
const maxRate = provenance.maxWrongOmissionRatePct ?? 0;
for (const fixture of benchmark.fixtures ?? []) {
  const relevant = fixture.relevantFacts?.length ?? 0;
  const omissions = fixture.wrongOmissions?.length ?? 0;
  const rate = relevant === 0 ? 0 : (omissions / relevant) * 100;
  if (rate > maxRate) {
    fail(`fixture "${fixture.name}" wrong omission rate ${rate}% above gate ${maxRate}%`);
  }
}

console.log(
  JSON.stringify(
    {
      ok: true,
      signals: requiredSignals.map((signal) => signal.label),
      license: provenance.license,
      wrongOmissionGatePct: maxRate,
    },
    null,
    2,
  ),
);
