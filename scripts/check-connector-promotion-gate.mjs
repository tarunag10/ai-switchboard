#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const fixturePath = path.join(root, "fixtures/connector-promotion-evidence.json");

const requiredSignals = [
  {
    label: "connector promotion gate",
    file: "src/lib/connectorPromotionGate.ts",
    needles: ["evaluateConnectorPromotionGate", "canPromoteConnectorPastSidecar"],
  },
  {
    label: "planned connector readiness contract",
    file: "src/lib/plannedConnectors.ts",
    needles: ["getPlannedConnectorReadinessContract", "offCleanupImplemented"],
  },
];

function fail(message) {
  console.error(`connector promotion gate check failed: ${message}`);
  process.exit(1);
}

for (const signal of requiredSignals) {
  const absolute = path.join(root, signal.file);
  if (!fs.existsSync(absolute)) {
    fail(`missing ${signal.file}`);
  }
  const contents = fs.readFileSync(absolute, "utf8");
  for (const needle of signal.needles) {
    if (!contents.includes(needle)) {
      fail(`${signal.label} missing needle ${needle} in ${signal.file}`);
    }
  }
}

if (!fs.existsSync(fixturePath)) {
  fail("missing fixtures/connector-promotion-evidence.json");
}

const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
if (!Array.isArray(fixture.requiredSidecarStages) || fixture.requiredSidecarStages.length < 7) {
  fail("connector promotion fixture must define required sidecar stages");
}
if (!fixture.gatedNativeConnectorIds?.includes("cursor")) {
  fail("connector promotion fixture must keep cursor native writes gated");
}

console.log(
  JSON.stringify(
    {
      ok: true,
      requiredSidecarStages: fixture.requiredSidecarStages.length,
      promotedNativeConnectorIds: fixture.promotedNativeConnectorIds ?? [],
      gatedNativeConnectorIds: fixture.gatedNativeConnectorIds ?? [],
    },
    null,
    2,
  ),
);
