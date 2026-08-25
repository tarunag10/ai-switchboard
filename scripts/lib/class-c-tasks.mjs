// Class C coding-agent task benchmark core: loads task definitions and
// operator-produced run results, aggregates deterministic per-taskClass
// baseline-vs-candidate arm stats, and emits ModelRoutingQualityEvidence-
// shaped blocks using the shared promotion thresholds from
// scripts/lib/model-routing-evidence.mjs.
//
// Honesty boundary: this module never executes tasks or talks to providers.
// It only aggregates run-result files that an operator or CI produced, and it
// emits observe-only evidence (evidenceClass "local_runtime_observation"),
// which the shared evaluator structurally keeps promotionEligible=false until
// real approved live-run evidence is produced through
// scripts/check-model-routing-evidence.mjs.

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { validateAgainstSchema } from "./json-schema-mini.mjs";
import { evaluatePromotionEligibility, thresholdChecks } from "./model-routing-evidence.mjs";

export const CLASS_C_SCHEMA_VERSION = 1;
export const CLASS_C_MINIMUM_SAMPLES = 100;
export const TASKS_DIR = "benchmarks/tasks";
export const RESULTS_SUBDIR = "results";

function readJsonFile(filePath) {
  let parsed;
  try {
    parsed = JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    throw new Error(`${filePath} contains invalid JSON`);
  }
  return parsed;
}

