#!/usr/bin/env node
import fs from "node:fs";
import {
  buildRtkPresetDecision,
  frameworkPresets,
} from "./rtk-presets.mjs";

const PROMPT_KEYS = new Set(["content", "message", "messages", "prompt", "prompts", "raw", "text", "transcript"]);

export function parseArgs(argv) {
  const args = {
    json: false,
    input: undefined,
    output: undefined,
    rtkPresets: false,
    mode: "auto",
    command: "",
    files: [],
    manifests: [],
    presetOutput: "",
    help: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === "--json") args.json = true;
    else if (value === "--input") args.input = argv[++index];
    else if (value === "--output") args.output = argv[++index];
    else if (value === "--rtk-presets") args.rtkPresets = true;
    else if (value === "--mode" || value === "--preset") args.mode = argv[++index];
    else if (value === "--command") args.command = argv[++index];
    else if (value === "--file") args.files.push(readMaybeFile(argv[++index]));
    else if (value === "--manifest") args.manifests.push(readMaybeFile(argv[++index]));
    else if (value === "--preset-output") args.presetOutput = readMaybeFile(argv[++index]);
    else if (value === "--help" || value === "-h") args.help = true;
    else throw new Error(`Unknown argument: ${value}`);
  }
  return args;
}

function readMaybeFile(value) {
  if (!value) return "";
  return fs.existsSync(value) ? fs.readFileSync(value, "utf8") : value;
}

function assertSafeJson(value, trail = []) {
  if (!value || typeof value !== "object") return;
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertSafeJson(item, [...trail, String(index)]));
    return;
  }
  for (const [key, child] of Object.entries(value)) {
    if (PROMPT_KEYS.has(key.toLowerCase())) {
      throw new Error(`Unsafe raw prompt field: ${[...trail, key].join(".")}`);
    }
    assertSafeJson(child, [...trail, key]);
  }
}

function readSnapshot(inputPath) {
  if (!inputPath) return {};
  const snapshot = JSON.parse(fs.readFileSync(inputPath, "utf8"));
  assertSafeJson(snapshot);
  return snapshot;
}

const number = (value) => (Number.isFinite(Number(value)) ? Number(value) : 0);
const pct = (part, total) => `${total > 0 ? ((part / total) * 100).toFixed(1) : "0.0"}%`;
const line = (label, value, hint = "") => `- ${label}: ${value}${hint ? ` (${hint})` : ""}`;

function tokenXray(snapshot) {
  const xray = snapshot.tokenXray ?? snapshot.xray ?? {};
  const prompt = number(xray.promptTokens);
  const completion = number(xray.completionTokens);
  const tools = number(xray.toolTokens);
  const total = number(xray.totalTokens) || prompt + completion + tools;
  const window = number(xray.contextWindow);
  const largest = [...(xray.segments ?? [])]
    .sort((a, b) => number(b.tokens) - number(a.tokens))
    .slice(0, 3)
    .map((segment) => ({ label: segment.label, tokens: number(segment.tokens) }));
  return {
    totalTokens: total,
    contextWindow: window,
    contextUsedPct: pct(total, window),
    promptTokens: prompt,
    completionTokens: completion,
    toolTokens: tools,
    largestSegments: largest,
  };
}

function redundancy(snapshot) {
  const data = snapshot.redundancy ?? {};
  const duplicateTokens = number(data.duplicateTokens);
  const total = number(data.totalTokens) || tokenXray(snapshot).totalTokens;
  return {
    duplicateBlocks: number(data.duplicateBlocks),
    duplicateTokens,
    duplicatePct: pct(duplicateTokens, total),
    repeatedToolCalls: number(data.repeatedToolCalls),
    savingsTokens: number(data.savingsTokens) || duplicateTokens,
  };
}

function cacheEfficiency(snapshot) {
  const cache = snapshot.cacheEfficiency ?? snapshot.cache ?? {};
  const reads = number(cache.cacheReadTokens);
  const writes = number(cache.cacheWriteTokens);
  const total = number(cache.totalInputTokens) || reads + writes + number(cache.uncachedInputTokens);
  return {
    cacheReadTokens: reads,
    cacheWriteTokens: writes,
    uncachedInputTokens: Math.max(0, total - reads - writes),
    hitRate: pct(reads, total),
  };
}

function compaction(snapshot) {
  const data = snapshot.compaction ?? {};
  const before = number(data.beforeTokens);
  const after = number(data.afterTokens);
  const saved = Math.max(0, before - after);
  return {
    beforeTokens: before,
    afterTokens: after,
    savedTokens: saved,
    reductionPct: pct(saved, before),
    triggerTokens: number(data.triggerTokens),
    summaryTokens: number(data.summaryTokens),
  };
}

function modelRouting(snapshot) {
  const data = snapshot.modelRouting ?? {};
  const routes = (data.routes ?? []).map((route) => ({
    model: route.model,
    requests: number(route.requests),
    inputTokens: number(route.inputTokens),
    outputTokens: number(route.outputTokens),
    reason: route.reason ?? "unspecified",
  }));
  return {
    fallbackCount: number(data.fallbackCount),
    routes,
    primaryModel: routes.sort((a, b) => b.requests - a.requests)[0]?.model ?? null,
  };
}

