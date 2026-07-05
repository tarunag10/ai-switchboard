#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";

const summaryPath = "dist/public-release-proof-summary.md";
const jsonPath = "dist/public-release-proof-summary.json";
const releaseReportPath = "dist/release-readiness-report.json";
const rebootLevelInstalledProofPath = "dist/reboot-level-installed-proof-summary.md";
const generatedAt = new Date().toISOString();
const releaseTag = process.env.MAC_AI_SWITCHBOARD_RELEASE_TAG || "v0.0.0";
const releaseRepo =
  process.env.MAC_AI_SWITCHBOARD_RELEASE_REPO || "tarunag10/ai-switchboard";

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

function parseJsonOutput(result) {
  if (result.status !== 0 || !result.stdout.trim()) {
    return null;
  }
  try {
    return JSON.parse(result.stdout);
  } catch {
    return null;
  }
}

const releaseResult = run("gh", [
  "release",
  "view",
  releaseTag,
  "--repo",
  releaseRepo,
  "--json",
  "tagName,name,isPrerelease,isDraft,publishedAt,url,assets",
]);
const githubRelease = parseJsonOutput(releaseResult);
const signedDmgAsset = githubRelease?.assets?.find(
  (asset) =>
    /signed-notarized-aarch64\.dmg$/.test(asset.name) &&
    asset.contentType === "application/x-apple-diskimage" &&
    asset.state === "uploaded",
);
const checksumAsset = githubRelease?.assets?.find(
  (asset) => signedDmgAsset && asset.name === `${signedDmgAsset.name}.sha256`,
);
const updaterFeedAsset = githubRelease?.assets?.find((asset) => asset.name === "latest.json");
const updaterSignatureAssets =
  githubRelease?.assets?.filter((asset) => /\.sig$/.test(asset.name)) ?? [];

const reportStep = run("npm", ["run", "release:report"]);
const releaseReport = readJson(releaseReportPath);
const gate = releaseReport?.shareableDmgGate ?? {};
const liveSignedDmgReady = Boolean(signedDmgAsset && checksumAsset);
const updaterFeedProofReady = Boolean(
  (updaterFeedAsset && updaterSignatureAssets.length > 0) || gate.updaterFeedReady,
);
const rebootLevelInstalledProofReady = fs.existsSync(rebootLevelInstalledProofPath);
const blockers = [
  signedDmgAsset ? null : "signed/notarized DMG",
  checksumAsset ? null : "public checksum",
  updaterFeedProofReady ? null : "updater feed/signature assets",
  gate.staticSmokePreflightReady ? null : "static smoke preflight",
  gate.installedAppSmokeReady ? null : "public installed-app smoke",
  rebootLevelInstalledProofReady ? null : "reboot-level installed proof",
].filter(Boolean);
const proofReady = blockers.length === 0;

const releaseSnapshot = githubRelease
  ? {
      repo: releaseRepo,
      tagName: githubRelease.tagName,
      name: githubRelease.name,
      url: githubRelease.url,
      publishedAt: githubRelease.publishedAt,
      isDraft: githubRelease.isDraft,
      isPrerelease: githubRelease.isPrerelease,
      signedDmgAsset: signedDmgAsset
        ? {
            name: signedDmgAsset.name,
            url: signedDmgAsset.url,
            size: signedDmgAsset.size,
            digest: signedDmgAsset.digest,
            downloadCount: signedDmgAsset.downloadCount,
          }
        : null,
      checksumAsset: checksumAsset
        ? {
            name: checksumAsset.name,
            url: checksumAsset.url,
            digest: checksumAsset.digest,
          }
        : null,
      updaterFeedAsset: updaterFeedAsset
        ? {
            name: updaterFeedAsset.name,
            url: updaterFeedAsset.url,
            digest: updaterFeedAsset.digest,
          }
        : null,
      updaterSignatureAssets: updaterSignatureAssets.map((asset) => ({
        name: asset.name,
        url: asset.url,
        digest: asset.digest,
      })),
    }
  : null;

