#!/usr/bin/env node
import fs from "node:fs";
import { spawnSync } from "node:child_process";

const summaryPath = "dist/public-release-proof-summary.md";
const jsonPath = "dist/public-release-proof-summary.json";
const releaseReportPath = "dist/release-readiness-report.json";
const generatedAt = new Date().toISOString();

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    timeout: 120_000,
  });
  return {
    command: [command, ...args].join(" "),
    status: result.status,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
  };
}

function readJson(file) {
  if (!fs.existsSync(file)) {
    return null;
  }
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

const reportStep = run("npm", ["run", "release:report"]);
const releaseReport = readJson(releaseReportPath);
const gate = releaseReport?.shareableDmgGate ?? {};
const releaseEnv = releaseReport?.releaseEnv ?? {};
const blockers = [
  gate.signedAndNotarized ? null : "signed/notarized DMG",
  gate.updaterFeedReady ? null : "updater feed",
  gate.staticSmokePreflightReady ? null : "static smoke preflight",
  gate.installedAppSmokeReady ? null : "public installed-app smoke",
  releaseEnv.ok ? null : "release environment",
].filter(Boolean);
const proofReady = blockers.length === 0;

const payload = {
  schemaVersion: 1,
  generatedAt,
  kind: "mac_ai_switchboard.public_release_proof",
  releaseGateEvidence: true,
  proofReady,
  blockers,
  requiredArtifacts: {
    releaseReadinessReport: releaseReportPath,
    installedSmokeSummary: "dist/installed-smoke-summary.md",
      staticSmokeSummary: "dist/smoke-preflight-summary.md",
    signedDmg: "dist/*.dmg with Developer ID signature and notarization ticket",
      updaterFeed: "signed latest.json from configured updater endpoint",
    },
    localOnlyEvidenceExcluded: [
      "dist/local-installed-smoke-summary.md",
      "dist/local-rollback-validation-summary.md",
      "dist/local-doctor-repair-validation-summary.md",
      "dist/local-connector-readiness-summary.md",
      "dist/measured-savings-benchmark.md",
    ],
  shareableDmgGate: gate,
  releaseEnv,
  command: reportStep.command,
  commandStatus: reportStep.status,
  stdoutPreview: reportStep.stdout.slice(0, 4000),
  stderrPreview: reportStep.stderr.slice(0, 4000),
};

fs.mkdirSync("dist", { recursive: true });
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

const markdown = `# Public Release Proof Summary

Generated: ${generatedAt}

- Release gate evidence: yes
- Proof ready: ${proofReady ? "yes" : "no"}
- Required command: \`${reportStep.command}\`
- Release report: ${releaseReportPath}
- Blockers: ${blockers.join(", ") || "none"}

## Required Artifacts

- Release readiness report: \`${payload.requiredArtifacts.releaseReadinessReport}\`
- Installed smoke summary: \`${payload.requiredArtifacts.installedSmokeSummary}\`
- Static smoke summary: \`${payload.requiredArtifacts.staticSmokeSummary}\`
- Signed DMG: \`${payload.requiredArtifacts.signedDmg}\`
- Updater feed: \`${payload.requiredArtifacts.updaterFeed}\`

## Gate Snapshot

- Environment clear: ${gate.environmentClear ? "yes" : "no"}
- Backend validation ready: ${gate.backendValidationReady ? "yes" : "no"}
- Signed and notarized: ${gate.signedAndNotarized ? "yes" : "no"}
- Updater feed ready: ${gate.updaterFeedReady ? "yes" : "no"}
- Static smoke preflight ready: ${gate.staticSmokePreflightReady ? "yes" : "no"}
- Installed app smoke ready: ${gate.installedAppSmokeReady ? "yes" : "no"}

## Local-Only Evidence Excluded

${payload.localOnlyEvidenceExcluded.map((artifact) => `- \`${artifact}\``).join("\n")}
`;

fs.writeFileSync(summaryPath, markdown);
console.log("Public release proof summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);