export function loadTaskDefinitions(root) {
  const tasksDir = path.join(root, TASKS_DIR);
  const schemaPath = path.join(tasksDir, "schema.json");
  const schema = readJsonFile(schemaPath);
  const definitions = [];
  for (const entry of fs.readdirSync(tasksDir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    if (!entry.isFile() || !entry.name.endsWith(".json") || entry.name === "schema.json") continue;
    const filePath = path.join(tasksDir, entry.name);
    const value = readJsonFile(filePath);
    const validation = validateAgainstSchema(schema, value, schema, `task ${entry.name}`);
    if (!validation.ok) throw new Error(`${filePath} fails benchmarks/tasks/schema.json: ${validation.errors.join("; ")}`);
    assertRepoFixtureIntegrity(filePath, value);
    definitions.push({ file: entry.name, ...value });
  }
  return { schema, definitions };
}

function assertRepoFixtureIntegrity(filePath, task) {
  const fixture = task.repo_fixture;
  if (fixture.kind === "repo_path") {
    if (!fixture.path || fixture.synthetic !== false) {
      throw new Error(`${filePath}: repo_path fixtures must set repo_fixture.path and synthetic=false`);
    }
    if (!fs.existsSync(path.resolve(filePath, "..", "..", "..", fixture.path))) {
      throw new Error(`${filePath}: repo_path fixture target ${fixture.path} does not exist in the repository`);
    }
  } else if (fixture.synthetic !== true || !Array.isArray(fixture.files) || fixture.files.length === 0) {
    throw new Error(`${filePath}: inline_synthetic fixtures must be marked synthetic=true and embed at least one file`);
  }
}

export function loadRunResults(root, schema) {
  const resultsDir = path.join(root, TASKS_DIR, RESULTS_SUBDIR);
  if (!fs.existsSync(resultsDir)) return [];
  const results = [];
  for (const entry of fs.readdirSync(resultsDir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    if (!entry.isFile() || !entry.name.endsWith(".json")) continue;
    const filePath = path.join(resultsDir, entry.name);
    const value = readJsonFile(filePath);
    const documents = Array.isArray(value) ? value : [value];
    for (const [index, document] of documents.entries()) {
      const label = `result ${entry.name}[${index}]`;
      const validation = validateAgainstSchema({ $ref: "#/$defs/runResult" }, document, schema, label);
      if (!validation.ok) throw new Error(`${filePath}: ${validation.errors.join("; ")}`);
      results.push({ sourceFile: entry.name, ...document });
    }
  }
  return results.sort((a, b) => a.result_id.localeCompare(b.result_id));
}

function bps(part, whole) {
  return whole === 0 ? 0 : Math.floor((part * 10_000) / whole);
}

function p95(values) {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.ceil(sorted.length * 0.95) - 1];
}

function modalValue(values) {
  const counts = new Map();
  let best = null;
  let bestCount = -1;
  for (const value of values) {
    const count = (counts.get(value) ?? 0) + 1;
    counts.set(value, count);
    if (count > bestCount || (count === bestCount && value < best)) {
      best = value;
      bestCount = count;
    }
  }
  return { value: best, count: bestCount, distinct: counts.size };
}

export function computeArmStats(runs, taskById) {
  const successful = runs.filter((run) => run.test_result === "pass");
  const scopeCompliant = successful.filter((run) => {
    const allowed = taskById.get(run.task_id).allowed_files;
    return run.files_changed.every((file) => allowed.includes(file));
  });
  const reworked = successful.filter((run) => run.retries > 0);
  const model = modalValue(runs.map((run) => run.selected_model));
  return {
    sampleCount: runs.length,
    successfulTaskCount: successful.length,
    successRateBps: bps(successful.length, runs.length),
    qualityScoreBps: bps(scopeCompliant.length, successful.length),
    p95LatencyMs: p95(successful.map((run) => run.elapsed_ms)),
    successfulTaskCostMicros: 0,
    followUpReworkRateBps: bps(reworked.length, successful.length),
    totals: {
      inputTokens: runs.reduce((sum, run) => sum + run.input_tokens, 0),
      outputTokens: runs.reduce((sum, run) => sum + run.output_tokens, 0),
      providerCacheReadTokens: runs.reduce((sum, run) => sum + run.provider_cache_read_tokens, 0),
      linesChanged: runs.reduce((sum, run) => sum + run.lines_changed, 0),
      toolCalls: runs.reduce((sum, run) => sum + run.tool_calls, 0),
    },
    provenance: model,
  };
}

function stableDigest(value) {
  return crypto.createHash("sha256").update(JSON.stringify(value)).digest("hex").slice(0, 12);
}

export function buildEvidenceBlock(taskClass, baseline, candidate, minimumSamples, capturedAtValues = []) {
  if (baseline.provenance.distinct !== 1 || candidate.provenance.distinct !== 1) {
    return { emitted: false, reason: "arm models are not uniform across runs; refusing to synthesize provenance" };
  }
  if (baseline.provenance.value === candidate.provenance.value) {
    return { emitted: false, reason: "baseline and candidate selected_model must differ" };
  }
  if (baseline.sampleCount < minimumSamples || candidate.sampleCount < minimumSamples) {
    return { emitted: false, reason: `minimum of ${minimumSamples} samples per arm not met` };
  }
  const block = {
    schemaVersion: 1,
    evidenceClass: "local_runtime_observation",
    promotionEligible: false,
    minimumSamples,
    provenance: {
      taskClass,
      baselineModel: baseline.provenance.value,
      candidateModel: candidate.provenance.value,
      source: "local_runtime_observation",
      costAttribution: "local_estimate",
      runId: `class-c-${stableDigest([taskClass, baseline, candidate])}`,
      capturedAt: capturedAtValues.length > 0 ? capturedAtValues[capturedAtValues.length - 1] : "operator-timestamps-not-provided",
    },
    baseline: {
      sampleCount: baseline.sampleCount,
      successfulTaskCount: baseline.successfulTaskCount,
      successRateBps: baseline.successRateBps,
      qualityScoreBps: baseline.qualityScoreBps,
      p95LatencyMs: baseline.p95LatencyMs,
      successfulTaskCostMicros: baseline.successfulTaskCostMicros,
      followUpReworkRateBps: baseline.followUpReworkRateBps,
    },
    candidate: {
      sampleCount: candidate.sampleCount,
      successfulTaskCount: candidate.successfulTaskCount,
      successRateBps: candidate.successRateBps,
      qualityScoreBps: candidate.qualityScoreBps,
      p95LatencyMs: candidate.p95LatencyMs,
      successfulTaskCostMicros: candidate.successfulTaskCostMicros,
      followUpReworkRateBps: candidate.followUpReworkRateBps,
    },
    note: "Aggregated from operator-recorded Class C task outcomes; no live provider runs are claimed.",
  };
  const eligibility = evaluatePromotionEligibility(block);
  block.promotionEligible = eligibility.eligible;
  return {
    emitted: true,
    evidence: block,
    eligibility,
    thresholdsMet: thresholdChecks(block),
  };
}

export function buildSummary(root, options = {}) {
  const minimumSamples = options.minimumSamples ?? CLASS_C_MINIMUM_SAMPLES;
  const { schema, definitions } = loadTaskDefinitions(root);
  const results = loadRunResults(root, schema);
  const taskById = new Map(definitions.map((task) => [task.task_id, task]));
  for (const result of results) {
    if (!taskById.has(result.task_id)) {
      throw new Error(`run result ${result.result_id} references unknown task_id '${result.task_id}'`);
    }
  }
  const byId = new Map(results.map((result) => [result.result_id, result]));
  if (byId.size !== results.length) {
    throw new Error("duplicate result_id values across run-result files");
  }

  const classes = [...new Set(definitions.map((task) => task.task_class))].sort();
  const capturedAtValues = [...new Set(results.map((result) => result.captured_at).filter(Boolean))].sort();
  const perClass = {};
  for (const taskClass of classes) {
    const classTaskIds = definitions
      .filter((task) => task.task_class === taskClass)
      .map((task) => task.task_id)
      .sort();
    const classRuns = results.filter((result) => classTaskIds.includes(result.task_id));
    const baselineRuns = classRuns.filter((result) => result.arm === "baseline");
    const candidateRuns = classRuns.filter((result) => result.arm === "candidate");
    const baseline = computeArmStats(baselineRuns, taskById);
    const candidate = computeArmStats(candidateRuns, taskById);
    const evidence = buildEvidenceBlock(taskClass, baseline, candidate, minimumSamples, capturedAtValues);
    perClass[taskClass] = {
      tasks: classTaskIds,
      baseline: publicArmStats(baseline),
      candidate: publicArmStats(candidate),
      evidence: evidence.emitted ? evidence : null,
      evidenceOmittedReason: evidence.emitted ? null : evidence.reason,
      ...(evidence.emitted ? { thresholdsMet: evidence.thresholdsMet } : {}),
    };
  }

  return {
    schemaVersion: CLASS_C_SCHEMA_VERSION,
    generator: "scripts/run-class-c-tasks.mjs",
    minimumSamples,
    taskCount: definitions.length,
    taskIds: definitions.map((task) => task.task_id).sort(),
    runResults: {
      total: results.length,
      baseline: results.filter((result) => result.arm === "baseline").length,
      candidate: results.filter((result) => result.arm === "candidate").length,
      capturedAtValues,
      claimsLiveProviderRuns: false,
    },
    perClass,
  };
}

function publicArmStats(stats) {
  const { provenance, ...rest } = stats;
  return { ...rest, selectedModel: provenance.value ?? null, selectedModelRuns: provenance.count ?? 0 };
}

export function renderMarkdown(summary) {
  const lines = [
    "# Class C coding-agent task benchmark summary",
    "",
    `- Generator: \`${summary.generator}\``,
    `- Minimum samples per arm: ${summary.minimumSamples}`,
    `- Task definitions: ${summary.taskCount}`,
    `- Run results: ${summary.runResults.total} (baseline ${summary.runResults.baseline}, candidate ${summary.runResults.candidate})`,
    "- Live provider claims: **none** (observe-only aggregate)",
    "",
  ];
  for (const [taskClass, stats] of Object.entries(summary.perClass)) {
    lines.push(`## ${taskClass}`, "");
    lines.push("| Arm | Samples | Passes | Success bps | Quality bps | p95 ms | Rework bps | Cost micros | Selected model |");
    lines.push("|---|---:|---:|---:|---:|---:|---:|---:|---|");
    for (const arm of ["baseline", "candidate"]) {
      const s = stats[arm];
      lines.push(`| ${arm} | ${s.sampleCount} | ${s.successfulTaskCount} | ${s.successRateBps} | ${s.qualityScoreBps} | ${s.p95LatencyMs} | ${s.followUpReworkRateBps} | ${s.successfulTaskCostMicros} | ${s.selectedModel ?? "n/a"} |`);
    }
    lines.push("");
    if (stats.evidence) {
      lines.push(`- Evidence block emitted: evidenceClass=${stats.evidence.evidence.evidenceClass}, promotionEligible=${stats.evidence.evidence.promotionEligible}`);
      lines.push(`- Threshold checks: ${JSON.stringify(stats.thresholdsMet)}`);
    } else {
      lines.push(`- Evidence block omitted: ${stats.evidenceOmittedReason}`);
    }
    lines.push("");
  }
  return `${lines.join("\n")}\n`;
}
