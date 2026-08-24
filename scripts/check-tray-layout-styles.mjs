#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const styles = readFileSync(resolve("src/styles.css"), "utf8");
const trayShell = readFileSync(resolve("src/styles/tray-shell.css"), "utf8");

const requiredSnippets = [
  ".tray-content--repo-intelligence {",
  "scroll-padding-top: 18px;",
  "padding-top: max(18px, env(safe-area-inset-top));",
];

const missing = requiredSnippets.filter((snippet) => !styles.includes(snippet));

if (!trayShell.includes("grid-template-columns: clamp(208px, 22vw, 248px) minmax(0, 1fr)") || !trayShell.includes("width: 100%;\n  min-width: 0;")) {
  console.error("Tray layout style guard failed. tray-shell.css must own desktop geometry and sidebar width.");
  process.exit(1);
}

if (styles.includes("104px minmax(0, 1fr)") || styles.includes("width: 104px")) {
  console.error("Tray layout style guard failed. stale 104px shell geometry remains in styles.css.");
  process.exit(1);
}

if (missing.length > 0) {
  console.error("Tray layout style guard failed. Missing snippets:");
  for (const snippet of missing) {
    console.error(`- ${JSON.stringify(snippet)}`);
  }
  process.exit(1);
}

console.log("Tray layout style guard passed.");
