import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const manifest = JSON.parse(fs.readFileSync(path.join(root, "connectors/manifest.json"), "utf8"));
const fixtures = JSON.parse(fs.readFileSync(path.join(root, "connectors/lifecycle-fixtures.json"), "utf8"));
const requiredStages = fixtures.requiredStages;
const fixtureById = new Map(fixtures.connectors.map((connector) => [connector.id, connector]));
const failures = [];

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
  console.log(`connector lifecycle matrix ok: ${managed} managed connectors, stages=${requiredStages.join(",")}`);
}
