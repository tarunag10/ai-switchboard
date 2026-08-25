#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { validateConnectorsMount } from "./lib/ui-wiring-checks.mjs";

function fail(message) {
  console.error(`connectors mount check failed: ${message}`);
  process.exit(1);
}

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDirectory, "..");
const trayAppSource = fs.readFileSync(path.join(root, "src/app/TrayApp.tsx"), "utf8");
const sidebarSource = fs.readFileSync(path.join(root, "src/components/TraySidebar.tsx"), "utf8");
const failures = validateConnectorsMount({ trayAppSource, sidebarSource });

if (failures.length > 0) fail(failures.join("; "));

console.log("Agents & Connectors mount OK: sidebar route renders SettingsConnectorPanel.");
