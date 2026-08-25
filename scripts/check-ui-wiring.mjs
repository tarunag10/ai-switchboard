#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  collectLiteralFrontendInvokes,
  collectDeadFrontendButtons,
  extractMountedRouteIds,
  extractRegisteredTauriCommands,
  extractSidebarRouteIds,
  findUnmountedSidebarRoutes,
  findUnregisteredInvokes,
  validateConnectorsMount,
  validateRepoMapMount,
} from "./lib/ui-wiring-checks.mjs";

function fail(message) {
  console.error(`UI wiring check failed: ${message}`);
  process.exit(1);
}

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDirectory, "..");
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), "utf8");

const shellSource = read("src/components/TrayAppShell.tsx");
const sidebarSource = read("src/components/TraySidebar.tsx");
const trayAppSource = read("src/app/TrayApp.tsx");
const repoMapFailures = validateRepoMapMount({ shellSource, sidebarSource });
if (repoMapFailures.length > 0) fail(repoMapFailures.join("; "));

const connectorsFailures = validateConnectorsMount({ trayAppSource, sidebarSource });
if (connectorsFailures.length > 0) fail(connectorsFailures.join("; "));

const sidebarRoutes = extractSidebarRouteIds(sidebarSource);
const mountedRoutes = extractMountedRouteIds([
  shellSource,
  trayAppSource,
  read("src/components/AddonsView.tsx"),
  read("src/components/OptimizationView.tsx"),
]);
const unmountedRoutes = findUnmountedSidebarRoutes(sidebarRoutes, mountedRoutes);
if (unmountedRoutes.length > 0) {
  fail(`sidebar routes without a mounted view: ${unmountedRoutes.join(", ")}`);
}

const frontendCommands = collectLiteralFrontendInvokes(root);
const registeredCommands = extractRegisteredTauriCommands(read("src-tauri/src/lib.rs"));
const unregisteredCommands = findUnregisteredInvokes(frontendCommands, registeredCommands);
if (unregisteredCommands.length > 0) {
  fail(`frontend invoke commands absent from the Tauri handler: ${unregisteredCommands.join(", ")}`);
}

const deadButtons = collectDeadFrontendButtons(root);
if (deadButtons.length > 0) {
  fail(`enabled buttons without an action handler: ${deadButtons.join(", ")}`);
}

console.log(
  `UI wiring OK: ${sidebarRoutes.size} sidebar routes mounted; ` +
    `${frontendCommands.size} literal frontend invoke commands registered; ` +
    "all enabled buttons have handlers.",
);
