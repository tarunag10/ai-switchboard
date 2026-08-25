#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import {
  extractConnectorPromotionFrontendContract,
  validateConnectorPromotionConsistency,
} from "./connector-promotion-contract.mjs";

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
const frontend = extractConnectorPromotionFrontendContract(
  fs.readFileSync(path.join(root, "src/lib/plannedConnectors.ts"), "utf8"),
);
for (const error of validateConnectorPromotionConsistency(fixture, frontend)) fail(error);

console.log(
  JSON.stringify(
    {
      ok: true,
      requiredSidecarStages: fixture.requiredSidecarStages.length,
      promotedNativeConnectorIds: fixture.promotedNativeConnectorIds ?? [],
      gatedNativeConfigConnectorIds:
        fixture.gatedNativeConfigConnectorIds ?? [],
    },
    null,
    2,
  ),
);
