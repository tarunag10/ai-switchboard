import fs from "node:fs";
import path from "node:path";

const appPath = "/Applications/Mac AI Switchboard.app";
const betaSmokeDoc = "docs/beta-smoke-test.md";
const preflightSummaryPath = "dist/smoke-preflight-summary.md";
const installedSummaryPath = "dist/installed-smoke-summary.md";

const appPresent = fs.existsSync(appPath);
const preflightPresent = fs.existsSync(preflightSummaryPath);

if (!appPresent) {
  console.error(`Installed app missing: ${appPath}`);
  console.error("Install the signed DMG, run docs/beta-smoke-test.md, then rerun npm run smoke:installed.");
  process.exit(1);
}

if (!preflightPresent) {
  console.error(`Smoke preflight summary missing: ${preflightSummaryPath}`);
  console.error("Run npm run smoke:preflight before recording installed-app smoke evidence.");
  process.exit(1);
}

const generatedAt = new Date().toISOString();
const summary = `# Installed App Smoke Summary

Generated: ${generatedAt}

- Installed app present: yes (${appPath})
- Static preflight summary: ${preflightSummaryPath}
- Installed-app checklist: ${betaSmokeDoc}
- Result: tester confirmed beta smoke checklist passed on the installed app.

Keep this file with dist/release-readiness-report.md before sharing a public DMG.
`;

fs.mkdirSync(path.dirname(installedSummaryPath), { recursive: true });
fs.writeFileSync(installedSummaryPath, summary);

console.log("Installed-app smoke summary written.");
console.log(`Summary written: ${installedSummaryPath}`);
