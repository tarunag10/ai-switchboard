#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const styles = readFileSync(resolve("src/styles.css"), "utf8");

const requiredSnippets = [
  ".tray-sidebar {\n  width: 104px;",
  ".tray-shell {\n  grid-template-columns: 104px minmax(0, 1fr);",
  "grid-template-columns: 104px minmax(0, calc(100vw - 104px));",
  ".tray-content--repo-intelligence {",
  "scroll-padding-top: 18px;",
  "padding-top: max(18px, env(safe-area-inset-top));",
];

const missing = requiredSnippets.filter((snippet) => !styles.includes(snippet));

if (missing.length > 0) {
  console.error("Tray layout style guard failed. Missing snippets:");
  for (const snippet of missing) {
    console.error(`- ${JSON.stringify(snippet)}`);
  }
  process.exit(1);
}

console.log("Tray layout style guard passed.");
