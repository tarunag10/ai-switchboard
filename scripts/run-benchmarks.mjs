#!/usr/bin/env node
import fs from "node:fs";
import { performance } from "node:perf_hooks";

const fixturesPath = "benchmarks/fixtures.json";

function estimateTokens(text) {
  return Math.ceil(text.length / 4);
}

function readFixtures() {
  const fixtures = JSON.parse(fs.readFileSync(fixturesPath, "utf8"));
  if (!Array.isArray(fixtures) || fixtures.length === 0) {
    throw new Error(`${fixturesPath} must contain at least one fixture.`);
  }
  return fixtures;
}

function factRetentionPct(relevantFacts, optimizedFacts) {
  if (!Array.isArray(relevantFacts) || relevantFacts.length === 0) {
    return 100;
  }
  const optimized = new Set(optimizedFacts ?? []);
  const retained = relevantFacts.filter((fact) => optimized.has(fact)).length;
  return Math.round((retained / relevantFacts.length) * 1000) / 10;
}

function wrongOmissionRatePct(relevantFacts, wrongOmissions) {
  if (!Array.isArray(relevantFacts) || relevantFacts.length === 0) {
    return 0;
  }
  return (
    Math.round(((wrongOmissions?.length ?? 0) / relevantFacts.length) * 1000) /
    10
  );
}

const startedAt = performance.now();
const samples = readFixtures();

const results = samples.map((sample) => {
  const originalTokens = estimateTokens(sample.original);
  const optimizedTokens = estimateTokens(sample.optimized);
  const savedTokens = Math.max(0, originalTokens - optimizedTokens);
  return {
    category: sample.category,
    name: sample.name,
    originalTokens,
    optimizedTokens,
    savedTokens,
    savedPct:
      Math.round((savedTokens / Math.max(1, originalTokens)) * 1000) / 10,
    latencyOverheadMs: sample.latencyOverheadMs ?? 0,
    relevantFactRetentionPct: factRetentionPct(
      sample.relevantFacts,
      sample.optimizedFacts,
    ),
    wrongOmissionRatePct: wrongOmissionRatePct(
      sample.relevantFacts,
      sample.wrongOmissions,
    ),
    agentSuccessProxy: sample.agentSuccessProxy ?? "not_applicable",
    qualityCheck: "static fixture only; no LLM judging",
  };
});

console.log(
  JSON.stringify(
    {
      generatedAt: new Date().toISOString(),
      fixturePath: fixturesPath,
      suiteRuntimeMs: Math.round(performance.now() - startedAt),
      results,
    },
    null,
    2,
  ),
);
