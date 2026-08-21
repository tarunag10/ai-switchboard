import { spawnSync } from "node:child_process";
import fs from "node:fs";
import { buildReleaseReadinessActions, validateReleaseReadinessReport } from "./release-readiness-actions.mjs";

const strict = process.argv.includes("--strict");
const jsonOutput = process.argv.includes("--json");
const noRefresh = process.argv.includes("--no-refresh");
const reportArgumentIndex = process.argv.indexOf("--report");
if (reportArgumentIndex >= 0 && (!process.argv[reportArgumentIndex + 1] || process.argv[reportArgumentIndex + 1].startsWith("--"))) {
  console.error("release readiness --report requires a file path");
  process.exit(1);
}
const reportJsonPath = reportArgumentIndex >= 0 ? process.argv[reportArgumentIndex + 1] : "dist/release-readiness-report.json";

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    stdio: jsonOutput ? "pipe" : "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
    throw new Error(`${command} ${args.join(" ")} failed${output ? `:\n${output}` : ""}`);
  }
}

if (!noRefresh) {
  run("npm", ["run", "check:branding"]);
  run("npm", ["run", "check:local-only-network"]);
  run("npm", ["run", "release:report"]);
  run("npm", ["run", "release:report:check"]);
}

if (!fs.existsSync(reportJsonPath)) {
  console.error(`release readiness report not found: ${reportJsonPath}; run npm run release:report or provide --report <path>`);
  process.exit(1);
}

let report;
try {
  report = JSON.parse(fs.readFileSync(reportJsonPath, "utf8"));
} catch (error) {
  const detail = error instanceof SyntaxError ? "invalid JSON" : "could not be read";
  console.error(`release readiness report ${detail}: ${reportJsonPath}`);
  process.exit(1);
}
const reportFailures = validateReleaseReadinessReport(report);
if (reportFailures.length) {
  console.error(`release readiness report invalid: ${reportFailures.join("; ")}`);
  process.exit(1);
}
const actions = buildReleaseReadinessActions(report);

if (jsonOutput) {
  console.log(
    JSON.stringify(
      {
        status: report.status,
        strict,
        reportPath: reportJsonPath,
        actions,
      },
      null,
      2,
    ),
  );
} else {
  console.log(`Release readiness: ${report.status}`);
  console.log(`Report: ${reportJsonPath}`);
  if (actions.length > 0) {
    console.log("Next actions:");
    for (const action of actions) {
      console.log(`- ${action.label}`);
      console.log(`  ${action.command}`);
      console.log(`  ${action.detail}`);
    }
  } else {
    console.log("No release blockers found.");
  }
}

if (strict && report.status !== "ready") {
  process.exit(1);
}
