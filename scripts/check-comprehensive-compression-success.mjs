#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const planDir = path.join(root, "docs/world-class-token-savings");

const successSignals = [
  {
    id: "unified-dashboard",
    label: "Single compression dashboard",
    file: "src/lib/compressionDashboard.ts",
    needles: ["buildCompressionDashboardOverview", "compressionTokensSaved"],
  },
  {
    id: "max-compression",
    label: "Max safe compression activation",
    file: "src/lib/maxCompressionActivation.ts",
    needles: ["createMaxCompressionActivationPlan", "filterActivatableOptimizationEngineIds"],
  },
  {
    id: "byok-upstream",
    label: "BYOK upstream profiles",
    file: "src-tauri/src/provider_upstream_profiles.rs",
    needles: ["doctor_byok_openai_compatible_issue", "apply_provider_upstream_env"],
  },
  {
    id: "repo-pack-rtk",
    label: "Repo pack and RTK presets",
    file: "scripts/rtk-presets.mjs",
    needles: ["taskPresets", "export const taskPresets"],
  },
  {
    id: "doctor-playbook",
    label: "Doctor compression repair playbook",
    file: "src/lib/doctorCompressionPlaybook.ts",
    needles: ["buildCompressionPlaybookSummary", "enable_semantic_cache"],
  },
  {
    id: "attribution-rules",
    label: "Attribution rules table",
    file: "src/lib/compressionAttributionRules.ts",
    needles: ["compressionAttributionRules", 'family: "cache"'],
  },
  {
    id: "rollback-off-mode",
    label: "Rollback and Off mode rollup",
    file: "docs/world-class-token-savings/COMPRESSION-ROLLBACK-OFF-MODE.md",
    needles: ["C0", "C5", "Rollback Center"],
  },
];

function fail(message) {
  console.error(`comprehensive compression success check failed: ${message}`);
  process.exit(1);
}

const sliceStatus = JSON.parse(
  fs.readFileSync(path.join(planDir, "slice-status.json"), "utf8"),
);

for (const phaseId of ["C0", "C1", "C2", "C3", "C4", "C5"]) {
  const phase = sliceStatus.phases?.[phaseId];
  if (!phase) {
    fail(`slice-status.json missing phase ${phaseId}`);
  }
  const incomplete = phase.slices.filter((slice) => slice.status !== "done");
  if (incomplete.length > 0) {
    fail(
      `phase ${phaseId} has incomplete slices: ${incomplete.map((slice) => slice.id).join(", ")}`,
    );
  }
}

for (const signal of successSignals) {
  const absolute = path.join(root, signal.file);
  if (!fs.existsSync(absolute)) {
    fail(`missing ${signal.file} for ${signal.id}`);
  }
  const contents = fs.readFileSync(absolute, "utf8");
  for (const needle of signal.needles) {
    if (!contents.includes(needle)) {
      fail(`${signal.label} missing needle ${needle} in ${signal.file}`);
    }
  }
}

const gateScripts = [
  "scripts/check-comprehensive-compression-plan.mjs",
  "scripts/check-chonkify-promotion-gate.mjs",
  "scripts/check-leanctx-promotion-gate.mjs",
  "scripts/check-pxpipe-promotion-gate.mjs",
  "scripts/check-semantic-cache-v2-gate.mjs",
];

for (const script of gateScripts) {
  const result = spawnSync("node", [path.join(root, script)], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    fail(`${script} failed:\n${result.stderr || result.stdout}`);
  }
}

console.log(
  JSON.stringify(
    {
      ok: true,
      program: "comprehensive-token-compression",
      phasesComplete: ["C0", "C1", "C2", "C3", "C4", "C5"],
      successCriteria: successSignals.map((signal) => signal.id),
      gateScripts,
    },
    null,
    2,
  ),
);
