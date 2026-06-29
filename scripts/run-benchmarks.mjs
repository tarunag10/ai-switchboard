#!/usr/bin/env node
const samples = [
  {
    category: "shell_output",
    name: "noisy test log",
    original: Array.from({ length: 80 }, (_, i) => `test_${i} ... ok`).join("\n"),
    optimized: "80 tests passed",
  },
  {
    category: "repo_context_pack",
    name: "task-aware pack vs broad scan",
    original: "full repo scan ".repeat(1200),
    optimized: "ranked implementation pack ".repeat(160),
  },
  {
    category: "document_conversion",
    name: "office/pdf markdown handoff",
    original: "layout text with repeated headers ".repeat(400),
    optimized: "clean markdown sections ".repeat(120),
  },
];

function estimateTokens(text) {
  return Math.ceil(text.length / 4);
}

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
    savedPct: Math.round((savedTokens / Math.max(1, originalTokens)) * 1000) / 10,
    qualityCheck: "static fixture only; no LLM judging",
  };
});

console.log(JSON.stringify({ generatedAt: new Date().toISOString(), results }, null, 2));
