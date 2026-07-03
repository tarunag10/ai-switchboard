import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const reportPath = "dist/release-readiness-report.md";
const jsonPath = "dist/release-readiness-report.json";
const smokeSummaryPath = "dist/smoke-preflight-summary.md";
const installedSmokeSummaryPath = "dist/installed-smoke-summary.md";
const localInstalledSmokeSummaryPath = "dist/local-installed-smoke-summary.md";
const localInstalledSmokeJsonPath = "dist/local-installed-smoke-summary.json";
const localModeRelaunchSummaryPath =
  "dist/local-mode-relaunch-smoke-summary.md";
const localModeRelaunchJsonPath =
  "dist/local-mode-relaunch-smoke-summary.json";
const localRollbackSummaryPath = "dist/local-rollback-validation-summary.md";
const localRollbackJsonPath = "dist/local-rollback-validation-summary.json";
const localDoctorRepairSummaryPath =
  "dist/local-doctor-repair-validation-summary.md";
const localDoctorRepairJsonPath =
  "dist/local-doctor-repair-validation-summary.json";
const localUninstallSummaryPath = "dist/local-uninstall-validation-summary.md";
const localUninstallJsonPath = "dist/local-uninstall-validation-summary.json";
const localRepoIntelligenceSummaryPath =
  "dist/local-repo-intelligence-validation-summary.md";
const localRepoIntelligenceJsonPath =
  "dist/local-repo-intelligence-validation-summary.json";
const localRepoMemoryMcpSummaryPath =
  "dist/local-repo-memory-mcp-validation-summary.md";
const localRepoMemoryMcpJsonPath =
  "dist/local-repo-memory-mcp-validation-summary.json";
const localOnlyNetworkSummaryPath =
  "dist/local-only-network-validation-summary.md";
const localOnlyNetworkJsonPath =
  "dist/local-only-network-validation-summary.json";
const betaSmokeDoc = "docs/beta-smoke-test.md";
const appPath = "/Applications/Mac AI Switchboard.app";
const appInfoPlistPath = path.join(appPath, "Contents", "Info.plist");
const staticSmokeRequiredEvidence = [
  "Switchboard modes",
  "Switchboard copyable state",
  "Doctor automatic manual triage",
  "Doctor copyable report",
  "Managed connector automation gates",
  "Managed connector native config gate",
  "Managed connector config creation plan",
  "Managed connector readiness evidence",
  "Repo Intelligence context packs",
  "Savings calculator copyable ledger",
  "Per-tool agent handoffs",
  "Connector readiness payload in agent handoffs",
  "Installed app metadata check",
];
const installedSmokeRequiredEvidence = [
  "Switchboard modes and degraded-mode Doctor guidance",
  "Switchboard copyable state",
  "Doctor automatic/manual triage repair actions",
  "Doctor copyable report",
  "Managed connector automation gates, manual workflow, config creation plan, and managed connector readiness evidence",
  "Repo Intelligence recipes and local context packs",
  "Savings calculator copyable ledger",
  "Per-tool agent handoffs",
  "Connector readiness payload in agent handoffs",
  "Codex compression recovery",
];
const connectorManifestPath = "connectors/manifest.json";

function buildManagedConnectorReadiness() {
  const manifest = JSON.parse(fs.readFileSync(connectorManifestPath, "utf8"));
  const connectors = Array.isArray(manifest) ? manifest : manifest.connectors;
  if (!Array.isArray(connectors)) {
    throw new Error(`${connectorManifestPath} must contain a connector array`);
  }

  const managed = connectors
    .filter((connector) => connector.support_status === "managed")
    .map((connector) => ({
      id: connector.id,
      name: connector.name,
      category: connector.category,
      configLocations: connector.config?.locations ?? [],
      gates: connector.automation_gates ?? [],
    }));
  const gated = connectors.filter(
    (connector) => connector.support_status !== "managed",
  );

  return {
    manifestPath: connectorManifestPath,
    managedCount: managed.length,
    gatedCount: gated.length,
    managed,
  };
}

