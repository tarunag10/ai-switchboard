import fs from "node:fs";

const reportPath = "dist/release-readiness-report.json";
const markdownReportPath = "dist/release-readiness-report.md";
const requiredReleaseReportPaths = [
  "backendValidation.requiredCommands",
  "backendValidation.unblockCommands",
  "staticSmokePreflight.smokeSummaryPresent",
  "staticSmokePreflight.requiredCommand",
  "staticSmokePreflight.requiredEvidence",
  "staticSmokePreflight.missingEvidence",
  "staticSmokePreflight.evidenceReady",
  "installedSmokeSummary.present",
  "installedSmoke.smokeSummaryPresent",
  "installedSmoke.requiredEvidence",
  "installedSmoke.missingEvidence",
  "installedSmoke.evidenceReady",
  "installedSmoke.checklistSha256Matches",
  "localValidation.ready",
  "localValidation.releaseGateEvidence",
  "localValidation.localInstalled.summaryPath",
  "localValidation.localInstalled.jsonPath",
  "localValidation.localInstalled.passed",
  "localValidation.localInstalled.appPresent",
  "localValidation.localInstalled.metadataMatches",
  "localValidation.localInstalled.dmgVerified",
  "localValidation.localInstalled.codesignVerified",
  "localValidation.localInstalled.runtimeHealthChecked",
  "localValidation.localInstalled.appListenerReady",
  "localValidation.localInstalled.engineProxyReady",
  "localValidation.localInstalled.requiredCommand",
  "localValidation.modeRelaunch.summaryPath",
  "localValidation.modeRelaunch.jsonPath",
  "localValidation.modeRelaunch.passed",
  "localValidation.modeRelaunch.offModeProxyDown",
  "localValidation.modeRelaunch.rtkModeProxyDown",
  "localValidation.modeRelaunch.restored",
  "localValidation.modeRelaunch.requiredCommand",
  "localValidation.rollback.summaryPath",
  "localValidation.rollback.jsonPath",
  "localValidation.rollback.passed",
  "localValidation.rollback.requiredCommand",
  "localValidation.doctorRepair.summaryPath",
  "localValidation.doctorRepair.jsonPath",
  "localValidation.doctorRepair.passed",
  "localValidation.doctorRepair.requiredCommand",
  "localValidation.uninstall.summaryPath",
  "localValidation.uninstall.jsonPath",
  "localValidation.uninstall.passed",
  "localValidation.uninstall.destructive",
  "localValidation.uninstall.requiredCommand",
  "localValidation.repoIntelligence.summaryPath",
  "localValidation.repoIntelligence.jsonPath",
  "localValidation.repoIntelligence.passed",
  "localValidation.repoIntelligence.readOnly",
  "localValidation.repoIntelligence.modifiesRepository",
  "localValidation.repoIntelligence.requiredCommand",
  "localValidation.repoMemoryMcp.summaryPath",
  "localValidation.repoMemoryMcp.jsonPath",
  "localValidation.repoMemoryMcp.passed",
  "localValidation.repoMemoryMcp.readOnly",
  "localValidation.repoMemoryMcp.modifiesRepository",
  "localValidation.repoMemoryMcp.requiredCommand",
  "localValidation.localOnlyNetwork.summaryPath",
  "localValidation.localOnlyNetwork.jsonPath",
  "localValidation.localOnlyNetwork.passed",
  "localValidation.localOnlyNetwork.localOnly",
  "localValidation.localOnlyNetwork.appOwnedRemoteCallsBlocked",
  "localValidation.localOnlyNetwork.requiredCommand",
  "shareableDmgGate.staticSmokePreflightReady",
  "shareableDmgGate.updaterFeedReady",
  "releaseEnv.blockers",
  "releaseEnv.warnings",
];

