import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const evidence = JSON.parse(fs.readFileSync(path.join(root, "benchmarks/fixtures/compression-four-variant-evidence.json"), "utf8"));
const expected = ["no_compression", "normal", "cache_safe", "aggressive"];
const failures = [];

if (evidence.routingPolicy?.stage !== "observe") failures.push("compression fixture must remain observe-only");
if (evidence.routingPolicy?.automaticRoutingAllowed !== false) failures.push("compression fixture must not authorize automatic routing");
if (!String(evidence.routingPolicy?.reason ?? "").includes("quality")) failures.push("observe-only reason must explain missing quality evidence");
if (!Array.isArray(evidence.results) || evidence.results.length !== expected.length) failures.push(`expected exactly ${expected.length} compression variants`);

const seen = new Set();
for (const result of evidence.results ?? []) {
  if (!expected.includes(result.variant)) failures.push(`unexpected compression variant: ${result.variant}`);
  if (seen.has(result.variant)) failures.push(`duplicate compression variant: ${result.variant}`);
  seen.add(result.variant);
  if (result.measured !== true) failures.push(`${result.variant}: evidence must be explicitly measured`);
  if (!Number.isInteger(result.sampleCount) || result.sampleCount < 50) failures.push(`${result.variant}: sampleCount must be at least 50`);
  for (const field of ["agentSuccessRateBasisPoints", "relevantFactRetentionBasisPoints", "wrongOmissionRateBasisPoints", "inputTokensSavedBasisPoints", "promptCacheHitRateBasisPoints"]) {
    if (!Number.isInteger(result[field]) || result[field] < 0 || result[field] > 10_000) failures.push(`${result.variant}: invalid ${field}`);
  }
}
if (seen.size !== expected.length) failures.push("compression fixture is missing one or more required variants");

if (failures.length) {
  console.error(failures.join("\n"));
  process.exitCode = 1;
} else {
  console.log(`compression proof ok: ${expected.length} measured variants, routing=observe-only`);
}
