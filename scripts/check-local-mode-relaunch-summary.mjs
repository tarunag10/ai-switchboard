#!/usr/bin/env node
import fs from "node:fs";
import { validateModeRelaunchSummary } from "./local-mode-relaunch-contract.mjs";

const reportPath = "dist/local-mode-relaunch-smoke-summary.json";
if (!fs.existsSync(reportPath)) {
  console.error(`mode relaunch summary check failed: ${reportPath} missing`);
  process.exit(1);
}
let report;
try {
  report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
} catch (error) {
  console.error(`mode relaunch summary check failed: invalid JSON (${error.message})`);
  process.exit(1);
}
const failures = validateModeRelaunchSummary(report);
if (failures.length) {
  console.error(`mode relaunch summary check failed: ${failures.join("; ")}`);
  process.exit(1);
}
console.log("Mode relaunch summary OK (config persistence boundary explicit).");
