#!/usr/bin/env node
import { execSync } from "node:child_process";
import fs from "node:fs";

const summaryPath = "dist/world-class-token-savings-summary.json";
const markdownPath = "dist/world-class-token-savings-summary.md";

function run(command) {
  return execSync(command, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

const plan = JSON.parse(run("node scripts/check-world-class-token-savings-plan.mjs"));
const benchmarks = JSON.parse(run("node scripts/check-world-class-benchmarks.mjs"));
const benchmarkRun = JSON.parse(run("npm run benchmarks --silent"));

const summary = {
  generatedAt: new Date().toISOString(),
  plan,
  benchmarks,
  benchmarkRun: {
    fixtureCount: benchmarkRun.results?.length ?? 0,
    suiteRuntimeMs: benchmarkRun.suiteRuntimeMs ?? null,
  },
};

fs.mkdirSync("dist", { recursive: true });
fs.writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
fs.writeFileSync(
  markdownPath,
  `# World-Class Token Savings Local Summary

Generated: ${summary.generatedAt}

## Plan check

- Plan directory: \`${plan.planDir}\`
- Analysis date: ${plan.analysisDate}
- In-progress slices: ${plan.inProgressSlices}

## Benchmark gate

- Fixture count: ${benchmarks.fixtureCount}
- Categories: ${benchmarks.categories.join(", ")}
- Benchmark runtime: ${summary.benchmarkRun.suiteRuntimeMs} ms
`,
);

console.log(JSON.stringify(summary, null, 2));
