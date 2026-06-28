import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const reportPath = "dist/release-readiness-report.md";
const jsonPath = "dist/release-readiness-report.json";
const smokeSummaryPath = "dist/smoke-preflight-summary.md";
const installedSmokeSummaryPath = "dist/installed-smoke-summary.md";
const betaSmokeDoc = "docs/beta-smoke-test.md";
const appPath = "/Applications/Mac AI Switchboard.app";
const appInfoPlistPath = path.join(appPath, "Contents", "Info.plist");
const staticSmokeRequiredEvidence = [
  "Switchboard modes",
  "Switchboard copyable state",
  "Doctor automatic manual triage",
  "Doctor copyable report",
  "Planned connector automation gates",
  "Planned connector manual workflow",
  "Planned connector config creation plan",
  "Repo Intelligence context packs",
  "Savings calculator copyable ledger",
  "Per-tool agent handoffs",
  "Installed app metadata check",
];
const installedSmokeRequiredEvidence = [
  "Switchboard modes and degraded-mode Doctor guidance",
  "Switchboard copyable state",
  "Doctor automatic/manual triage repair actions",
  "Doctor copyable report",
  "Planned connector automation gates, manual workflow, and config creation plan",
  "Repo Intelligence recipes and local context packs",
  "Savings calculator copyable ledger",
  "Per-tool agent handoffs",
  "Codex compression recovery",
];
const plannedConnectorReadinessSummary = [
  "## Planned Connector Readiness",
  "",
  "- Planned connectors: 11",
  "- Automation ready: 0",
  "- Next blocked gates: Backup Implemented (11)",
  "- Gemini CLI: Guide, next gate Backup Implemented",
  "- OpenCode: Adapt, next gate Backup Implemented",
  "- Cursor: Guide, next gate Backup Implemented",
  "- Grok / xAI CLI: Detect, next gate Backup Implemented",
  "- Aider: Adapt, next gate Backup Implemented",
  "- Continue: Guide, next gate Backup Implemented",
  "- Full per-tool dossiers are available from Doctor's connector dossier copy action.",
].join("\n");
function runReleaseEnv() {
  const result = spawnSync(
    process.execPath,
    ["scripts/check-release-env.mjs", "--json"],
    {
      encoding: "utf8",
    },
  );

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    throw new Error(
      `release env preflight failed before JSON output: ${result.stderr || result.stdout}`,
    );
  }

  return JSON.parse(result.stdout);
}

function listItems(items, emptyCopy) {
  if (items.length === 0) {
    return `- ${emptyCopy}`;
  }

  return items.map((item) => `- ${item.label}\n  ${item.hint}`).join("\n");
}

function readSummaryStatus(summaryPath) {
  if (!fs.existsSync(summaryPath)) {
    return {
      present: false,
      generatedLine: null,
      body: "",
    };
  }

  const body = fs.readFileSync(summaryPath, "utf8");
  const firstGeneratedLine =
    body.split("\n").find((line) => line.startsWith("Generated: ")) ?? null;

  return {
    present: true,
    generatedLine: firstGeneratedLine,
    body,
  };
}

function currentFileSha256(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }

  return crypto
    .createHash("sha256")
    .update(fs.readFileSync(filePath))
    .digest("hex");
}

function extractChecklistSha256(body) {
  return (
    body
      .split("\n")
      .find((line) => line.startsWith("- Installed-app checklist SHA-256: "))
      ?.replace("- Installed-app checklist SHA-256: ", "")
      .trim() || null
  );
}

function hasBlocker(releaseEnv, pattern) {
  return releaseEnv.blockers.some((blocker) => pattern.test(blocker.label));
}

function buildBackendValidation(releaseEnv) {
  const cargoAvailable = !hasBlocker(releaseEnv, /missing command: cargo/);
  const rustupAvailable = !hasBlocker(releaseEnv, /missing command: rustup/);
  const ready = cargoAvailable && rustupAvailable;

  return {
    ready,
    cargoAvailable,
    rustupAvailable,
    requiredCommands: ["npm run fmt:desktop", "npm run test:desktop"],
    unblockCommands: [
      "rustup --version",
      "cargo --version",
      "rustup target add aarch64-apple-darwin x86_64-apple-darwin",
      "npm run fmt:desktop",
      "npm run test:desktop",
    ],
    message: ready
      ? "Rust toolchain present. Run desktop formatting and tests before release."
      : "Rust validation cannot run here until cargo and rustup are available.",
  };
}

