#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = process.argv[2] ? path.resolve(process.argv[2]) : process.cwd();
const ignoredSegments = new Set([
  ".git",
  "node_modules",
  "dist",
  "build",
  "coverage",
  "target",
  ".next",
  ".turbo",
]);

const languageByExtension = {
  ".css": "CSS",
  ".html": "HTML",
  ".js": "JavaScript",
  ".json": "JSON",
  ".jsx": "React",
  ".md": "Markdown",
  ".mjs": "JavaScript",
  ".rs": "Rust",
  ".sh": "Shell",
  ".toml": "TOML",
  ".ts": "TypeScript",
  ".tsx": "React",
  ".yml": "YAML",
  ".yaml": "YAML",
};

const lockfileNames = new Set([
  "Cargo.lock",
  "package-lock.json",
  "pnpm-lock.yaml",
  "yarn.lock",
  "bun.lockb",
]);

function walk(dir, files = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (ignoredSegments.has(entry.name)) continue;
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(absolute, files);
    } else if (entry.isFile()) {
      const relative = path.relative(repoRoot, absolute).split(path.sep).join("/");
      const stat = fs.statSync(absolute);
      files.push({ path: relative, bytes: stat.size });
    }
  }
  return files;
}

function estimateTokens(bytes) {
  return Math.max(1, Math.ceil(bytes / 4));
}

function classify(filePath, bytes) {
  const name = filePath.split("/").pop() ?? filePath;
  const lower = filePath.toLowerCase();
  const extension = path.extname(name).toLowerCase();
  let role = "unknown";

  if (lockfileNames.has(name)) {
    role = "lockfile";
  } else if (lower.includes(".test.") || lower.includes(".spec.") || lower.includes("/tests/")) {
    role = "test";
  } else if (lower.endsWith(".md") || lower.startsWith("docs/") || lower.includes("/docs/")) {
    role = "docs";
  } else if (
    name.startsWith(".") ||
    lower.endsWith(".toml") ||
    lower.endsWith(".json") ||
    lower.endsWith(".yml") ||
    lower.endsWith(".yaml")
  ) {
    role = "config";
  } else if ([".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".webp"].includes(extension)) {
    role = "asset";
  } else if (languageByExtension[extension]) {
    role = "source";
  }

  return {
    path: filePath,
    role,
    language: languageByExtension[extension] ?? "Unknown",
    estimatedTokens: estimateTokens(bytes),
  };
}

function pack(id, title, purpose, files, fullScanTokens) {
  const selected = [...files]
    .sort((a, b) => a.estimatedTokens - b.estimatedTokens || a.path.localeCompare(b.path))
    .slice(0, 40);
  const estimatedTokens = selected.reduce((sum, file) => sum + file.estimatedTokens, 0);
  const savingsVsFullScanPct =
    fullScanTokens > 0
      ? Math.max(0, Math.round((1 - estimatedTokens / fullScanTokens) * 1000) / 10)
      : 0;

  return { id, title, purpose, estimatedTokens, savingsVsFullScanPct, files: selected };
}

const signals = walk(repoRoot).map((file) => classify(file.path, file.bytes));
const indexable = signals.filter((file) =>
  ["source", "test", "config", "docs"].includes(file.role),
);
const estimatedFullScanTokens = signals.reduce((sum, file) => sum + file.estimatedTokens, 0);
const roleCounts = signals.reduce((counts, file) => {
  counts[file.role] = (counts[file.role] ?? 0) + 1;
  return counts;
}, {});

const summary = {
  repoRoot,
  totalFiles: signals.length,
  indexedFiles: indexable.length,
  estimatedFullScanTokens,
  roleCounts,
  packs: [
    pack(
      "implementation",
      "Implementation Pack",
      "Source files likely needed for feature work.",
      indexable.filter((file) => file.role === "source" || file.role === "config"),
      estimatedFullScanTokens,
    ),
    pack(
      "verification",
      "Verification Pack",
      "Tests, scripts, and config likely needed before committing.",
      indexable.filter((file) => file.role === "test" || file.role === "config"),
      estimatedFullScanTokens,
    ),
    pack(
      "handoff",
      "Handoff Pack",
      "Docs and project metadata useful for another agent or maintainer.",
      indexable.filter((file) => file.role === "docs" || file.role === "config"),
      estimatedFullScanTokens,
    ),
  ],
};

console.log(JSON.stringify(summary, null, 2));