function rtkPresets(snapshot, args) {
  const files = args.files ?? [];
  const manifests = args.manifests ?? [];
  const source = snapshot.rtkPresets ?? {};
  const decision = buildRtkPresetDecision({
    mode: args.mode ?? source.mode ?? "auto",
    command: args.command || source.command || "",
    files: files.length ? files : source.files ?? [],
    manifests: manifests.length ? manifests : source.manifests ?? [],
    output: args.presetOutput || source.output || "",
  });
  return {
    available: frameworkPresets.map((preset) => preset.id),
    selectedPreset: decision.selectedPreset,
    detectedFramework: decision.detectedFramework,
    reason: decision.reason,
    estimatedTokensSaved: decision.estimatedTokensSaved,
    collapsedCounts: decision.collapsedCounts,
  };
}

export function buildOptimizationReport(snapshot = {}, args = {}) {
  assertSafeJson(snapshot);
  return {
    schemaVersion: 1,
    generatedAt: snapshot.generatedAt ?? new Date().toISOString(),
    source: snapshot.source ?? "safe-json-snapshot",
    tokenXray: tokenXray(snapshot),
    redundancy: redundancy(snapshot),
    cacheEfficiency: cacheEfficiency(snapshot),
    compaction: compaction(snapshot),
    modelRouting: modelRouting(snapshot),
    rtkPresets: rtkPresets(snapshot, args),
  };
}

export function renderMarkdown(report) {
  const routeLines = report.modelRouting.routes.length
    ? report.modelRouting.routes.map((route) => line(route.model, `${route.requests} request(s)`, route.reason))
    : ["- None."];
  const segmentLines = report.tokenXray.largestSegments.length
    ? report.tokenXray.largestSegments.map((segment) => line(segment.label, `${segment.tokens} tokens`))
    : ["- None."];
  return [
    "# Optimization Report",
    "",
    `Generated: ${report.generatedAt}`,
    `Source: ${report.source}`,
    "",
    "## Token X-ray",
    line("Total", `${report.tokenXray.totalTokens} tokens`, `${report.tokenXray.contextUsedPct} of context`),
    line("Prompt", `${report.tokenXray.promptTokens} tokens`),
    line("Completion", `${report.tokenXray.completionTokens} tokens`),
    line("Tools", `${report.tokenXray.toolTokens} tokens`),
    ...segmentLines,
    "",
    "## Redundancy",
    line("Duplicate blocks", report.redundancy.duplicateBlocks),
    line("Duplicate tokens", `${report.redundancy.duplicateTokens} tokens`, report.redundancy.duplicatePct),
    line("Repeated tool calls", report.redundancy.repeatedToolCalls),
    line("Estimated savings", `${report.redundancy.savingsTokens} tokens`),
    "",
    "## Cache Efficiency",
    line("Cache reads", `${report.cacheEfficiency.cacheReadTokens} tokens`),
    line("Cache writes", `${report.cacheEfficiency.cacheWriteTokens} tokens`),
    line("Uncached input", `${report.cacheEfficiency.uncachedInputTokens} tokens`),
    line("Hit rate", report.cacheEfficiency.hitRate),
    "",
    "## Compaction",
    line("Before", `${report.compaction.beforeTokens} tokens`),
    line("After", `${report.compaction.afterTokens} tokens`),
    line("Saved", `${report.compaction.savedTokens} tokens`, report.compaction.reductionPct),
    line("Trigger", `${report.compaction.triggerTokens} tokens`),
    "",
    "## Model Routing",
    line("Primary model", report.modelRouting.primaryModel ?? "none"),
    line("Fallbacks", report.modelRouting.fallbackCount),
    ...routeLines,
    "",
    "## RTK Presets",
    line("Available", report.rtkPresets.available.join(", ")),
    line("Selected", report.rtkPresets.selectedPreset ?? "none", report.rtkPresets.reason),
    line("Estimated savings", `${report.rtkPresets.estimatedTokensSaved} tokens`),
    "",
  ].join("\n");
}

function renderRtkPresetMarkdown(report) {
  return [
    "# RTK Presets",
    "",
    line("Available", report.rtkPresets.available.join(", ")),
    line("Selected", report.rtkPresets.selectedPreset ?? "none", report.rtkPresets.reason),
    line("Detected", report.rtkPresets.detectedFramework ?? "none"),
    line("Estimated savings", `${report.rtkPresets.estimatedTokensSaved} tokens`),
    "",
  ].join("\n");
}

function printHelp() {
  console.log(`Usage:
  node scripts/optimization-report.mjs --input snapshot.json [--json] [--output report.md]
  node scripts/optimization-report.mjs --rtk-presets [--json] [--command "npm test"] [--preset-output log.txt]

Inputs must be metrics-only JSON. Raw prompt/message/text fields are rejected.`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help) {
      printHelp();
      process.exit(0);
    }
    const snapshot = readSnapshot(args.input);
    const report = buildOptimizationReport(snapshot, args);
    const payload = args.rtkPresets && !args.input ? report.rtkPresets : report;
    const output = args.json
      ? `${JSON.stringify(payload, null, 2)}\n`
      : args.rtkPresets && !args.input
        ? renderRtkPresetMarkdown(report)
        : renderMarkdown(report);
    if (args.output) fs.writeFileSync(args.output, output);
    else process.stdout.write(output);
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exit(1);
  }
}
