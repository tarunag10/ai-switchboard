import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const manifest = JSON.parse(fs.readFileSync(path.join(root, "connectors/manifest.json"), "utf8"));
const fixtures = JSON.parse(fs.readFileSync(path.join(root, "connectors/lifecycle-fixtures.json"), "utf8"));
const approvedTestFile = "src-tauri/src/client_adapters_tests.rs";
const approvedTestSource = fs.readFileSync(path.join(root, approvedTestFile), "utf8");
const requiredStages = fixtures.requiredStages;
const fixtureById = new Map(fixtures.connectors.map((connector) => [connector.id, connector]));
const failures = [];
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
    if (connector.support_status !== "managed" && evidence !== null && typeof evidence !== "string") {
      failures.push(`${connector.id}: ${stage} must be a string or explicit null`);
    }
    if (typeof evidence === "string") {
      evidenceLinks.push({ connector: connector.id, stage, test: evidence });
      if (!isRustTest(evidence)) {
        failures.push(`${connector.id}: ${stage} evidence '${evidence}' is not a #[test] in ${approvedTestFile}`);
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
    managedConnectors: managed,
    requiredStages,
    approvedTestFile,
    evidenceLinks,
  }, null, 2));
}
