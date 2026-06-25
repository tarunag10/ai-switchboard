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

const releaseEnv = runReleaseEnv();
const smokeSummary = readSmokeSummaryStatus();
const installedAppPresent = fs.existsSync(appPath);
const generatedAt = new Date().toISOString();
const status = releaseEnv.ok && installedAppPresent ? "ready" : "blocked";

const payload = {
  generatedAt,
  status,
  installedAppPresent,
  appPath,
  smokeSummary,
  releaseEnv,
};

const report = `# Release Readiness Report

Generated: ${generatedAt}

Status: ${status}

## Environment Blockers

${listItems(releaseEnv.blockers, "None. Release environment blockers are clear.")}

## Environment Warnings

${listItems(releaseEnv.warnings, "None. Recommended release settings are present.")}

## Installed App Smoke

- Installed app present: ${installedAppPresent ? "yes" : "no"} (${appPath})
- Smoke summary present: ${smokeSummary.present ? "yes" : "no"} (${smokeSummaryPath})
${smokeSummary.generatedLine ? `- ${smokeSummary.generatedLine}` : "- Smoke summary has not been generated in this checkout."}

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
- Before publishing, run \`npm run release:check\`.
`;

fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, report);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log(`Release readiness status: ${status}`);
console.log(`Report written: ${reportPath}`);
console.log(`JSON written: ${jsonPath}`);

if (releaseEnv.blockers.length > 0) {
  console.log(`Blockers: ${releaseEnv.blockers.length}`);
}

if (!installedAppPresent) {
  console.log(`Installed app missing: ${appPath}`);
}
