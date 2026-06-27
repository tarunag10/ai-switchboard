import fs from "node:fs";
import path from "node:path";

const roots = ["src", "src-tauri"];
const blockedPathFragments = ["logo" + "ipsum", "headroom-" + "logo.svg"];
const blockedTextPatterns = [
  new RegExp("\\b" + "logo" + "ipsum" + "\\b", "i"),
  new RegExp("headroom-" + "logo\\.svg", "i"),
];
const ignoredDirs = new Set(["node_modules", "dist", "target", ".git"]);

function walk(dir, files = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!ignoredDirs.has(entry.name)) {
        walk(fullPath, files);
      }
    } else {
      files.push(fullPath);
    }
  }
  return files;
}

const failures = [];

for (const root of roots) {
  if (!fs.existsSync(root)) {
    continue;
  }
  for (const file of walk(root)) {
    const normalized = file.replace(/\\/g, "/");
    for (const fragment of blockedPathFragments) {
      if (normalized.toLowerCase().includes(fragment.toLowerCase())) {
        failures.push(`Blocked inherited logo path: ${normalized}`);
      }
    }
    if (!/\.(ts|tsx|js|mjs|json|md|html|css|toml|rs|sh)$/i.test(file)) {
      continue;
    }
    const content = fs.readFileSync(file, "utf8");
    for (const pattern of blockedTextPatterns) {
      if (pattern.test(content)) {
        failures.push(`Blocked inherited logo reference in ${normalized}: ${pattern}`);
      }
    }
  }
}

if (!fs.existsSync("docs/asset-provenance.md")) {
  failures.push("Missing docs/asset-provenance.md");
} else {
  const provenance = fs.readFileSync("docs/asset-provenance.md", "utf8");
  for (const phrase of [
    "ChatGPT image generation",
    "src/assets/",
    "src-tauri/icons/",
  ]) {
    if (!provenance.includes(phrase)) {
      failures.push(`docs/asset-provenance.md missing ${phrase}`);
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("Branding assets guard passed.");
