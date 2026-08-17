#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

function fail(message) {
  console.error(`Design system check failed: ${message}`);
  process.exit(1);
}

const main = read("src/main.tsx");
const designSystem = read("src/styles/design-system.css");
const activationCard = read("src/components/MasterActivationCard.tsx");
const doctrine = read("docs/design-system.md");

const themeImport = main.indexOf('import "./switchboard-theme.css";');
const contractImport = main.indexOf('import "./styles/design-system.css";');
if (themeImport < 0 || contractImport < 0 || contractImport < themeImport) {
  fail("design-system.css must be imported after switchboard-theme.css");
}

for (const token of [
  "--surface-elevated",
  "--surface-muted",
  "--text-primary",
  "--text-secondary",
  "--control-accent-ink",
  "--focus-ring",
]) {
  if (!designSystem.includes(`${token}:`)) {
    fail(`missing canonical token ${token}`);
  }
}

if (!designSystem.includes("color: var(--control-accent-ink)")) {
  fail("brass primary controls must use the dark accent-ink token");
}

if (!designSystem.includes(".intro-shell--post-install")) {
  fail("launcher and onboarding surfaces must inherit the product theme");
}

if (/<style[>\s]/.test(activationCard)) {
  fail("MasterActivationCard must not inject CSP-blocked runtime styles");
}

for (const section of [
  "## Color roles",
  "## Typography and spacing",
  "## Component behavior",
  "## Surface and contrast rules",
  "## Review checklist",
]) {
  if (!doctrine.includes(section)) {
    fail(`design doctrine is missing ${section}`);
  }
}

console.log("Design system OK: canonical tokens, import order, CSP-safe styles, control contrast, and doctrine verified.");
