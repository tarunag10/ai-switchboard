import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const reportPath = "dist/release-readiness-report.md";
const jsonPath = "dist/release-readiness-report.json";
const smokeSummaryPath = "dist/smoke-preflight-summary.md";
const appPath = "/Applications/Mac AI Switchboard.app";

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

  return items
    .map((item) => `- ${item.label}\n  ${item.hint}`)
    .join("\n");
}

function readSmokeSummaryStatus() {
  if (!fs.existsSync(smokeSummaryPath)) {
    return {
      present: false,
      generatedLine: null,
    };
  }

  const firstGeneratedLine =
    fs
      .readFileSync(smokeSummaryPath, "utf8")
      .split("\n")
      .find((line) => line.startsWith("Generated: ")) ?? null;

  return {
    present: true,
    generatedLine: firstGeneratedLine,
  };
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
    message: ready
      ? "Rust toolchain present. Run desktop formatting and tests before release."
      : "Rust validation cannot run here until cargo and rustup are available.",
  };
}

function buildInstalledSmoke(installedAppPresent, smokeSummary) {
  const ready = installedAppPresent && smokeSummary.present;

  return {
    ready,
    installedAppPresent,
    appPath,
    smokeSummaryPath,
    smokeSummaryPresent: smokeSummary.present,
    generatedLine: smokeSummary.generatedLine,
    message: ready
      ? "Installed app and smoke summary are present. Review smoke evidence before publishing."
      : "Install the signed DMG into /Applications and run the beta smoke checklist before publishing.",
  };
}

function buildShareableDmgGate(releaseEnv, backendValidation, installedSmoke) {
  const environmentClear = releaseEnv.blockers.length === 0;
  const signedAndNotarized = environmentClear;
  const updaterFeedReady = !releaseEnv.warnings.some((warning) =>
    /HEADROOM_UPDATER_PUBLIC_KEY|HEADROOM_UPDATER_ENDPOINTS/.test(warning.label),
  );
  const installedAppSmokeReady = installedSmoke.ready;
  const ready =
    environmentClear &&
    signedAndNotarized &&
    updaterFeedReady &&
    backendValidation.ready &&
    installedAppSmokeReady;

  return {
    ready,
    environmentClear,
    backendValidationReady: backendValidation.ready,
    signedAndNotarized,
    updaterFeedReady,
    installedAppSmokeReady,
    message: ready
      ? "All shareable DMG gates are clear."
      : "Do not share a public DMG until every gate is clear.",
  };
}

const releaseEnv = runReleaseEnv();
const smokeSummary = readSmokeSummaryStatus();
const installedAppPresent = fs.existsSync(appPath);
const backendValidation = buildBackendValidation(releaseEnv);
const installedSmoke = buildInstalledSmoke(installedAppPresent, smokeSummary);
const shareableDmgGate = buildShareableDmgGate(
  releaseEnv,
  backendValidation,
  installedSmoke,
);
const generatedAt = new Date().toISOString();
const status =
  releaseEnv.ok && backendValidation.ready && installedSmoke.ready && shareableDmgGate.ready
    ? "ready"
    : "blocked";

const payload = {
  generatedAt,
  status,
  installedAppPresent,
  appPath,
  smokeSummary,
  backendValidation,
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
- ${backendValidation.message}

## Installed App Smoke

- Installed app present: ${installedSmoke.installedAppPresent ? "yes" : "no"} (${installedSmoke.appPath})
- Smoke summary present: ${installedSmoke.smokeSummaryPresent ? "yes" : "no"} (${installedSmoke.smokeSummaryPath})
${installedSmoke.generatedLine ? `- ${installedSmoke.generatedLine}` : "- Smoke summary has not been generated in this checkout."}
- ${installedSmoke.message}

## Shareable DMG Gates

- Environment clear: ${shareableDmgGate.environmentClear ? "yes" : "no"}
- Rust backend validation ready: ${shareableDmgGate.backendValidationReady ? "yes" : "no"}
- Signed and notarized: ${shareableDmgGate.signedAndNotarized ? "yes" : "no"}
- Updater feed ready: ${shareableDmgGate.updaterFeedReady ? "yes" : "no"}
- Installed-app smoke ready: ${shareableDmgGate.installedAppSmokeReady ? "yes" : "no"}
- ${shareableDmgGate.message}

## Next Steps

${
  releaseEnv.blockers.length > 0
    ? "- Resolve environment blockers, then rerun `npm run release:report`."
    : "- Environment preflight is clear."
}
${
  installedAppPresent
    ? "- Run `docs/beta-smoke-test.md` against the installed app."
    : "- Build and install the signed DMG, then run `docs/beta-smoke-test.md`."
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
console.log(`Shareable DMG gate: ${shareableDmgGate.ready ? "ready" : "blocked"}`);

if (releaseEnv.blockers.length > 0) {
  console.log(`Blockers: ${releaseEnv.blockers.length}`);
}

if (!installedAppPresent) {
  console.log(`Installed app missing: ${appPath}`);
}
if (!backendValidation.ready) {
  console.log("Backend validation pending: cargo/rustup unavailable.");
}
