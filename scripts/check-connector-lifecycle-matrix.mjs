import fs from "node:fs";
import path from "node:path";
import { canonicalLifecycleStages, lifecycleIntentForTest, lifecycleIntentMarkerFailures, runtimeStageByFixtureStage, validateLifecycleSchema } from "./connector-lifecycle-contract.mjs";

const root = process.cwd();
function readJson(relativePath, label) {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, relativePath), "utf8"));
  } catch {
    console.error(`${label} is not valid JSON`);
    process.exit(1);
  }
}

const manifest = readJson("connectors/manifest.json", "connector manifest");
const fixtures = readJson(
  "connectors/lifecycle-fixtures.json",
  "connector lifecycle fixture",
);
const rootFailures = validateLifecycleSchema(manifest, fixtures);
if (rootFailures.length > 0) {
  console.error(rootFailures.join("\n"));
  process.exit(1);
}
const approvedTestFile = "src-tauri/src/client_adapters_tests.rs";
const approvedTestSource = fs.readFileSync(path.join(root, approvedTestFile), "utf8");
const requiredStages = fixtures.requiredStages;
const fixtureById = new Map(fixtures.connectors.map((connector) => [connector.id, connector]));
const failures = lifecycleIntentMarkerFailures(approvedTestSource);
const evidenceLinks = [];

function isRustTest(name) {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`#\\[test\\][\\s\\S]{0,160}\\bfn\\s+${escaped}\\s*\\(`).test(approvedTestSource);
}

for (const connector of manifest) {
  const fixture = fixtureById.get(connector.id);
  if (!fixture) {
    failures.push(`${connector.id}: missing lifecycle fixture`);
    continue;
  }
  const stages = fixture.stages ?? {};
  for (const stage of requiredStages) {
    const evidence = stages[stage];
    if (connector.support_status === "managed" && typeof evidence !== "string") {
      failures.push(`${connector.id}: managed connector is missing ${stage} evidence`);
    }
    if (typeof evidence === "string") {
      evidenceLinks.push({ connector: connector.id, stage, test: evidence });
      if (!isRustTest(evidence)) {
        failures.push(`${connector.id}: ${stage} evidence '${evidence}' is not a #[test] in ${approvedTestFile}`);
      }
      const intent = lifecycleIntentForTest(approvedTestSource, evidence);
      if (!intent) {
        failures.push(`${connector.id}: ${stage} evidence '${evidence}' is missing a lifecycle-intent marker`);
      } else if (!intent.includes(stage)) {
        failures.push(`${connector.id}: ${stage} evidence '${evidence}' marker does not declare ${stage}`);
      }
    }
  }
}

for (const fixture of fixtures.connectors) {
  if (!manifest.some((connector) => connector.id === fixture.id)) failures.push(`${fixture.id}: fixture has no manifest entry`);
}

if (failures.length) {
  console.error(failures.join("\n"));
  process.exitCode = 1;
} else {
  const managed = manifest.filter((connector) => connector.support_status === "managed").length;
  console.log(JSON.stringify({
    ok: true,
    fixtureVersion: fixtures.version,
    managedConnectors: managed,
    requiredStages,
    runtimeStageByFixtureStage,
    approvedTestFile,
    evidenceLinks,
  }, null, 2));
}
