#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { validateRepoMapMount } from "./lib/ui-wiring-checks.mjs";

function fail(message) {
  console.error(`repo map mount check failed: ${message}`);
  process.exit(1);
}

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDirectory, "..");
const shellSource = fs.readFileSync(path.join(root, "src/components/TrayAppShell.tsx"), "utf8");
const sidebarSource = fs.readFileSync(path.join(root, "src/components/TraySidebar.tsx"), "utf8");
const failures = validateRepoMapMount({ shellSource, sidebarSource });

if (failures.length > 0) fail(failures.join("; "));

console.log("Repo Map mount OK: sidebar route renders RepoMapView.");
