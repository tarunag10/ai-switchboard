import fs from "node:fs";

const reportPath = "dist/release-readiness-report.json";
const requiredReleaseReportPaths = [
  "backendValidation.requiredCommands",
  "staticSmokePreflight.smokeSummaryPresent",
  "staticSmokePreflight.requiredCommand",
  "installedSmokeSummary.present",
  "installedSmoke.smokeSummaryPresent",
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
  const value = path.split(".").reduce((current, part) => current?.[part], root);
  if (!isObject(value)) {
    fail(`${path} must be an object`);
  }
  return value;
}

function requireArray(root, path) {
  const value = path.split(".").reduce((current, part) => current?.[part], root);
  if (!Array.isArray(value)) {
    fail(`${path} must be an array`);
  }
  return value;
}

function requireType(root, path, type) {
  const value = path.split(".").reduce((current, part) => current?.[part], root);
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
    const parent = parts.slice(0, index).reduce((current, key) => current?.[key], root);
    return isObject(parent) || Array.isArray(parent) ? part in parent : false;
  });
}

if (!fs.existsSync(reportPath)) {
  fail(`${reportPath} is missing; run npm run release:report first`);
  process.exit();
}

let report;
try {
  report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
} catch (error) {
  fail(`${reportPath} is not valid JSON: ${error.message}`);
  process.exit();
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
requireType(report, "backendValidation.message", "string");

for (const command of backendValidation.requiredCommands) {
  if (typeof command !== "string" || command.length === 0) {
    fail("backendValidation.requiredCommands must contain non-empty strings");
  }
}

requireObject(report, "staticSmokePreflight");
requireBooleanFields(report, "staticSmokePreflight", [
  "ready",
  "smokeSummaryPresent",
]);
requireType(report, "staticSmokePreflight.smokeSummaryPath", "string");
requireType(report, "staticSmokePreflight.requiredCommand", "string");
requireType(report, "staticSmokePreflight.message", "string");

requireObject(report, "installedSmokeSummary");
requireBooleanFields(report, "installedSmokeSummary", ["present"]);

requireObject(report, "installedSmoke");
requireBooleanFields(report, "installedSmoke", [
  "ready",
  "installedAppPresent",
  "smokeSummaryPresent",
]);
requireType(report, "installedSmoke.appPath", "string");
requireType(report, "installedSmoke.smokeSummaryPath", "string");
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
    if (!isObject(item) || typeof item.label !== "string" || typeof item.hint !== "string") {
      fail(`${collectionName} entries must include label and hint strings`);
    }
  }
}

if (process.exitCode) {
  process.exit();
}

console.log(`Release report schema OK: ${reportPath}`);
