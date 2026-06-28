import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const appPath = "/Applications/Mac AI Switchboard.app";
const appInfoPlistPath = path.join(appPath, "Contents", "Info.plist");
const betaSmokeDoc = "docs/beta-smoke-test.md";
const preflightSummaryPath = "dist/smoke-preflight-summary.md";
const installedSummaryPath = "dist/installed-smoke-summary.md";
const confirmed =
  process.argv.includes("--confirm") ||
  process.env.MAC_AI_SWITCHBOARD_INSTALLED_SMOKE_PASSED === "1";

const appPresent = fs.existsSync(appPath);
const bundleMetadataPresent = fs.existsSync(appInfoPlistPath);
const preflightPresent = fs.existsSync(preflightSummaryPath);

if (!appPresent) {
  console.error(`Installed app missing: ${appPath}`);
  console.error(
    "Install the signed DMG, run docs/beta-smoke-test.md, then rerun npm run smoke:installed -- --confirm.",
  );
  process.exit(1);
}

if (!bundleMetadataPresent) {
  console.error(`Installed app metadata missing: ${appInfoPlistPath}`);
  console.error(
    "Install the signed DMG by dragging Mac AI Switchboard.app into /Applications, then rerun npm run smoke:installed -- --confirm.",
  );
  process.exit(1);
}

if (!preflightPresent) {
  console.error(`Smoke preflight summary missing: ${preflightSummaryPath}`);
  console.error(
    "Run npm run smoke:preflight before recording installed-app smoke evidence.",
  );
  process.exit(1);
}

if (!confirmed) {
  console.error("Installed-app smoke confirmation missing.");
  console.error(
    "After docs/beta-smoke-test.md passes on /Applications/Mac AI Switchboard.app, rerun: npm run smoke:installed -- --confirm",
  );
  console.error(
    "Automation may set MAC_AI_SWITCHBOARD_INSTALLED_SMOKE_PASSED=1 instead.",
  );
  process.exit(1);
}

const generatedAt = new Date().toISOString();
const betaSmokeChecklistSha256 = crypto
  .createHash("sha256")
  .update(fs.readFileSync(betaSmokeDoc))
  .digest("hex");
const evidenceAreas = [
  "Switchboard modes and degraded-mode Doctor guidance",
  "Switchboard copyable state",
  "Doctor automatic/manual triage and repair actions",
  "Doctor copyable report",
  "Managed connector automation gates, manual workflow, config creation plan, and Gemini dry-run preview evidence",
  "Repo Intelligence recipes and local context packs",
  "Savings calculator copyable ledger",
  "Per-tool agent handoffs",
  "Codex compression recovery",
];
const summary = `# Installed App Smoke Summary

Generated: ${generatedAt}

- Installed app present: yes (${appPath})
- Installed app metadata present: yes (${appInfoPlistPath})
- Static preflight summary: ${preflightSummaryPath}
- Installed-app checklist: ${betaSmokeDoc}
- Installed-app checklist SHA-256: ${betaSmokeChecklistSha256}
- Confirmation: explicit tester confirmation received
- Result: tester confirmed beta smoke checklist passed on the installed app.

## Confirmed Evidence Areas

${evidenceAreas.map((area) => `- ${area}`).join("\n")}

Keep this file with dist/release-readiness-report.md before sharing a public DMG.
`;

fs.mkdirSync(path.dirname(installedSummaryPath), { recursive: true });
fs.writeFileSync(installedSummaryPath, summary);

console.log("Installed-app smoke summary written.");
console.log(`Summary written: ${installedSummaryPath}`);
