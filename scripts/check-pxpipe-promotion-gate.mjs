#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

const requiredSignals = [
  {
    label: "pxpipe promotion gate",
    file: "src/lib/pxpipePromotionGate.ts",
    needles: ["evaluatePxpipePromotionGate", "canPromotePxpipeExperimental"],
  },
  {
    label: "pxpipe provenance fixture",
    file: "fixtures/pxpipe-promotion-evidence.json",
    needles: ["text_image", "visualQualityChecklistSigned"],
  },
];

function fail(message) {
  console.error(`pxpipe promotion gate check failed: ${message}`);
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

const fixture = JSON.parse(
  fs.readFileSync(path.join(root, "fixtures/pxpipe-promotion-evidence.json"), "utf8"),
);
if (fixture.visualQualityChecklistSigned !== false) {
  fail("pxpipe promotion gate expects visualQualityChecklistSigned=false until review passes");
}

console.log(
  JSON.stringify(
    {
      ok: true,
      signals: requiredSignals.map((signal) => signal.label),
      headroomCapability: fixture.headroomCapability,
    },
    null,
    2,
  ),
);
