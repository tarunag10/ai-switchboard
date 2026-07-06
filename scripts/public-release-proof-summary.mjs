#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";

const summaryPath = "dist/public-release-proof-summary.md";
const jsonPath = "dist/public-release-proof-summary.json";
const releaseReportPath = "dist/release-readiness-report.json";
const rebootLevelInstalledProofPath = "dist/reboot-level-installed-proof-summary.md";
const rebootLevelInstalledProofJsonPath = "dist/reboot-level-installed-proof-summary.json";
const generatedAt = new Date().toISOString();
const releaseTag = process.env.MAC_AI_SWITCHBOARD_RELEASE_TAG || "v0.0.0";
const releaseRepo =
  process.env.MAC_AI_SWITCHBOARD_RELEASE_REPO || "tarunag10/ai-switchboard";
const defaultUpdaterEndpoint = `https://github.com/${releaseRepo}/releases/latest/download/latest.json`;
const workflowUpdaterEndpointFiles = [
  ".github/workflows/release-macos.yml",
  ".github/workflows/release-macos-staging.yml",
];

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

function parseUpdaterEndpoints(raw) {
  const trimmed = raw?.trim();
  if (!trimmed) {
    return [];
  }
  try {
    const parsed = JSON.parse(trimmed);
    if (Array.isArray(parsed)) {
      return parsed
        .map((value) => String(value).trim())
        .filter((value) => value.startsWith("https://"));
    }
  } catch {
    // Fall through to comma/newline parsing.
  }
  return trimmed
    .split(/[,\n]/)
    .map((value) => value.trim())
    .filter((value) => value.startsWith("https://"));
}

