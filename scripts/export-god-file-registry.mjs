#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { evaluateGodFileRegistry } from "./lib/god-file-registry.mjs";

const outputPath = path.join(process.cwd(), "benchmarks/god-file-registry-report.json");

const report = evaluateGodFileRegistry();
const payload = {
  generatedAt: new Date().toISOString(),
  defaultBudget: report.registry.defaultBudget,
  entries: report.entries.map((entry) => ({
    id: entry.id,
    path: entry.path,
    domain: entry.domain,
    splitSlice: entry.splitSlice,
    baselineLines: entry.baselineLines,
    measuredLines: entry.measuredLines,
    growthLines: entry.growthLines,
    lineCeiling: entry.lineCeiling,
    withinGrowth: entry.withinGrowth,
    measuredBytes: entry.measuredBytes,
  })),
};

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log(
  JSON.stringify(
    {
      ok: true,
      outputPath: "benchmarks/god-file-registry-report.json",
      godFileCount: payload.entries.length,
    },
    null,
    2,
  ),
);