function renderManagedConnectorReadiness(readiness) {
  const managedRows = readiness.managed.map(
    (connector) =>
      `- ${connector.name}: ${connector.category}, ${connector.configLocations.length} config surface${connector.configLocations.length === 1 ? "" : "s"}, ${connector.gates.length} automation gate${connector.gates.length === 1 ? "" : "s"}`,
  );

  return [
    "## Managed Connector Readiness",
    "",
    `- Source: ${readiness.manifestPath}`,
    `- Managed connectors: ${readiness.managedCount}`,
    `- Gated/guided connectors retained: ${readiness.gatedCount}`,
    ...managedRows,
    "- Full per-tool dossiers are available from Doctor's connector dossier copy action.",
  ].join("\n");
}
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

function readJsonStatus(jsonPath) {
  if (!fs.existsSync(jsonPath)) {
    return {
      present: false,
      body: null,
      parseError: null,
    };
  }

  try {
    return {
      present: true,
      body: JSON.parse(fs.readFileSync(jsonPath, "utf8")),
      parseError: null,
    };
  } catch (error) {
    return {
      present: true,
      body: null,
      parseError: error.message,
    };
  }
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

function buildLocalValidationEvidence() {
  const localInstalledSummary = readSummaryStatus(localInstalledSmokeSummaryPath);
  const localInstalledJson = readJsonStatus(localInstalledSmokeJsonPath);
  const modeRelaunchSummary = readSummaryStatus(localModeRelaunchSummaryPath);
  const modeRelaunchJson = readJsonStatus(localModeRelaunchJsonPath);
  const rollbackSummary = readSummaryStatus(localRollbackSummaryPath);
  const rollbackJson = readJsonStatus(localRollbackJsonPath);
  const doctorSummary = readSummaryStatus(localDoctorRepairSummaryPath);
  const doctorJson = readJsonStatus(localDoctorRepairJsonPath);
  const uninstallSummary = readSummaryStatus(localUninstallSummaryPath);
  const uninstallJson = readJsonStatus(localUninstallJsonPath);
  const repoIntelligenceSummary = readSummaryStatus(
    localRepoIntelligenceSummaryPath,
  );
  const repoIntelligenceJson = readJsonStatus(localRepoIntelligenceJsonPath);
  const repoMemoryMcpSummary = readSummaryStatus(localRepoMemoryMcpSummaryPath);
  const repoMemoryMcpJson = readJsonStatus(localRepoMemoryMcpJsonPath);
  const localOnlyNetworkSummary = readSummaryStatus(localOnlyNetworkSummaryPath);
  const localOnlyNetworkJson = readJsonStatus(localOnlyNetworkJsonPath);
  const modeRelaunchPassed = modeRelaunchJson.body?.passed === true;
  const rollbackPassed = rollbackJson.body?.passed === true;
  const doctorRepairPassed = doctorJson.body?.passed === true;
  const uninstallPassed = uninstallJson.body?.passed === true;
  const repoIntelligencePassed = repoIntelligenceJson.body?.passed === true;
  const repoMemoryMcpPassed = repoMemoryMcpJson.body?.passed === true;
  const localOnlyNetworkPassed = localOnlyNetworkJson.body?.passed === true;
  const localInstalledAppPresent = localInstalledJson.body?.app?.present === true;
  const localInstalledMetadataMatches =
    localInstalledJson.body?.app?.metadataMatches === true;
  const localInstalledDmgVerified =
    localInstalledJson.body?.dmg?.hdiutilVerifyOk === true;
  const localInstalledCodesignVerified =
    localInstalledJson.body?.signing?.codesignVerifyOk === true;
  const localInstalledRuntimeChecked =
    localInstalledJson.body?.runtimeHealth?.checked === true;
  const localInstalledAppListenerReady =
    localInstalledJson.body?.runtimeHealth?.appListener?.ready === true;
  const localInstalledEngineProxyReady =
    localInstalledJson.body?.runtimeHealth?.engineProxy?.ready === true;
  const localInstalledPassed =
    localInstalledAppPresent &&
    localInstalledMetadataMatches &&
    localInstalledDmgVerified &&
    localInstalledCodesignVerified;
  const ready =
    localInstalledPassed &&
    modeRelaunchPassed &&
    rollbackPassed &&
    doctorRepairPassed &&
    uninstallPassed &&
    repoIntelligencePassed &&
    repoMemoryMcpPassed &&
    localOnlyNetworkPassed;

  return {
    ready,
    releaseGateEvidence: false,
    localInstalled: {
      summaryPath: localInstalledSmokeSummaryPath,
      jsonPath: localInstalledSmokeJsonPath,
      summaryPresent: localInstalledSummary.present,
      jsonPresent: localInstalledJson.present,
      generatedLine: localInstalledSummary.generatedLine,
      passed: localInstalledPassed,
      parseError: localInstalledJson.parseError,
      kind: localInstalledJson.body?.kind ?? null,
      appPresent: localInstalledAppPresent,
      metadataMatches: localInstalledMetadataMatches,
      dmgVerified: localInstalledDmgVerified,
      codesignVerified: localInstalledCodesignVerified,
      runtimeHealthChecked: localInstalledRuntimeChecked,
      appListenerReady: localInstalledAppListenerReady,
      engineProxyReady: localInstalledEngineProxyReady,
      requiredCommand: "npm run smoke:installed:local",
    },
    modeRelaunch: {
      summaryPath: localModeRelaunchSummaryPath,
      jsonPath: localModeRelaunchJsonPath,
      summaryPresent: modeRelaunchSummary.present,
      jsonPresent: modeRelaunchJson.present,
      generatedLine: modeRelaunchSummary.generatedLine,
      passed: modeRelaunchPassed,
      parseError: modeRelaunchJson.parseError,
      kind: modeRelaunchJson.body?.kind ?? null,
      modeCount: Array.isArray(modeRelaunchJson.body?.modes)
        ? modeRelaunchJson.body.modes.length
        : 0,
      offModeProxyDown:
        modeRelaunchJson.body?.modes?.find((mode) => mode.mode === "off")
          ?.proxyListening === false,
      rtkModeProxyDown:
        modeRelaunchJson.body?.modes?.find((mode) => mode.mode === "rtk")
          ?.proxyListening === false,
      restored: modeRelaunchJson.body?.restored === true,
      requiredCommand: "npm run smoke:mode-relaunch:local -- --confirm",
    },
    rollback: {
      summaryPath: localRollbackSummaryPath,
      jsonPath: localRollbackJsonPath,
      summaryPresent: rollbackSummary.present,
      jsonPresent: rollbackJson.present,
      generatedLine: rollbackSummary.generatedLine,
      passed: rollbackPassed,
      parseError: rollbackJson.parseError,
      kind: rollbackJson.body?.kind ?? null,
      stepCount: Array.isArray(rollbackJson.body?.steps)
        ? rollbackJson.body.steps.length
        : 0,
      relaunchSurvivalEvidence:
        rollbackJson.body?.relaunchSurvivalEvidence ?? null,
      requiredCommand: "npm run smoke:rollback:local",
    },
    doctorRepair: {
      summaryPath: localDoctorRepairSummaryPath,
      jsonPath: localDoctorRepairJsonPath,
      summaryPresent: doctorSummary.present,
      jsonPresent: doctorJson.present,
      generatedLine: doctorSummary.generatedLine,
      passed: doctorRepairPassed,
      parseError: doctorJson.parseError,
      kind: doctorJson.body?.kind ?? null,
      stepCount: Array.isArray(doctorJson.body?.steps)
        ? doctorJson.body.steps.length
        : 0,
      requiredCommand: "npm run smoke:doctor-repair:local",
    },
    uninstall: {
      summaryPath: localUninstallSummaryPath,
      jsonPath: localUninstallJsonPath,
      summaryPresent: uninstallSummary.present,
      jsonPresent: uninstallJson.present,
      generatedLine: uninstallSummary.generatedLine,
      passed: uninstallPassed,
      destructive: uninstallJson.body?.destructive === true,
      parseError: uninstallJson.parseError,
      kind: uninstallJson.body?.kind ?? null,
      stepCount: Array.isArray(uninstallJson.body?.steps)
        ? uninstallJson.body.steps.length
        : 0,
      requiredCommand: "npm run smoke:uninstall:local",
    },
    repoIntelligence: {
      summaryPath: localRepoIntelligenceSummaryPath,
      jsonPath: localRepoIntelligenceJsonPath,
      summaryPresent: repoIntelligenceSummary.present,
      jsonPresent: repoIntelligenceJson.present,
      generatedLine: repoIntelligenceSummary.generatedLine,
      passed: repoIntelligencePassed,
      readOnly: repoIntelligenceJson.body?.readOnly === true,
      modifiesRepository: repoIntelligenceJson.body?.modifiesRepository === true,
      parseError: repoIntelligenceJson.parseError,
      kind: repoIntelligenceJson.body?.kind ?? null,
      stepCount: Array.isArray(repoIntelligenceJson.body?.steps)
        ? repoIntelligenceJson.body.steps.length
        : 0,
      requiredCommand: "npm run smoke:repo-intelligence:local",
    },
    repoMemoryMcp: {
      summaryPath: localRepoMemoryMcpSummaryPath,
      jsonPath: localRepoMemoryMcpJsonPath,
      summaryPresent: repoMemoryMcpSummary.present,
      jsonPresent: repoMemoryMcpJson.present,
      generatedLine: repoMemoryMcpSummary.generatedLine,
      passed: repoMemoryMcpPassed,
      readOnly: repoMemoryMcpJson.body?.readOnly === true,
      modifiesRepository: repoMemoryMcpJson.body?.modifiesRepository === true,
      relaunchSurvivalEvidence:
        repoMemoryMcpJson.body?.relaunchSurvivalEvidence ?? null,
      connectorBridgeRecipesVerified:
        repoMemoryMcpJson.body?.connectorBridgeRecipesVerified === true,
      expectedToolsPresent:
        repoMemoryMcpJson.body?.expectedToolsPresent === true,
      parseError: repoMemoryMcpJson.parseError,
      kind: repoMemoryMcpJson.body?.kind ?? null,
      toolCount: Number.isFinite(repoMemoryMcpJson.body?.toolCount)
        ? repoMemoryMcpJson.body.toolCount
        : 0,
      stepCount: Array.isArray(repoMemoryMcpJson.body?.steps)
        ? repoMemoryMcpJson.body.steps.length
        : 0,
      requiredCommand: "npm run smoke:repo-memory-mcp:local",
    },
    localOnlyNetwork: {
      summaryPath: localOnlyNetworkSummaryPath,
      jsonPath: localOnlyNetworkJsonPath,
      summaryPresent: localOnlyNetworkSummary.present,
      jsonPresent: localOnlyNetworkJson.present,
      generatedLine: localOnlyNetworkSummary.generatedLine,
      passed: localOnlyNetworkPassed,
      localOnly: localOnlyNetworkJson.body?.localOnly === true,
      appOwnedRemoteCallsBlocked:
        localOnlyNetworkJson.body?.appOwnedRemoteCallsBlocked === true,
      coverage: localOnlyNetworkJson.body?.coverage ?? null,
      parseError: localOnlyNetworkJson.parseError,
      kind: localOnlyNetworkJson.body?.kind ?? null,
      stepCount: Array.isArray(localOnlyNetworkJson.body?.steps)
        ? localOnlyNetworkJson.body.steps.length
        : 0,
      requiredCommand: "npm run smoke:local-only:local",
    },
    message: ready
      ? "Local installed smoke, mode relaunch, Doctor repair, Rollback Center, uninstall dry-run, Repo Intelligence, Repo Memory MCP, and local-only network validation summaries passed. This is local-only evidence and does not replace signed installed-app smoke."
      : "Run npm run smoke:installed:local, npm run smoke:mode-relaunch:local -- --confirm, npm run smoke:rollback:local, npm run smoke:doctor-repair:local, npm run smoke:uninstall:local, npm run smoke:repo-intelligence:local, npm run smoke:repo-memory-mcp:local, and npm run smoke:local-only:local to refresh local install, relaunch, survival, cleanup, repo-context, MCP bridge, and local-only network evidence before public installed-smoke proof.",
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
  const publicDmgBlockers = [
    signedAndNotarized ? null : "signed/notarized DMG",
    updaterFeedReady ? null : "updater feed",
    staticSmokePreflightReady ? null : "static smoke preflight",
    installedAppSmokeReady ? null : "public installed-app smoke",
  ].filter(Boolean);

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
      : `Public DMG blocked until ${publicDmgBlockers.join(", ")} evidence is ready.`,
  };
}

function missingLocalEvidenceLabels(localValidation) {
  return [
    localValidation.localInstalled?.passed ? null : "local installed smoke",
    localValidation.modeRelaunch?.passed ? null : "Off/RTK relaunch smoke",
    localValidation.rollback?.passed ? null : "Rollback Center validation",
    localValidation.rollback?.relaunchSurvivalEvidence
      ? null
      : "Rollback Center relaunch survival evidence",
    localValidation.doctorRepair?.passed ? null : "Doctor repair validation",
    localValidation.uninstall?.passed ? null : "uninstall dry-run validation",
    localValidation.repoIntelligence?.passed ? null : "Repo Intelligence validation",
    localValidation.repoMemoryMcp?.passed ? null : "Repo Memory MCP validation",
    localValidation.localOnlyNetwork?.passed ? null : "local-only network validation",
  ].filter(Boolean);
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
const localValidation = buildLocalValidationEvidence();
const shareableDmgGate = buildShareableDmgGate(
  releaseEnv,
  backendValidation,
  staticSmokePreflight,
  installedSmoke,
);
const managedConnectorReadiness = buildManagedConnectorReadiness();
const managedConnectorReadinessSummary = renderManagedConnectorReadiness(
  managedConnectorReadiness,
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
  localValidation,
  shareableDmgGate,
  managedConnectorReadiness,
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

## Local Doctor and Rollback Validation

- Release gate evidence: ${localValidation.releaseGateEvidence ? "yes" : "no"}
- Local validation ready: ${localValidation.ready ? "yes" : "no"}
- Local installed smoke summary present: ${localValidation.localInstalled.summaryPresent ? "yes" : "no"} (${localValidation.localInstalled.summaryPath})
- Local installed smoke JSON present: ${localValidation.localInstalled.jsonPresent ? "yes" : "no"} (${localValidation.localInstalled.jsonPath})
${localValidation.localInstalled.generatedLine ? `- ${localValidation.localInstalled.generatedLine}` : "- Local installed smoke summary has not been generated in this checkout."}
- Local installed validation passed: ${localValidation.localInstalled.passed ? "yes" : "no"}
- Local installed app present: ${localValidation.localInstalled.appPresent ? "yes" : "no"}
- Local installed metadata matches: ${localValidation.localInstalled.metadataMatches ? "yes" : "no"}
- Local installed DMG verified: ${localValidation.localInstalled.dmgVerified ? "yes" : "no"}
- Local installed codesign verified: ${localValidation.localInstalled.codesignVerified ? "yes" : "no"}
- Local installed runtime health checked: ${localValidation.localInstalled.runtimeHealthChecked ? "yes" : "no"}
- Local installed app listener ready: ${localValidation.localInstalled.appListenerReady ? "yes" : "no"}
- Local installed Headroom engine proxy ready: ${localValidation.localInstalled.engineProxyReady ? "yes" : "no"}
- Local installed command: ${localValidation.localInstalled.requiredCommand}
- Mode relaunch summary present: ${localValidation.modeRelaunch.summaryPresent ? "yes" : "no"} (${localValidation.modeRelaunch.summaryPath})
- Mode relaunch JSON present: ${localValidation.modeRelaunch.jsonPresent ? "yes" : "no"} (${localValidation.modeRelaunch.jsonPath})
${localValidation.modeRelaunch.generatedLine ? `- ${localValidation.modeRelaunch.generatedLine}` : "- Mode relaunch smoke summary has not been generated in this checkout."}
- Mode relaunch validation passed: ${localValidation.modeRelaunch.passed ? "yes" : "no"}
- Mode relaunch checked modes: ${localValidation.modeRelaunch.modeCount}
- Mode relaunch Off proxy down: ${localValidation.modeRelaunch.offModeProxyDown ? "yes" : "unknown"}
- Mode relaunch RTK proxy down: ${localValidation.modeRelaunch.rtkModeProxyDown ? "yes" : "unknown"}
- Mode relaunch config restored: ${localValidation.modeRelaunch.restored ? "yes" : "unknown"}
- Mode relaunch command: ${localValidation.modeRelaunch.requiredCommand}
- Rollback summary present: ${localValidation.rollback.summaryPresent ? "yes" : "no"} (${localValidation.rollback.summaryPath})
- Rollback JSON present: ${localValidation.rollback.jsonPresent ? "yes" : "no"} (${localValidation.rollback.jsonPath})
${localValidation.rollback.generatedLine ? `- ${localValidation.rollback.generatedLine}` : "- Rollback validation summary has not been generated in this checkout."}
- Rollback validation passed: ${localValidation.rollback.passed ? "yes" : "no"}
- Rollback validation steps: ${localValidation.rollback.stepCount}
- Rollback relaunch survival evidence: ${localValidation.rollback.relaunchSurvivalEvidence ?? "missing"}
- Rollback command: ${localValidation.rollback.requiredCommand}
- Doctor repair summary present: ${localValidation.doctorRepair.summaryPresent ? "yes" : "no"} (${localValidation.doctorRepair.summaryPath})
- Doctor repair JSON present: ${localValidation.doctorRepair.jsonPresent ? "yes" : "no"} (${localValidation.doctorRepair.jsonPath})
${localValidation.doctorRepair.generatedLine ? `- ${localValidation.doctorRepair.generatedLine}` : "- Doctor repair validation summary has not been generated in this checkout."}
- Doctor repair validation passed: ${localValidation.doctorRepair.passed ? "yes" : "no"}
- Doctor repair validation steps: ${localValidation.doctorRepair.stepCount}
- Doctor repair command: ${localValidation.doctorRepair.requiredCommand}
- Uninstall summary present: ${localValidation.uninstall.summaryPresent ? "yes" : "no"} (${localValidation.uninstall.summaryPath})
- Uninstall JSON present: ${localValidation.uninstall.jsonPresent ? "yes" : "no"} (${localValidation.uninstall.jsonPath})
${localValidation.uninstall.generatedLine ? `- ${localValidation.uninstall.generatedLine}` : "- Uninstall validation summary has not been generated in this checkout."}
- Uninstall validation passed: ${localValidation.uninstall.passed ? "yes" : "no"}
- Uninstall validation destructive: ${localValidation.uninstall.destructive === false ? "no" : "unknown"}
- Uninstall validation steps: ${localValidation.uninstall.stepCount}
- Uninstall command: ${localValidation.uninstall.requiredCommand}
- Repo Intelligence summary present: ${localValidation.repoIntelligence.summaryPresent ? "yes" : "no"} (${localValidation.repoIntelligence.summaryPath})
- Repo Intelligence JSON present: ${localValidation.repoIntelligence.jsonPresent ? "yes" : "no"} (${localValidation.repoIntelligence.jsonPath})
${localValidation.repoIntelligence.generatedLine ? `- ${localValidation.repoIntelligence.generatedLine}` : "- Repo Intelligence validation summary has not been generated in this checkout."}
- Repo Intelligence validation passed: ${localValidation.repoIntelligence.passed ? "yes" : "no"}
- Repo Intelligence validation read-only: ${localValidation.repoIntelligence.readOnly === true ? "yes" : "unknown"}
- Repo Intelligence modifies repository: ${localValidation.repoIntelligence.modifiesRepository === false ? "no" : "unknown"}
- Repo Intelligence validation steps: ${localValidation.repoIntelligence.stepCount}
- Repo Intelligence command: ${localValidation.repoIntelligence.requiredCommand}
- Repo Memory MCP summary present: ${localValidation.repoMemoryMcp.summaryPresent ? "yes" : "no"} (${localValidation.repoMemoryMcp.summaryPath})
- Repo Memory MCP JSON present: ${localValidation.repoMemoryMcp.jsonPresent ? "yes" : "no"} (${localValidation.repoMemoryMcp.jsonPath})
${localValidation.repoMemoryMcp.generatedLine ? `- ${localValidation.repoMemoryMcp.generatedLine}` : "- Repo Memory MCP validation summary has not been generated in this checkout."}
- Repo Memory MCP validation passed: ${localValidation.repoMemoryMcp.passed ? "yes" : "no"}
- Repo Memory MCP validation read-only: ${localValidation.repoMemoryMcp.readOnly === true ? "yes" : "unknown"}
- Repo Memory MCP modifies repository: ${localValidation.repoMemoryMcp.modifiesRepository === false ? "no" : "unknown"}
- Repo Memory MCP expected tools present: ${localValidation.repoMemoryMcp.expectedToolsPresent ? "yes" : "unknown"}
- Repo Memory MCP connector bridge recipes verified: ${localValidation.repoMemoryMcp.connectorBridgeRecipesVerified ? "yes" : "unknown"}
- Repo Memory MCP relaunch survival evidence: ${localValidation.repoMemoryMcp.relaunchSurvivalEvidence ?? "unknown"}
- Repo Memory MCP tool count: ${localValidation.repoMemoryMcp.toolCount}
- Repo Memory MCP validation steps: ${localValidation.repoMemoryMcp.stepCount}
- Repo Memory MCP command: ${localValidation.repoMemoryMcp.requiredCommand}
- Local-only network summary present: ${localValidation.localOnlyNetwork.summaryPresent ? "yes" : "no"} (${localValidation.localOnlyNetwork.summaryPath})
- Local-only network JSON present: ${localValidation.localOnlyNetwork.jsonPresent ? "yes" : "no"} (${localValidation.localOnlyNetwork.jsonPath})
${localValidation.localOnlyNetwork.generatedLine ? `- ${localValidation.localOnlyNetwork.generatedLine}` : "- Local-only network validation summary has not been generated in this checkout."}
- Local-only network validation passed: ${localValidation.localOnlyNetwork.passed ? "yes" : "no"}
- Local-only network mode: ${localValidation.localOnlyNetwork.localOnly ? "yes" : "unknown"}
- App-owned remote calls blocked: ${localValidation.localOnlyNetwork.appOwnedRemoteCallsBlocked ? "yes" : "unknown"}
- Local-only network guard surfaces: ${localValidation.localOnlyNetwork.coverage?.guardSurfaces ?? "unknown"}
- Local-only network app-owned remote-service surfaces: ${localValidation.localOnlyNetwork.coverage?.appOwnedRemoteServiceSurfaces ?? "unknown"}
- Local-only network provider-traffic surfaces: ${localValidation.localOnlyNetwork.coverage?.providerTrafficSurfaces ?? "unknown"}
- Local-only network managed-download surfaces: ${localValidation.localOnlyNetwork.coverage?.managedDownloadSurfaces ?? "unknown"}
- Local-only network validation steps: ${localValidation.localOnlyNetwork.stepCount}
- Local-only network command: ${localValidation.localOnlyNetwork.requiredCommand}
- ${localValidation.message}

## Shareable DMG Gates

- Environment clear: ${shareableDmgGate.environmentClear ? "yes" : "no"}
- Rust backend validation ready: ${shareableDmgGate.backendValidationReady ? "yes" : "no"}
- Signed and notarized: ${shareableDmgGate.signedAndNotarized ? "yes" : "no"}
- Updater feed ready: ${shareableDmgGate.updaterFeedReady ? "yes" : "no"}
- Static smoke preflight ready: ${shareableDmgGate.staticSmokePreflightReady ? "yes" : "no"}
- Installed-app smoke ready: ${shareableDmgGate.installedAppSmokeReady ? "yes" : "no"}
- Missing local evidence: ${missingLocalEvidenceLabels(localValidation).join(", ") || "none"}
- ${shareableDmgGate.message}

${managedConnectorReadinessSummary}

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
