import fs from "node:fs";
import os from "node:os";
import { spawnSync } from "node:child_process";

export function estimateTokens(text) {
  return Math.ceil(text.length / 4);
}

function round(value, digits = 1) {
  const scale = 10 ** digits;
  return Math.round(value * scale) / scale;
}

function factRetentionPct(relevantFacts, optimizedFacts) {
  if (!Array.isArray(relevantFacts) || relevantFacts.length === 0) {
    return 100;
  }
  const optimized = new Set(optimizedFacts ?? []);
  const retained = relevantFacts.filter((fact) => optimized.has(fact)).length;
  return round((retained / relevantFacts.length) * 100);
}

function wrongOmissionRatePct(relevantFacts, wrongOmissions) {
  if (!Array.isArray(relevantFacts) || relevantFacts.length === 0) {
    return 0;
  }
  return round(((wrongOmissions?.length ?? 0) / relevantFacts.length) * 100);
}

export function buildResults(fixtures) {
  return fixtures.map((fixture) => {
    const originalTokens = estimateTokens(fixture.original);
    const optimizedTokens = estimateTokens(fixture.optimized);
    const savedTokens = Math.max(0, originalTokens - optimizedTokens);
    return {
      category: fixture.category,
      name: fixture.name,
      originalTokens,
      optimizedTokens,
      savedTokens,
      savedPct: round((savedTokens / Math.max(1, originalTokens)) * 100),
      latencyOverheadMs: fixture.latencyOverheadMs ?? 0,
      relevantFactRetentionPct: factRetentionPct(
        fixture.relevantFacts,
        fixture.optimizedFacts,
      ),
      wrongOmissionRatePct: wrongOmissionRatePct(
        fixture.relevantFacts,
        fixture.wrongOmissions,
      ),
      agentSuccessProxy: fixture.agentSuccessProxy ?? "not_applicable",
      qualityLabel: "offline_static_fixture",
      qualityCheck: "static fixture only; no LLM judging",
    };
  });
}

export function aggregateResults(results) {
  const originalTokens = results.reduce((sum, result) => sum + result.originalTokens, 0);
  const optimizedTokens = results.reduce((sum, result) => sum + result.optimizedTokens, 0);
  const savedTokens = originalTokens - optimizedTokens;
  const applicable = results.filter(
    (result) => result.agentSuccessProxy !== "not_applicable",
  );
  const passing = applicable.filter((result) => result.agentSuccessProxy === "pass");

  return {
    fixtureCount: results.length,
    originalTokens,
    optimizedTokens,
    savedTokens,
    savedPct: round((savedTokens / Math.max(1, originalTokens)) * 100),
    averageLatencyOverheadMs: round(
      results.reduce((sum, result) => sum + result.latencyOverheadMs, 0) /
        Math.max(1, results.length),
    ),
    minimumRelevantFactRetentionPct: Math.min(
      ...results.map((result) => result.relevantFactRetentionPct),
    ),
    maximumWrongOmissionRatePct: Math.max(
      ...results.map((result) => result.wrongOmissionRatePct),
    ),
    staticSuccessPassRatePct: round(
      (passing.length / Math.max(1, applicable.length)) * 100,
    ),
  };
}

function resultKey(result) {
  return `${result.category}/${result.name}`;
}

export function compareWithBaseline(manifest, baseline, thresholds) {
  const violations = [];
  const aggregate = manifest.aggregate;
  const baselineAggregate = baseline.aggregate;

  if (
    baselineAggregate.savedPct - aggregate.savedPct >
    thresholds.maximumAggregateSavedPctDrop
  ) {
    violations.push(
      `aggregate saved percent dropped ${round(baselineAggregate.savedPct - aggregate.savedPct)} points`,
    );
  }
  if (
    aggregate.averageLatencyOverheadMs - baselineAggregate.averageLatencyOverheadMs >
    thresholds.maximumAggregateLatencyIncreaseMs
  ) {
    violations.push("aggregate average latency exceeded its allowed increase");
  }
  if (
    baselineAggregate.minimumRelevantFactRetentionPct -
      aggregate.minimumRelevantFactRetentionPct >
    thresholds.maximumFactRetentionDropPct
  ) {
    violations.push("minimum fact retention regressed beyond its threshold");
  }
  if (
    aggregate.maximumWrongOmissionRatePct -
      baselineAggregate.maximumWrongOmissionRatePct >
    thresholds.maximumWrongOmissionIncreasePct
  ) {
    violations.push("maximum wrong omission rate regressed beyond its threshold");
  }
  if (aggregate.staticSuccessPassRatePct < thresholds.minimumStaticSuccessPassRatePct) {
    violations.push("static success pass rate fell below its threshold");
  }

  const baselineByKey = new Map(baseline.results.map((result) => [resultKey(result), result]));
  for (const result of manifest.results) {
    const previous = baselineByKey.get(resultKey(result));
    if (!previous) {
      continue;
    }
    if (previous.savedPct - result.savedPct > thresholds.maximumPerFixtureSavedPctDrop) {
      violations.push(`${resultKey(result)} saved percent regressed beyond its threshold`);
    }
    if (
      result.latencyOverheadMs - previous.latencyOverheadMs >
      thresholds.maximumPerFixtureLatencyIncreaseMs
    ) {
      violations.push(`${resultKey(result)} latency regressed beyond its threshold`);
    }
  }

  return {
    baselineVersion: baseline.baselineVersion,
    status: violations.length === 0 ? "pass" : "regression",
    thresholds,
    violations,
  };
}