function workflowUpdaterEndpoints() {
  const endpoints = [];
  for (const file of workflowUpdaterEndpointFiles) {
    if (!fs.existsSync(file)) {
      continue;
    }
    const body = fs.readFileSync(file, "utf8");
    const matches = body.matchAll(/^\s*HEADROOM_UPDATER_ENDPOINTS:\s*(.+?)\s*$/gm);
    for (const match of matches) {
      const rawValue = match[1].trim().replace(/^['"]|['"]$/g, "");
      endpoints.push(...parseUpdaterEndpoints(rawValue));
    }
  }
  return [...new Set(endpoints)];
}

function hasUpdaterSignatureMetadata(body) {
  if (!body || typeof body !== "object") {
    return false;
  }
  if (typeof body.signature === "string" && body.signature.trim()) {
    return true;
  }
  const platforms = body.platforms;
  if (!platforms || typeof platforms !== "object") {
    return false;
  }
  return Object.values(platforms).some(
    (platform) =>
      platform &&
      typeof platform === "object" &&
      typeof platform.signature === "string" &&
      platform.signature.trim(),
  );
}

async function probeUpdaterEndpoint(url) {
  try {
    const response = await fetch(url, {
      headers: {
        accept: "application/json, text/plain;q=0.9, */*;q=0.5",
      },
      redirect: "follow",
      signal: AbortSignal.timeout(20_000),
    });
    const text = await response.text();
    let json = null;
    let parseError = null;
    if (text.trim()) {
      try {
        json = JSON.parse(text);
      } catch (err) {
        parseError = err instanceof Error ? err.message : String(err);
      }
    }
    return {
      url,
      ok: response.ok,
      status: response.status,
      finalUrl: response.url,
      contentType: response.headers.get("content-type") ?? null,
      hasJsonBody: Boolean(json),
      hasSignatureMetadata: hasUpdaterSignatureMetadata(json),
      parseError,
      bodyPreview: response.ok ? "" : text.slice(0, 500),
    };
  } catch (err) {
    return {
      url,
      ok: false,
      status: null,
      finalUrl: null,
      contentType: null,
      hasJsonBody: false,
      hasSignatureMetadata: false,
      parseError: err instanceof Error ? err.message : String(err),
      bodyPreview: "",
    };
  }
}

async function buildUpdaterEvidence({
  githubRelease,
  updaterFeedAsset,
  updaterSignatureAssets,
}) {
  const configuredEndpoints = parseUpdaterEndpoints(
    process.env.HEADROOM_UPDATER_ENDPOINTS,
  );
  const workflowConfiguredEndpoints = workflowUpdaterEndpoints();
  const candidateEndpoints =
    configuredEndpoints.length > 0
      ? configuredEndpoints
      : workflowConfiguredEndpoints.length > 0
        ? workflowConfiguredEndpoints
        : [defaultUpdaterEndpoint];
  const checkedEndpoints = await Promise.all(
    candidateEndpoints.map((url) => probeUpdaterEndpoint(url)),
  );
  const feedReleaseAssetReady = Boolean(
    updaterFeedAsset?.url && updaterFeedAsset.state === "uploaded",
  );
  const endpointReady = checkedEndpoints.some((check) => check.ok);
  const endpointWithSignatureMetadata = checkedEndpoints.some(
    (check) => check.ok && check.hasSignatureMetadata,
  );
  const signatureReleaseAssetsReady = updaterSignatureAssets.length > 0;
  const blockers = [
    feedReleaseAssetReady ? null : "updater feed release asset latest.json",
    endpointReady ? null : "updater feed endpoint latest.json",
    signatureReleaseAssetsReady ? null : "updater signature release asset",
    endpointReady && !endpointWithSignatureMetadata
      ? "updater feed signature metadata"
      : null,
  ].filter(Boolean);

  return {
    ready: blockers.length === 0,
    blockers,
    releaseAsset: updaterFeedAsset
      ? {
          name: updaterFeedAsset.name,
          url: updaterFeedAsset.url,
          state: updaterFeedAsset.state,
          digest: updaterFeedAsset.digest,
        }
      : null,
    signatureAssets: updaterSignatureAssets.map((asset) => ({
      name: asset.name,
      url: asset.url,
      state: asset.state,
      digest: asset.digest,
    })),
    configuredEndpoints,
    workflowConfiguredEndpoints,
    checkedEndpoints,
    endpointSource:
      configuredEndpoints.length > 0
        ? "environment"
        : workflowConfiguredEndpoints.length > 0
          ? "workflow"
          : "default",
    defaultEndpointUsed:
      configuredEndpoints.length === 0 && workflowConfiguredEndpoints.length === 0,
    defaultEndpoint: defaultUpdaterEndpoint,
    releaseUrl: githubRelease?.url ?? null,
  };
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
  githubRelease?.assets?.filter((asset) => /\.sig$/.test(asset.name) && asset.state === "uploaded") ?? [];
const updaterEvidence = await buildUpdaterEvidence({
  githubRelease,
  updaterFeedAsset,
  updaterSignatureAssets,
});

const reportStep = run("npm", ["run", "release:report"]);
const releaseReport = readJson(releaseReportPath);
const rebootLevelInstalledProof = readJson(rebootLevelInstalledProofJsonPath);
const gate = releaseReport?.shareableDmgGate ?? {};
const liveSignedDmgReady = Boolean(signedDmgAsset && checksumAsset);
const updaterFeedProofReady = updaterEvidence.ready;
const rebootLevelInstalledProofReady =
  fs.existsSync(rebootLevelInstalledProofPath) &&
  rebootLevelInstalledProof?.kind === "mac_ai_switchboard.reboot_level_installed_proof" &&
  rebootLevelInstalledProof?.proofReady === true &&
  rebootLevelInstalledProof?.releaseGateEvidence === true;
const blockers = [
  signedDmgAsset ? null : "signed/notarized DMG",
  checksumAsset ? null : "public checksum",
  ...updaterEvidence.blockers,
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
      updaterFeedAsset?.url ?? "latest.json release asset and reachable updater endpoint",
    updaterSignatureAssets:
      updaterSignatureAssets.length > 0
        ? updaterSignatureAssets.map((asset) => asset.url)
        : "signed updater .sig assets from the public GitHub release",
    rebootLevelInstalledProof: rebootLevelInstalledProofPath,
  },
  rebootLevelInstalledProof: rebootLevelInstalledProof
    ? {
        path: rebootLevelInstalledProofJsonPath,
        proofReady: rebootLevelInstalledProof.proofReady === true,
        releaseGateEvidence: rebootLevelInstalledProof.releaseGateEvidence === true,
        blockers: rebootLevelInstalledProof.blockers ?? [],
        rebootMarker: rebootLevelInstalledProof.rebootMarker ?? null,
      }
    : null,
  evidenceReconciliation: {
    completedToday: {
      signedNotarizedDmgAsset: liveSignedDmgReady,
      publicChecksumAsset: Boolean(checksumAsset),
    },
    remainingProof: {
      updaterFeedReleaseAssetLatestJson: !updaterEvidence.releaseAsset,
      updaterFeedEndpointLatestJson: !updaterEvidence.checkedEndpoints.some(
        (check) => check.ok,
      ),
      updaterSignatureReleaseAssets: updaterEvidence.signatureAssets.length === 0,
      updaterFeedSignatureMetadata:
        updaterEvidence.checkedEndpoints.some((check) => check.ok) &&
        !updaterEvidence.checkedEndpoints.some(
          (check) => check.ok && check.hasSignatureMetadata,
        ),
      staticSmokePreflight: !gate.staticSmokePreflightReady,
      publicInstalledAppSmoke: !gate.installedAppSmokeReady,
      rebootLevelInstalledProof: !rebootLevelInstalledProofReady,
    },
    note:
      "Live release metadata proves the signed/notarized DMG asset and checksum separately from updater feed assets, updater endpoint reachability, updater signature assets, and reboot-level installed proof.",
  },
  updaterEvidence,
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
- Updater feed endpoint: ${
  updaterEvidence.checkedEndpoints.find((check) => check.ok)?.url ?? "missing"
}
- Updater signature assets: ${
  updaterSignatureAssets.length > 0
    ? updaterSignatureAssets.map((asset) => asset.name).join(", ")
    : "missing"
}
- Updater evidence blockers: ${updaterEvidence.blockers.join(", ") || "none"}

## Evidence Reconciliation

- Completed signed/notarized DMG asset proof today: ${liveSignedDmgReady ? "yes" : "no"}
- Completed public checksum proof today: ${checksumAsset ? "yes" : "no"}
- Remaining updater feed release asset proof: ${updaterEvidence.releaseAsset ? "no" : "yes"}
- Remaining updater feed endpoint proof: ${
  updaterEvidence.checkedEndpoints.some((check) => check.ok) ? "no" : "yes"
}
- Remaining updater signature release-asset proof: ${
  updaterEvidence.signatureAssets.length > 0 ? "no" : "yes"
}
- Remaining updater feed signature-metadata proof: ${
  updaterEvidence.checkedEndpoints.some((check) => check.ok) &&
  !updaterEvidence.checkedEndpoints.some((check) => check.ok && check.hasSignatureMetadata)
    ? "yes"
    : "no"
}
- Remaining static smoke preflight proof: ${gate.staticSmokePreflightReady ? "no" : "yes"}
- Remaining public installed-app smoke proof: ${gate.installedAppSmokeReady ? "no" : "yes"}
- Remaining reboot-level installed proof: ${rebootLevelInstalledProofReady ? "no" : "yes"}

## Updater Evidence

- Default endpoint used: ${updaterEvidence.defaultEndpointUsed ? "yes" : "no"}
- Endpoint source: ${updaterEvidence.endpointSource}
- Default endpoint: \`${updaterEvidence.defaultEndpoint}\`
- Workflow-configured endpoints: ${
  updaterEvidence.workflowConfiguredEndpoints.length > 0
    ? updaterEvidence.workflowConfiguredEndpoints.map((url) => `\`${url}\``).join(", ")
    : "none"
}
- Checked endpoints:
${updaterEvidence.checkedEndpoints
  .map(
    (check) =>
      `  - \`${check.url}\`: ${check.ok ? "ok" : "blocked"}${
        check.status ? ` (${check.status})` : ""
      }${check.parseError ? `; ${check.parseError}` : ""}`,
  )
  .join("\n")}
- Signature metadata in reachable feed: ${
  updaterEvidence.checkedEndpoints.some((check) => check.ok && check.hasSignatureMetadata)
    ? "yes"
    : "no"
}

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