const payload = {
  schemaVersion: 1,
  generatedAt,
  kind: "mac_ai_switchboard.public_release_proof",
  releaseGateEvidence: proofReady,
  proofReady,
  blockers,
  githubRelease: releaseSnapshot,
  requiredArtifacts: {
    releaseReadinessReport: releaseReportPath,
    installedSmokeSummary: "dist/installed-smoke-summary.md",
    staticSmokeSummary: "dist/smoke-preflight-summary.md",
    signedDmg:
      signedDmgAsset?.url ??
      "dist/*.dmg with Developer ID signature and notarization ticket",
    updaterFeed:
      updaterFeedAsset?.url ?? "signed latest.json from configured updater endpoint",
    updaterSignatureAssets:
      updaterSignatureAssets.length > 0
        ? updaterSignatureAssets.map((asset) => asset.url)
        : "signed updater .sig assets from the release or configured updater endpoint",
    rebootLevelInstalledProof: rebootLevelInstalledProofPath,
  },
  evidenceReconciliation: {
    completedToday: {
      signedNotarizedDmgAsset: liveSignedDmgReady,
      publicChecksumAsset: Boolean(checksumAsset),
    },
    remainingProof: {
      updaterFeedAndSignatureAssets: !updaterFeedProofReady,
      staticSmokePreflight: !gate.staticSmokePreflightReady,
      publicInstalledAppSmoke: !gate.installedAppSmokeReady,
      rebootLevelInstalledProof: !rebootLevelInstalledProofReady,
    },
    note:
      "Live release metadata can prove the signed/notarized DMG asset and checksum separately from updater feed/signature and reboot-level installed proof.",
  },
  localOnlyEvidenceExcluded: [
    "dist/local-installed-smoke-summary.md",
    "dist/local-rollback-validation-summary.md",
    "dist/local-doctor-repair-validation-summary.md",
    "dist/local-connector-readiness-summary.md",
    "dist/measured-savings-benchmark.md",
  ],
  shareableDmgGate: gate,
  releaseEnv: releaseReport?.releaseEnv ?? {},
  command: reportStep.command,
  commandStatus: reportStep.status,
  releaseCommand: releaseResult.command,
  releaseCommandStatus: releaseResult.status,
  stdoutPreview: reportStep.stdout.slice(0, 4000),
  stderrPreview: reportStep.stderr.slice(0, 4000),
  releaseStdoutPreview: releaseResult.stdout.slice(0, 4000),
  releaseStderrPreview: releaseResult.stderr.slice(0, 4000),
};

fs.mkdirSync("dist", { recursive: true });
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

const markdown = `# Public Release Proof Summary

Generated: ${generatedAt}

- Release gate evidence: ${proofReady ? "yes" : "no"}
- Proof ready: ${proofReady ? "yes" : "no"}
- Required command: \`${reportStep.command}\`
- Live release command: \`${releaseResult.command}\`
- Release report: ${releaseReportPath}
- Blockers: ${blockers.join(", ") || "none"}
- GitHub release: ${githubRelease?.url ?? "missing"}
- Signed/notarized DMG asset: ${
  signedDmgAsset
    ? `${signedDmgAsset.name} (${signedDmgAsset.digest ?? "digest missing"})`
    : "missing"
}
- Public checksum asset: ${checksumAsset ? checksumAsset.name : "missing"}
- Updater feed asset: ${updaterFeedAsset ? updaterFeedAsset.name : "missing"}
- Updater signature assets: ${
  updaterSignatureAssets.length > 0
    ? updaterSignatureAssets.map((asset) => asset.name).join(", ")
    : "missing"
}

## Evidence Reconciliation

- Completed signed/notarized DMG asset proof today: ${liveSignedDmgReady ? "yes" : "no"}
- Completed public checksum proof today: ${checksumAsset ? "yes" : "no"}
- Remaining updater feed/signature proof: ${updaterFeedProofReady ? "no" : "yes"}
- Remaining static smoke preflight proof: ${gate.staticSmokePreflightReady ? "no" : "yes"}
- Remaining public installed-app smoke proof: ${gate.installedAppSmokeReady ? "no" : "yes"}
- Remaining reboot-level installed proof: ${rebootLevelInstalledProofReady ? "no" : "yes"}

## Required Artifacts

- Release readiness report: \`${payload.requiredArtifacts.releaseReadinessReport}\`
- Installed smoke summary: \`${payload.requiredArtifacts.installedSmokeSummary}\`
- Static smoke summary: \`${payload.requiredArtifacts.staticSmokeSummary}\`
- Signed DMG: \`${payload.requiredArtifacts.signedDmg}\`
- Updater feed: \`${payload.requiredArtifacts.updaterFeed}\`
- Updater signature assets: \`${Array.isArray(payload.requiredArtifacts.updaterSignatureAssets) ? payload.requiredArtifacts.updaterSignatureAssets.join(", ") : payload.requiredArtifacts.updaterSignatureAssets}\`
- Reboot-level installed proof: \`${payload.requiredArtifacts.rebootLevelInstalledProof}\`

## Gate Snapshot

- Environment clear: ${gate.environmentClear ? "yes" : "no"}
- Backend validation ready: ${gate.backendValidationReady ? "yes" : "no"}
- Signed/notarized release asset present: ${signedDmgAsset ? "yes" : "no"}
- Updater feed/signature ready: ${updaterFeedProofReady ? "yes" : "no"}
- Static smoke preflight ready: ${gate.staticSmokePreflightReady ? "yes" : "no"}
- Installed app smoke ready: ${gate.installedAppSmokeReady ? "yes" : "no"}
- Reboot-level installed proof ready: ${rebootLevelInstalledProofReady ? "yes" : "no"}

## Local-Only Evidence Excluded

${payload.localOnlyEvidenceExcluded.map((artifact) => `- \`${artifact}\``).join("\n")}
`;

fs.writeFileSync(summaryPath, markdown);
console.log("Public release proof summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);