function buildInstalledSmoke(
  installedAppPresent,
  bundleMetadataPresent,
  installedSmokeSummary,
) {
  const missingEvidence = installedSmokeRequiredEvidence.filter(
    (item) => !installedSmokeSummary.body.includes(item),
  );
  const currentChecklistSha256 = currentFileSha256(betaSmokeDoc);
  const recordedChecklistSha256 = extractChecklistSha256(
    installedSmokeSummary.body,
  );
  const checklistSha256Matches =
    installedSmokeSummary.present &&
    Boolean(recordedChecklistSha256) &&
    recordedChecklistSha256 === currentChecklistSha256;
  const evidenceReady =
    installedSmokeSummary.present &&
    missingEvidence.length === 0 &&
    checklistSha256Matches;
  const ready = installedAppPresent && bundleMetadataPresent && evidenceReady;

  return {
    ready,
    installedAppPresent,
    bundleMetadataPresent,
    appPath,
    appInfoPlistPath,
    smokeSummaryPath: installedSmokeSummaryPath,
    smokeSummaryPresent: installedSmokeSummary.present,
    generatedLine: installedSmokeSummary.generatedLine,
    betaSmokeDoc,
    currentChecklistSha256,
    recordedChecklistSha256,
    checklistSha256Matches,
    requiredEvidence: installedSmokeRequiredEvidence,
    missingEvidence,
    evidenceReady,
    message: ready
      ? "Installed-app smoke summary includes required evidence and matches the current checklist."
      : "Install signed DMG into /Applications, run docs/beta-smoke-test.md, run npm run smoke:installed -- --confirm with required evidence from the current checklist.",
  };
}

function buildStaticSmokePreflight(smokeSummary) {
  const missingEvidence = staticSmokeRequiredEvidence.filter(
    (item) => !smokeSummary.body.includes(item),
  );
  const evidenceReady = smokeSummary.present && missingEvidence.length === 0;

  return {
    ready: evidenceReady,
    smokeSummaryPath,
    smokeSummaryPresent: smokeSummary.present,
    generatedLine: smokeSummary.generatedLine,
    requiredCommand: "npm run smoke:preflight",
    requiredEvidence: staticSmokeRequiredEvidence,
    missingEvidence,
    evidenceReady,
    message: evidenceReady
      ? "Static smoke preflight summary present with every required evidence line. Keep it with release evidence."
      : "Run npm run smoke:preflight before handing a DMG to a tester, and make sure it includes every required evidence line.",
  };
}

function buildShareableDmgGate(
  releaseEnv,
  backendValidation,
  staticSmokePreflight,
  installedSmoke,
) {
  const environmentClear = releaseEnv.blockers.length === 0;
  const signedAndNotarized = environmentClear;
  const updaterFeedReady = !releaseEnv.warnings.some((warning) =>
    /HEADROOM_UPDATER_PUBLIC_KEY|HEADROOM_UPDATER_ENDPOINTS/.test(
      warning.label,
    ),
  );
  const staticSmokePreflightReady = staticSmokePreflight.ready;
  const installedAppSmokeReady = installedSmoke.ready;
  const ready =
    environmentClear &&
    signedAndNotarized &&
    updaterFeedReady &&
    backendValidation.ready &&
    staticSmokePreflightReady &&
    installedAppSmokeReady;

  return {
    ready,
    environmentClear,
    backendValidationReady: backendValidation.ready,
    signedAndNotarized,
    updaterFeedReady,
    staticSmokePreflightReady,
    installedAppSmokeReady,
    message: ready
      ? "All shareable DMG gates are clear."
      : "Do not share a public DMG until every gate is clear.",
  };
}

const releaseEnv = runReleaseEnv();
const smokeSummary = readSummaryStatus(smokeSummaryPath);
const installedSmokeSummary = readSummaryStatus(installedSmokeSummaryPath);
const installedAppPresent = fs.existsSync(appPath);
const bundleMetadataPresent = fs.existsSync(appInfoPlistPath);
const backendValidation = buildBackendValidation(releaseEnv);
const staticSmokePreflight = buildStaticSmokePreflight(smokeSummary);
const installedSmoke = buildInstalledSmoke(
  installedAppPresent,
  bundleMetadataPresent,
  installedSmokeSummary,
);
const shareableDmgGate = buildShareableDmgGate(
  releaseEnv,
  backendValidation,
  staticSmokePreflight,
  installedSmoke,
);
const generatedAt = new Date().toISOString();
const status =
  releaseEnv.ok &&
  backendValidation.ready &&
  staticSmokePreflight.ready &&
  installedSmoke.ready &&
  shareableDmgGate.ready
    ? "ready"
    : "blocked";

const payload = {
  generatedAt,
  status,
  installedAppPresent,
  appPath,
  smokeSummary,
  installedSmokeSummary,
  backendValidation,
  staticSmokePreflight,
  installedSmoke,
  shareableDmgGate,
  releaseEnv,
};