export function resolveSwitchboardCommit(root, env = process.env) {
  if (env.SWITCHBOARD_BENCHMARK_COMMIT) {
    return env.SWITCHBOARD_BENCHMARK_COMMIT;
  }
  const result = spawnSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  });
  return result.status === 0 ? result.stdout.trim() : "unknown";
}

export function buildManifest({ root, fixtures, schema, baseline, env = process.env }) {
  if (!Array.isArray(fixtures) || fixtures.length === 0) {
    throw new Error("benchmark fixtures must contain at least one fixture");
  }
  const results = buildResults(fixtures);
  const metadata = schema.manifest;
  const manifest = {
    schemaVersion: schema.schemaVersion,
    switchboardCommit: resolveSwitchboardCommit(root, env),
    platform: env.SWITCHBOARD_BENCHMARK_PLATFORM ?? `${os.platform()}-${os.arch()}`,
    fixturesVersion: metadata.fixturesVersion,
    headroomVersion:
      env.SWITCHBOARD_BENCHMARK_HEADROOM_VERSION ?? metadata.offlineHeadroomVersion,
    rtkVersion: env.SWITCHBOARD_BENCHMARK_RTK_VERSION ?? metadata.offlineRtkVersion,
    profile: env.SWITCHBOARD_BENCHMARK_PROFILE ?? metadata.defaultProfile,
    results,
    aggregate: aggregateResults(results),
  };
  return {
    ...manifest,
    baselineComparison: compareWithBaseline(
      manifest,
      baseline,
      schema.regressionThresholds,
    ),
  };
}

export function renderMarkdown(manifest) {
  const rows = manifest.results.map(
    (result) =>
      `| ${result.category} | ${result.name} | ${result.originalTokens} | ${result.optimizedTokens} | ${result.savedPct}% | ${result.latencyOverheadMs} ms | ${result.relevantFactRetentionPct}% | ${result.wrongOmissionRatePct}% | ${result.agentSuccessProxy} | ${result.qualityLabel} |`,
  );
  const comparison = manifest.baselineComparison;
  const violations = comparison.violations.length
    ? comparison.violations.map((violation) => `- ${violation}`).join("\n")
    : "- None.";

  return `# Offline benchmark summary

- Switchboard commit: \`${manifest.switchboardCommit}\`
- Platform: \`${manifest.platform}\`
- Fixtures: \`${manifest.fixturesVersion}\`
- Headroom version evidence: \`${manifest.headroomVersion}\`
- RTK version evidence: \`${manifest.rtkVersion}\`
- Profile: \`${manifest.profile}\`
- Baseline comparison: **${comparison.status}** (baseline ${comparison.baselineVersion})

## Aggregate

| Fixtures | Original tokens | Optimized tokens | Saved | Saved % | Avg latency | Min retention | Max wrong omission | Static success |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ${manifest.aggregate.fixtureCount} | ${manifest.aggregate.originalTokens} | ${manifest.aggregate.optimizedTokens} | ${manifest.aggregate.savedTokens} | ${manifest.aggregate.savedPct}% | ${manifest.aggregate.averageLatencyOverheadMs} ms | ${manifest.aggregate.minimumRelevantFactRetentionPct}% | ${manifest.aggregate.maximumWrongOmissionRatePct}% | ${manifest.aggregate.staticSuccessPassRatePct}% |

## Fixtures

| Category | Fixture | Original | Optimized | Saved | Latency | Retention | Wrong omission | Success proxy | Quality label |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
${rows.join("\n")}

## Regression thresholds

\`\`\`json
${JSON.stringify(comparison.thresholds, null, 2)}
\`\`\`

## Violations

${violations}
`;
}

export function readJson(path) {
  return JSON.parse(fs.readFileSync(path, "utf8"));
}
