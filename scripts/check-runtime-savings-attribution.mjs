#!/usr/bin/env node

import fs from "node:fs";
import { spawnSync } from "node:child_process";

const requiredSources = ["Caveman", "Ponytail", "Markitdown", "CompactChinese"];
const summaryPath = "dist/runtime-savings-attribution-summary.md";
const jsonPath = "dist/runtime-savings-attribution-summary.json";

function fail(message) {
  console.error(`runtime savings attribution check failed: ${message}`);
  process.exit(1);
}

const libSource = fs.readFileSync("src-tauri/src/lib.rs", "utf8");
const dashboardCommandsSource = fs.readFileSync(
  "src-tauri/src/dashboard_commands.rs",
  "utf8",
);
const stateSource = fs.readFileSync("src-tauri/src/state.rs", "utf8");
const modelsSource = fs.readFileSync("src-tauri/src/models.rs", "utf8");

for (const source of requiredSources) {
  if (!modelsSource.includes(source)) {
    fail(`SavingsAttributionSource missing ${source}`);
  }
}
if (!libSource.includes("record_measured_savings_attribution")) {
  fail("Tauri command record_measured_savings_attribution missing");
}
if (!dashboardCommandsSource.includes("record_measured_addon_attribution")) {
  fail("Tauri command does not call record_measured_addon_attribution");
}
if (!stateSource.includes("SavingsAttributionConfidence::Measured")) {
  fail("state does not record measured attribution confidence");
}
if (!stateSource.includes("baseline_tokens.saturating_sub(optimized_tokens)")) {
  fail("state does not compute measured before/after token delta");
}
if (!modelsSource.includes("runtime_event_count")) {
  fail("SavingsAttributionCounter missing runtime_event_count");
}
if (!modelsSource.includes("estimated_event_count")) {
  fail("SavingsAttributionCounter missing estimated_event_count");
}
if (
  !stateSource.includes("entry.runtime_event_count") ||
  !stateSource.includes("event.request_delta as u64")
) {
  fail("state does not aggregate event-backed runtime counter units");
}

const tests = [
  "savings_tracker_appends_measured_headroom_attribution_events",
  "savings_tracker_appends_measured_rtk_attribution_events_from_deltas",
  "addon_attribution_event_records_caveman_and_compact_chinese",
  "addon_attribution_event_records_estimated_ponytail_host_registration",
  "savings_attribution_counters_group_addon_sources",
];

const testResults = tests.map((testName) => {
  const result = spawnSync(
    "cargo",
    ["test", "--manifest-path", "src-tauri/Cargo.toml", testName],
    {
      cwd: process.cwd(),
      encoding: "utf8",
      timeout: 240_000,
      maxBuffer: 16 * 1024 * 1024,
    },
  );
  return {
    id: testName,
    status: result.status,
    ok: result.status === 0,
    stdoutPreview: (result.stdout ?? "").slice(0, 2000),
    stderrPreview: (result.stderr ?? "").slice(0, 2000),
  };
});

const passed = testResults.every((result) => result.ok);
const payload = {
  generatedAt: new Date().toISOString(),
  kind: "mac_ai_switchboard.runtime_savings_attribution_contract",
  releaseGateEvidence: false,
  requiredSources,
  measuredCommandExposed: true,
  measuredConfidenceRecorded: true,
  beforeAfterDeltaRecorded: true,
  runtimeCountersRecorded: true,
  tests: testResults,
  passed,
};

fs.mkdirSync("dist", { recursive: true });
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);
fs.writeFileSync(
  summaryPath,
  `# Runtime Savings Attribution Contract

Generated: ${payload.generatedAt}

- Passed: ${passed ? "yes" : "no"}
- Release gate evidence: no
- Required sources: ${requiredSources.join(", ")}
- Measured command exposed: yes
- Measured confidence recorded: yes
- Before/after token delta recorded: yes
- Runtime counter units recorded: yes
- Tests: ${tests.join(", ")}

\`\`\`
${testResults
  .map(
    (result) =>
      `${result.id}: ${result.ok ? "ok" : `failed status ${result.status}`}`,
  )
  .join("\n")}
\`\`\`
`,
);

if (!passed) {
  fail("targeted Rust savings attribution tests failed");
}

console.log(
  `Runtime savings attribution contract OK (${requiredSources.join(", ")}).`,
);
