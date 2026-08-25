#!/usr/bin/env node
// Gate for the Class C coding-agent task benchmark. Fails when:
//  - the task schema or any task definition is invalid,
//  - run results reference unknown task_ids or duplicate result_ids,
//  - an emitted evidence block claims promotionEligible=true while the shared,
//    recomputed thresholds disagree (observe-only evidence must stay false),
//  - or the committed/checked summary drifts from what the current inputs
//    deterministically produce.

import fs from "node:fs";
import path from "node:path";
import { buildSummary, renderMarkdown, CLASS_C_MINIMUM_SAMPLES } from "./lib/class-c-tasks.mjs";
import { evaluatePromotionEligibility } from "./lib/model-routing-evidence.mjs";

const root = process.cwd();
const args = process.argv.slice(2);

try {
  const summary = buildSummary(root);
  let failures = [];
  for (const [taskClass, stats] of Object.entries(summary.perClass)) {
    const evidence = stats.evidence?.evidence;
    if (!evidence) continue;
    if (evidence.promotionEligible !== evaluatePromotionEligibility(evidence).eligible) {
      failures.push(`${taskClass}: promotionEligible does not match recomputed threshold result`);
    }
    if (evidence.evidenceClass === "local_runtime_observation" && evidence.promotionEligible !== false) {
      failures.push(`${taskClass}: observe-only Class C evidence must remain promotionEligible=false`);
    }
    if (evidence.baseline.sampleCount < CLASS_C_MINIMUM_SAMPLES) {
      failures.push(`${taskClass}: evidence emitted below minimum samples`);
    }
  }

  const outputDir = path.join(root, "benchmarks/results");
  const jsonPath = path.join(outputDir, "class-c-summary.json");
  const mdPath = path.join(outputDir, "class-c-summary.md");
  const expectedJson = `${JSON.stringify(summary, null, 2)}\n`;
  const expectedMd = renderMarkdown(summary);
  for (const [filePath, expected] of [[jsonPath, expectedJson], [mdPath, expectedMd]]) {
    if (!fs.existsSync(filePath)) {
      failures.push(`missing ${path.relative(root, filePath)}; run: node scripts/run-class-c-tasks.mjs`);
      continue;
    }
    if (fs.readFileSync(filePath, "utf8") !== expected) {
      failures.push(`${path.relative(root, filePath)} drifted from deterministic rebuild; rerun the runner and commit the regenerated file`);
    }
  }

  if (failures.length > 0) {
    for (const failure of failures) console.error(`class-c tasks check failed: ${failure}`);
    process.exit(1);
  }
  console.log(JSON.stringify({
    ok: true,
    taskCount: summary.taskCount,
    runResults: summary.runResults.total,
    classes: Object.fromEntries(
      Object.entries(summary.perClass).map(([name, stats]) => [
        name,
        {
          baselineSamples: stats.baseline.sampleCount,
          candidateSamples: stats.candidate.sampleCount,
          evidenceEmitted: Boolean(stats.evidence),
          promotionEligible: stats.evidence ? stats.evidence.evidence.promotionEligible : false,
        },
      ]),
    ),
  }, null, 2));
} catch (error) {
  console.error(`class-c tasks check failed: ${error.message}`);
  process.exit(1);
}
