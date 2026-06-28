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
if (!markdownReport.includes("Planned connector config creation plan")) {
  fail(`${markdownReportPath} must include planned connector config creation plan evidence`);
}
if (!markdownReport.includes("## Planned Connector Readiness")) {
  fail(`${markdownReportPath} must include planned connector readiness summary`);
}
if (!markdownReport.includes("Full per-tool dossiers are available from Doctor")) {
  fail(`${markdownReportPath} must point to Doctor connector dossiers`);
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
if (!requiredEvidence.includes("Planned connector automation gates")) {
  fail(
    "staticSmokePreflight.requiredEvidence must include planned connector automation gates",
  );
}
if (!requiredEvidence.includes("Planned connector manual workflow")) {
  fail(
    "staticSmokePreflight.requiredEvidence must include planned connector manual workflow",
  );
}
if (!requiredEvidence.includes("Planned connector config creation plan")) {
  fail(
    "staticSmokePreflight.requiredEvidence must include planned connector config creation plan",
  );
}
if (!requiredEvidence.includes("Gemini connector dry-run preview evidence")) {
  fail(
    "staticSmokePreflight.requiredEvidence must include Gemini connector dry-run preview evidence",
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
  "Planned connector automation gates, manual workflow, config creation plan, and Gemini dry-run preview evidence",
  "Codex compression recovery",
]) {
  if (!installedRequiredEvidence.includes(requiredItem)) {
    fail(`installedSmoke.requiredEvidence must include ${requiredItem}`);
  }
}
requireType(report, "installedSmoke.message", "string");

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