function fail(message) {
  console.error(`release report schema check failed: ${message}`);
  process.exitCode = 1;
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function requireObject(root, path) {
  const value = path
    .split(".")
    .reduce((current, part) => current?.[part], root);
  if (!isObject(value)) {
    fail(`${path} must be an object`);
  }
  return value;
}

function requireArray(root, path) {
  const value = path
    .split(".")
    .reduce((current, part) => current?.[part], root);
  if (!Array.isArray(value)) {
    fail(`${path} must be an array`);
  }
  return value;
}

function requireType(root, path, type) {
  const value = path
    .split(".")
    .reduce((current, part) => current?.[part], root);
  if (typeof value !== type) {
    fail(`${path} must be ${type}`);
  }
  return value;
}

function requireBooleanFields(root, prefix, fields) {
  for (const field of fields) {
    requireType(root, `${prefix}.${field}`, "boolean");
  }
}

function hasPath(root, path) {
  return path.split(".").every((part, index, parts) => {
    const parent = parts
      .slice(0, index)
      .reduce((current, key) => current?.[key], root);
    return isObject(parent) || Array.isArray(parent) ? part in parent : false;
  });
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} is missing; run npm run release:report first`);
  process.exit();
}
if (!fs.existsSync(markdownReportPath)) {
  fail(`${markdownReportPath} is missing; run npm run release:report first`);
  process.exit();
}

let report;
try {
  report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
} catch (error) {
  fail(`${reportPath} is not valid JSON: ${error.message}`);
  process.exit();
}
const markdownReport = fs.readFileSync(markdownReportPath, "utf8");
if (!markdownReport.includes("Managed connector config creation plan")) {
  fail(`${markdownReportPath} must include managed connector config creation plan evidence`);
}
if (!markdownReport.includes("Connector readiness payload in agent handoffs")) {
  fail(`${markdownReportPath} must include connector readiness payload evidence`);
}
if (!markdownReport.includes("## Managed Connector Readiness")) {
  fail(`${markdownReportPath} must include managed connector readiness summary`);
}
if (!markdownReport.includes("Full per-tool dossiers are available from Doctor")) {
  fail(`${markdownReportPath} must point to Doctor connector dossiers`);
}
const connectorManifest = JSON.parse(
  fs.readFileSync("connectors/manifest.json", "utf8"),
);
const connectorManifestRows = Array.isArray(connectorManifest)
  ? connectorManifest
  : connectorManifest.connectors;
if (!Array.isArray(connectorManifestRows)) {
  fail("connectors/manifest.json must contain a connector array");
}
const managedConnectors = connectorManifestRows.filter(
  (connector) => connector.support_status === "managed",
);
if (report.managedConnectorReadiness?.managedCount !== managedConnectors.length) {
  fail(
    `managedConnectorReadiness.managedCount must match connectors/manifest.json (${managedConnectors.length})`,
  );
}
if (!markdownReport.includes(`Managed connectors: ${managedConnectors.length}`)) {
  fail(
    `${markdownReportPath} must report the manifest-managed connector count (${managedConnectors.length})`,
  );
}
for (const connector of managedConnectors) {
  if (!markdownReport.includes(connector.name)) {
    fail(`${markdownReportPath} must list managed connector ${connector.name}`);
  }
}
if (!markdownReport.includes("## Local Doctor and Rollback Validation")) {
  fail(`${markdownReportPath} must include local Doctor and Rollback validation evidence`);
}
if (
  !markdownReport.includes(
    "Mode relaunch command: npm run smoke:mode-relaunch:local -- --confirm",
  )
) {
  fail(`${markdownReportPath} must include the local mode relaunch smoke command`);
}
if (!markdownReport.includes("Rollback command: npm run smoke:rollback:local")) {
  fail(`${markdownReportPath} must include the local rollback validation command`);
}
if (
  !markdownReport.includes(
    "Doctor repair command: npm run smoke:doctor-repair:local",
  )
) {
  fail(`${markdownReportPath} must include the local Doctor repair validation command`);
}
if (!markdownReport.includes("Uninstall command: npm run smoke:uninstall:local")) {
  fail(`${markdownReportPath} must include the local uninstall validation command`);
}
if (
  !markdownReport.includes(
    "Repo Intelligence command: npm run smoke:repo-intelligence:local",
  )
) {
  fail(`${markdownReportPath} must include the local Repo Intelligence validation command`);
}

requireType(report, "status", "string");
requireType(report, "generatedAt", "string");

for (const path of requiredReleaseReportPaths) {
  if (!hasPath(report, path)) {
    fail(`${path} is missing`);
  }
}

const backendValidation = requireObject(report, "backendValidation");
requireBooleanFields(report, "backendValidation", [
  "ready",
  "cargoAvailable",
  "rustupAvailable",
]);
requireArray(report, "backendValidation.requiredCommands");
requireArray(report, "backendValidation.unblockCommands");
requireType(report, "backendValidation.message", "string");

for (const command of [
  ...backendValidation.requiredCommands,
  ...backendValidation.unblockCommands,
]) {
  if (typeof command !== "string" || command.length === 0) {
    fail("backendValidation command arrays must contain non-empty strings");
  }
}

requireObject(report, "staticSmokePreflight");
requireBooleanFields(report, "staticSmokePreflight", [
  "ready",
  "smokeSummaryPresent",
  "evidenceReady",
]);
requireType(report, "staticSmokePreflight.smokeSummaryPath", "string");
requireType(report, "staticSmokePreflight.requiredCommand", "string");
const requiredEvidence = requireArray(
  report,
  "staticSmokePreflight.requiredEvidence",
);
const missingStaticEvidence = requireArray(
  report,
  "staticSmokePreflight.missingEvidence",
);
for (const item of requiredEvidence) {
  if (typeof item !== "string" || item.length === 0) {
    fail(
      "staticSmokePreflight.requiredEvidence entries must be non-empty strings",
    );
  }
}
for (const item of missingStaticEvidence) {
  if (typeof item !== "string" || item.length === 0) {
    fail(
      "staticSmokePreflight.missingEvidence entries must be non-empty strings",
    );
  }
}
if (!requiredEvidence.includes("Managed connector automation gates")) {
  fail(
    "staticSmokePreflight.requiredEvidence must include managed connector automation gates",
  );
}
if (!requiredEvidence.includes("Managed connector native config gate")) {
  fail(
    "staticSmokePreflight.requiredEvidence must include managed connector native config gate",
  );
}
if (!requiredEvidence.includes("Managed connector config creation plan")) {
  fail(
    "staticSmokePreflight.requiredEvidence must include managed connector config creation plan",
  );
}
if (!requiredEvidence.includes("Connector readiness payload in agent handoffs")) {
  fail(
    "staticSmokePreflight.requiredEvidence must include connector readiness payload in agent handoffs",
  );
}
if (!requiredEvidence.includes("Managed connector readiness evidence")) {
  fail(
    "staticSmokePreflight.requiredEvidence must include managed connector readiness evidence",
  );
}
requireType(report, "staticSmokePreflight.message", "string");

requireObject(report, "installedSmokeSummary");
requireBooleanFields(report, "installedSmokeSummary", ["present"]);

requireObject(report, "installedSmoke");
requireBooleanFields(report, "installedSmoke", [
  "ready",
  "installedAppPresent",
  "bundleMetadataPresent",
  "smokeSummaryPresent",
  "evidenceReady",
  "checklistSha256Matches",
]);
requireType(report, "installedSmoke.appPath", "string");
requireType(report, "installedSmoke.appInfoPlistPath", "string");
requireType(report, "installedSmoke.smokeSummaryPath", "string");
requireType(report, "installedSmoke.betaSmokeDoc", "string");
const currentChecklistSha256 = report.installedSmoke.currentChecklistSha256;
if (
  currentChecklistSha256 !== null &&
  typeof currentChecklistSha256 !== "string"
) {
  fail("installedSmoke.currentChecklistSha256 must be string or null");
}
const recordedChecklistSha256 = report.installedSmoke.recordedChecklistSha256;
if (
  recordedChecklistSha256 !== null &&
  typeof recordedChecklistSha256 !== "string"
) {
  fail("installedSmoke.recordedChecklistSha256 must be string or null");
}
const installedRequiredEvidence = requireArray(
  report,
  "installedSmoke.requiredEvidence",
);
const installedMissingEvidence = requireArray(
  report,
  "installedSmoke.missingEvidence",
);
for (const item of [
  ...installedRequiredEvidence,
  ...installedMissingEvidence,
]) {
  if (typeof item !== "string" || item.length === 0) {
    fail("installedSmoke evidence arrays must contain non-empty strings");
  }
}
for (const requiredItem of [
  "Switchboard modes and degraded-mode Doctor guidance",
  "Switchboard copyable state",
  "Doctor copyable report",
  "Savings calculator copyable ledger",
  "Managed connector automation gates, manual workflow, config creation plan, and managed connector readiness evidence",
  "Connector readiness payload in agent handoffs",
  "Codex compression recovery",
]) {
  if (!installedRequiredEvidence.includes(requiredItem)) {
    fail(`installedSmoke.requiredEvidence must include ${requiredItem}`);
  }
}
requireType(report, "installedSmoke.message", "string");

const localValidation = requireObject(report, "localValidation");
requireBooleanFields(report, "localValidation", [
  "ready",
  "releaseGateEvidence",
]);
if (localValidation.releaseGateEvidence !== false) {
  fail("localValidation.releaseGateEvidence must remain false for local-only evidence");
}
for (const prefix of [
  "localValidation.localInstalled",
  "localValidation.rollback",
  "localValidation.doctorRepair",
  "localValidation.uninstall",
  "localValidation.repoIntelligence",
  "localValidation.repoMemoryMcp",
  "localValidation.localOnlyNetwork",
]) {
  requireObject(report, prefix);
  requireType(report, `${prefix}.summaryPath`, "string");
  requireType(report, `${prefix}.jsonPath`, "string");
  requireBooleanFields(report, prefix, [
    "summaryPresent",
    "jsonPresent",
    "passed",
  ]);
  if (prefix !== "localValidation.localInstalled") {
    requireType(report, `${prefix}.stepCount`, "number");
  }
  requireType(report, `${prefix}.requiredCommand`, "string");
  const kind = prefix
    .split(".")
    .reduce((current, part) => current?.[part], report).kind;
  if (kind !== null && typeof kind !== "string") {
    fail(`${prefix}.kind must be string or null`);
  }
  const parseError = prefix
    .split(".")
    .reduce((current, part) => current?.[part], report).parseError;
  if (parseError !== null && typeof parseError !== "string") {
    fail(`${prefix}.parseError must be string or null`);
  }
}
for (const field of [
  "appPresent",
  "metadataMatches",
  "dmgVerified",
  "codesignVerified",
  "runtimeHealthChecked",
  "appListenerReady",
  "engineProxyReady",
]) {
  requireType(report, `localValidation.localInstalled.${field}`, "boolean");
}
requireObject(report, "localValidation.modeRelaunch");
requireType(report, "localValidation.modeRelaunch.summaryPath", "string");
requireType(report, "localValidation.modeRelaunch.jsonPath", "string");
requireBooleanFields(report, "localValidation.modeRelaunch", [
  "summaryPresent",
  "jsonPresent",
  "passed",
  "offModeProxyDown",
  "rtkModeProxyDown",
  "restored",
]);
requireType(report, "localValidation.modeRelaunch.modeCount", "number");
requireType(report, "localValidation.modeRelaunch.requiredCommand", "string");
const modeRelaunchKind = report.localValidation.modeRelaunch.kind;
if (modeRelaunchKind !== null && typeof modeRelaunchKind !== "string") {
  fail("localValidation.modeRelaunch.kind must be string or null");
}
const modeRelaunchParseError = report.localValidation.modeRelaunch.parseError;
if (
  modeRelaunchParseError !== null &&
  typeof modeRelaunchParseError !== "string"
) {
  fail("localValidation.modeRelaunch.parseError must be string or null");
}
if (
  report.localValidation.localInstalled.requiredCommand !==
  "npm run smoke:installed:local"
) {
  fail(
    "localValidation.localInstalled.requiredCommand must be npm run smoke:installed:local",
  );
}
if (
  report.localValidation.modeRelaunch.requiredCommand !==
  "npm run smoke:mode-relaunch:local -- --confirm"
) {
  fail(
    "localValidation.modeRelaunch.requiredCommand must be npm run smoke:mode-relaunch:local -- --confirm",
  );
}
if (
  report.localValidation.rollback.requiredCommand !==
  "npm run smoke:rollback:local"
) {
  fail("localValidation.rollback.requiredCommand must be npm run smoke:rollback:local");
}
if (
  report.localValidation.doctorRepair.requiredCommand !==
  "npm run smoke:doctor-repair:local"
) {
  fail(
    "localValidation.doctorRepair.requiredCommand must be npm run smoke:doctor-repair:local",
  );
}
if (
  report.localValidation.uninstall.requiredCommand !==
  "npm run smoke:uninstall:local"
) {
  fail(
    "localValidation.uninstall.requiredCommand must be npm run smoke:uninstall:local",
  );
}
if (report.localValidation.uninstall.destructive !== false) {
  fail("localValidation.uninstall.destructive must be false");
}
if (
  report.localValidation.repoIntelligence.requiredCommand !==
  "npm run smoke:repo-intelligence:local"
) {
  fail(
    "localValidation.repoIntelligence.requiredCommand must be npm run smoke:repo-intelligence:local",
  );
}
if (report.localValidation.repoIntelligence.modifiesRepository !== false) {
  fail("localValidation.repoIntelligence.modifiesRepository must be false");
}
if (
  report.localValidation.repoMemoryMcp.requiredCommand !==
  "npm run smoke:repo-memory-mcp:local"
) {
  fail(
    "localValidation.repoMemoryMcp.requiredCommand must be npm run smoke:repo-memory-mcp:local",
  );
}
if (report.localValidation.repoMemoryMcp.modifiesRepository !== false) {
  fail("localValidation.repoMemoryMcp.modifiesRepository must be false");
}
requireType(report, "localValidation.repoMemoryMcp.readOnly", "boolean");
requireType(
  report,
  "localValidation.repoMemoryMcp.expectedToolsPresent",
  "boolean",
);
requireType(
  report,
  "localValidation.repoMemoryMcp.connectorBridgeRecipesVerified",
  "boolean",
);
requireType(report, "localValidation.repoMemoryMcp.toolCount", "number");
const repoMemoryMcpRelaunchEvidence =
  report.localValidation.repoMemoryMcp.relaunchSurvivalEvidence;
if (
  repoMemoryMcpRelaunchEvidence !== null &&
  typeof repoMemoryMcpRelaunchEvidence !== "string"
) {
  fail("localValidation.repoMemoryMcp.relaunchSurvivalEvidence must be string or null");
}
if (
  report.localValidation.repoMemoryMcp.passed === true &&
  report.localValidation.repoMemoryMcp.readOnly !== true
) {
  fail("localValidation.repoMemoryMcp.readOnly must be true when validation passes");
}
if (
  report.localValidation.repoMemoryMcp.passed === true &&
  report.localValidation.repoMemoryMcp.expectedToolsPresent !== true
) {
  fail(
    "localValidation.repoMemoryMcp.expectedToolsPresent must be true when validation passes",
  );
}
if (
  report.localValidation.localOnlyNetwork.requiredCommand !==
  "npm run smoke:local-only:local"
) {
  fail(
    "localValidation.localOnlyNetwork.requiredCommand must be npm run smoke:local-only:local",
  );
}
requireType(report, "localValidation.localOnlyNetwork.localOnly", "boolean");
requireType(report, "localValidation.localOnlyNetwork.appOwnedRemoteCallsBlocked", "boolean");
if (report.localValidation.localOnlyNetwork.passed === true) {
  requireObject(report, "localValidation.localOnlyNetwork.coverage");
  for (const field of [
    "guardSurfaces",
    "appOwnedRemoteServiceSurfaces",
    "providerTrafficSurfaces",
    "managedDownloadSurfaces",
  ]) {
    requireType(report, `localValidation.localOnlyNetwork.coverage.${field}`, "number");
  }
}
if (
  report.localValidation.localOnlyNetwork.passed === true &&
  report.localValidation.localOnlyNetwork.localOnly !== true
) {
  fail("localValidation.localOnlyNetwork.localOnly must be true when validation passes");
}
if (
  report.localValidation.localOnlyNetwork.passed === true &&
  report.localValidation.localOnlyNetwork.appOwnedRemoteCallsBlocked !== true
) {
  fail(
    "localValidation.localOnlyNetwork.appOwnedRemoteCallsBlocked must be true when validation passes",
  );
}
requireType(report, "localValidation.message", "string");

requireObject(report, "shareableDmgGate");
requireBooleanFields(report, "shareableDmgGate", [
  "ready",
  "environmentClear",
  "backendValidationReady",
  "signedAndNotarized",
  "updaterFeedReady",
  "staticSmokePreflightReady",
  "installedAppSmokeReady",
]);
requireType(report, "shareableDmgGate.message", "string");

requireObject(report, "releaseEnv");
requireType(report, "releaseEnv.ok", "boolean");
requireType(report, "releaseEnv.strict", "boolean");
const blockers = requireArray(report, "releaseEnv.blockers");
const warnings = requireArray(report, "releaseEnv.warnings");

for (const [collectionName, items] of [
  ["releaseEnv.blockers", blockers],
  ["releaseEnv.warnings", warnings],
]) {
  for (const item of items) {
    if (
      !isObject(item) ||
      typeof item.label !== "string" ||
      typeof item.hint !== "string"
    ) {
      fail(`${collectionName} entries must include label and hint strings`);
    }
  }
}

if (process.exitCode) {
  process.exit();
}

console.log(`Release report schema OK: ${reportPath}`);
