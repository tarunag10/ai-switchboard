#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const requiredPaths = [
  "docs/release-truth.json",
  "docs/plan-status-ledger.md",
  "benchmarks/fixtures/model-routing-quality-evidence.json",
  "connectors/manifest.json",
  "connectors/lifecycle-fixtures.json",
  "fixtures/connector-promotion-evidence.json",
  "scripts/check-model-routing-evidence.mjs",
  "scripts/check-release-truth.mjs",
  "scripts/public-release-proof-summary.mjs",
  "scripts/reboot-level-installed-proof-summary.mjs",
  "scripts/check-connector-lifecycle-matrix.mjs",
  "src-tauri/src/optimization/model_routing.rs",
  "src-tauri/src/route_plan.rs",
];
const requiredScripts = [
  "check:implementation-plan-master",
  "check:release-documentation-drift",
  "check:model-routing-evidence",
  "release:report:check",
  "release:proof:check",
  "smoke:reboot-level:local:check",
];

const readJson = (root, relativePath) => JSON.parse(fs.readFileSync(path.join(root, relativePath), "utf8"));

export function checkMasterPlan(root = process.cwd(), planRelativePath = "docs/implementation-plan-master.md") {
  const failures = [];
  const planPath = path.join(root, planRelativePath);
  if (!fs.existsSync(planPath)) return [`missing ${planRelativePath}`];
  const plan = fs.readFileSync(planPath, "utf8");

  try {
    const packageJson = readJson(root, "package.json");
    for (const script of requiredScripts) {
      if (typeof packageJson.scripts?.[script] !== "string") failures.push(`missing operational package script: ${script}`);
    }
  } catch (error) {
    failures.push(`cannot read package.json for operational gates: ${error.message}`);
  }

  for (const relativePath of requiredPaths) {
    if (!fs.existsSync(path.join(root, relativePath))) failures.push(`missing referenced evidence path: ${relativePath}`);
  }
  for (const phrase of [
    "automatic routing stays observe-only until that evidence exists",
    "Cursor",
    "Public installed-app smoke and reboot-level Doctor/Rollback/uninstall proof",
    "Prepared but externally blocked",
    "Remaining build work",
  ]) {
    if (!plan.includes(phrase)) failures.push(`master plan missing required boundary or section: ${phrase}`);
  }

  try {
    const truth = readJson(root, "docs/release-truth.json");
    if (truth.evidence?.publicInstalledAppSmoke !== "unverified") failures.push("release truth publicInstalledAppSmoke must remain unverified");
    if (truth.evidence?.rebootLevelDoctorRollbackUninstall !== "unverified") failures.push("release truth reboot proof must remain unverified");
    if (truth.publicRelease?.status !== "documented") failures.push("release truth public release must remain documented until refreshed");

    const routing = readJson(root, "benchmarks/fixtures/model-routing-quality-evidence.json");
    if (routing.evidenceClass !== "offline_static_fixture" || routing.promotionEligible !== false) {
      failures.push("canonical model-routing fixture must remain offline and observe-only");
    }

    const promotion = readJson(root, "fixtures/connector-promotion-evidence.json");
    if (!promotion.gatedNativeConfigConnectorIds?.includes("cursor")) failures.push("Cursor native writes must remain gated");
    if (!Array.isArray(promotion.requiredSidecarStages) || promotion.requiredSidecarStages.length < 7) {
      failures.push("connector promotion evidence must retain all required sidecar stages");
    }
  } catch (error) {
    failures.push(`cannot read master-plan evidence contract: ${error.message}`);
  }
  return failures;
}

function main() {
  const failures = checkMasterPlan();
  if (failures.length) {
    console.error(failures.join("\n"));
    process.exitCode = 1;
    return;
  }
  console.log(JSON.stringify({ ok: true, requiredPaths: requiredPaths.length, requiredScripts: requiredScripts.length, routing: "observe_only", externalReleaseProof: "blocked_until_fresh_artifacts" }, null, 2));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