const report = `# Release Readiness Report

Generated: ${generatedAt}

Status: ${status}

## Environment Blockers

${listItems(releaseEnv.blockers, "None. Release environment blockers are clear.")}

## Environment Warnings

${listItems(releaseEnv.warnings, "None. Recommended release settings are present.")}

## Backend Validation

- Rust toolchain ready: ${backendValidation.ready ? "yes" : "no"}
- cargo available: ${backendValidation.cargoAvailable ? "yes" : "no"}
- rustup available: ${backendValidation.rustupAvailable ? "yes" : "no"}
- Required commands: ${backendValidation.requiredCommands.join(", ")}
- Rust unblock commands: ${backendValidation.unblockCommands.join(" -> ")}
- ${backendValidation.message}

## Static Smoke Preflight

- Preflight summary present: ${staticSmokePreflight.smokeSummaryPresent ? "yes" : "no"} (${staticSmokePreflight.smokeSummaryPath})
${staticSmokePreflight.generatedLine ? `- ${staticSmokePreflight.generatedLine}` : "- Smoke preflight summary has not been generated in this checkout."}
- Required command: ${staticSmokePreflight.requiredCommand}
- Required evidence: ${staticSmokePreflight.requiredEvidence.join(", ")}
- Missing evidence: ${staticSmokePreflight.missingEvidence.length ? staticSmokePreflight.missingEvidence.join(", ") : "none"}
- Static smoke evidence ready: ${staticSmokePreflight.evidenceReady ? "yes" : "no"}
- ${staticSmokePreflight.message}

## Installed App Smoke

- Installed app present: ${installedSmoke.installedAppPresent ? "yes" : "no"} (${installedSmoke.appPath})
- Installed app metadata present: ${installedSmoke.bundleMetadataPresent ? "yes" : "no"} (${installedSmoke.appInfoPlistPath})
- Installed smoke summary present: ${installedSmoke.smokeSummaryPresent ? "yes" : "no"} (${installedSmoke.smokeSummaryPath})
${installedSmoke.generatedLine ? `- ${installedSmoke.generatedLine}` : "- Installed smoke summary has not been generated in this checkout."}
- Installed-app checklist: ${installedSmoke.betaSmokeDoc}
- Installed-app checklist hash matches current checklist: ${installedSmoke.checklistSha256Matches ? "yes" : "no"}
- Recorded checklist SHA-256: ${installedSmoke.recordedChecklistSha256 ?? "missing"}
- Current checklist SHA-256: ${installedSmoke.currentChecklistSha256 ?? "missing"}
- Required evidence: ${installedSmoke.requiredEvidence.join(", ")}
- Missing evidence: ${installedSmoke.missingEvidence.length ? installedSmoke.missingEvidence.join(", ") : "none"}
- Installed smoke evidence ready: ${installedSmoke.evidenceReady ? "yes" : "no"}
- ${installedSmoke.message}

## Shareable DMG Gates

- Environment clear: ${shareableDmgGate.environmentClear ? "yes" : "no"}
- Rust backend validation ready: ${shareableDmgGate.backendValidationReady ? "yes" : "no"}
- Signed and notarized: ${shareableDmgGate.signedAndNotarized ? "yes" : "no"}
- Updater feed ready: ${shareableDmgGate.updaterFeedReady ? "yes" : "no"}
- Static smoke preflight ready: ${shareableDmgGate.staticSmokePreflightReady ? "yes" : "no"}
- Installed-app smoke ready: ${shareableDmgGate.installedAppSmokeReady ? "yes" : "no"}
- ${shareableDmgGate.message}

${plannedConnectorReadinessSummary}

## Next Steps

${
  releaseEnv.blockers.length > 0
    ? "- Resolve environment blockers, then rerun `npm run release:report`."
    : "- Environment preflight is clear."
}
${
  installedAppPresent
    ? "- Run `docs/beta-smoke-test.md` against the installed app."
    : "- Build and install the signed DMG, run `docs/beta-smoke-test.md`, then run `npm run smoke:installed -- --confirm`."
}
${backendValidation.ready ? "- Run `npm run fmt:desktop` and `npm run test:desktop` on release Mac." : "- Install Rust with rustup so `npm run fmt:desktop` and `npm run test:desktop` can run."}
- Before publishing, run \`npm run release:check\`.
`;

fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, report);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log(`Release readiness status: ${status}`);
console.log(`Report written: ${reportPath}`);
console.log(`JSON written: ${jsonPath}`);
console.log(
  `Shareable DMG gate: ${shareableDmgGate.ready ? "ready" : "blocked"}`,
);

if (releaseEnv.blockers.length > 0) {
  console.log(`Blockers: ${releaseEnv.blockers.length}`);
}

if (!installedAppPresent) {
  console.log(`Installed app missing: ${appPath}`);
}
if (!backendValidation.ready) {
  console.log("Backend validation pending: cargo/rustup unavailable.");
}
