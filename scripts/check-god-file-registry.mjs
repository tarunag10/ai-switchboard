#!/usr/bin/env node
import { evaluateGodFileRegistry } from "./lib/god-file-registry.mjs";

function fail(message) {
  console.error(`god file registry check failed: ${message}`);
  process.exit(1);
}

let report;
try {
  report = evaluateGodFileRegistry();
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}

if (report.violations.length > 0) {
  for (const entry of report.violations) {
    fail(
      `${entry.path} is ${entry.measuredLines} lines (+${entry.growthLines} vs baseline ${entry.baselineLines}); ceiling is ${entry.lineCeiling}. Split in ${entry.splitSlice} or bump the registry baseline intentionally.`,
    );
  }
}

console.log(
  JSON.stringify(
    {
      ok: true,
      registryPath: "fixtures/god-file-registry.json",
      defaultBudget: report.registry.defaultBudget,
      godFiles: report.entries.map((entry) => ({
        tier: entry.tier,
        id: entry.id,
        path: entry.path,
        measuredLines: entry.measuredLines,
        growthLines: entry.growthLines,
        lineCeiling: entry.lineCeiling,
        splitSlice: entry.splitSlice,
      })),
    },
    null,
    2,
  ),
);
