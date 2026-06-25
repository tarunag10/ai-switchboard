#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

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

function parseArgs(argv) {
  const options = {
    repoRoot: process.cwd(),
    packId: null,
    format: "json",
    listPacks: false,
  };
  const positional = [];

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--pack") {
      options.packId = argv[index + 1] ?? null;
      index += 1;
    } else if (arg.startsWith("--pack=")) {
      options.packId = arg.slice("--pack=".length);
    } else if (arg === "--format") {
      options.format = argv[index + 1] ?? "json";
      index += 1;
    } else if (arg.startsWith("--format=")) {
      options.format = arg.slice("--format=".length);
    } else if (arg === "--list-packs") {
      options.listPacks = true;
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      positional.push(arg);
    }
  }

  if (positional[0]) {
    options.repoRoot = path.resolve(positional[0]);
  }

  return options;
}

function printHelp() {
  console.log(`Usage: npm run repo:intelligence -- [repo-path] [options]

Options:
  --pack <id>          Print one context pack: implementation, verification, handoff
  --format <format>   json or markdown
  --list-packs        Print available pack ids
  --help              Show this help

Examples:
  npm run repo:intelligence -- .
  npm run repo:intelligence -- . --pack implementation --format markdown`);
}

function walk(repoRoot, dir = repoRoot, files = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (ignoredSegments.has(entry.name)) continue;
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(repoRoot, absolute, files);
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

  if (lockfileNames.has(name)) role = "lockfile";
  else if (lower.includes(".test.") || lower.includes(".spec.") || lower.includes("/tests/")) role = "test";
  else if (lower.startsWith("docs/") || extension === ".md") role = "docs";
  else if ([".json", ".toml", ".yaml", ".yml", ".sh"].includes(extension)) role = "config";
  else if ([".ts", ".tsx", ".js", ".jsx", ".rs", ".css", ".html"].includes(extension)) role = "source";
  else if ([".svg", ".png", ".jpg", ".jpeg", ".gif", ".ico", ".webp"].includes(extension)) role = "asset";

  return {
    path: filePath,
    role,
    language: languageByExtension[extension] ?? "Unknown",
    estimatedTokens: estimateTokens(bytes),
    includeByDefault: role !== "asset" && role !== "lockfile",
  };
}

function pack(id, title, purpose, files, estimatedFullScanTokens) {
  const sorted = [...files]
    .sort((a, b) => a.estimatedTokens - b.estimatedTokens || a.path.localeCompare(b.path))
    .slice(0, 40);
  const estimatedTokens = sorted.reduce((sum, file) => sum + file.estimatedTokens, 0);
  const savingsVsFullScanPct =
    estimatedFullScanTokens > 0
      ? Math.max(0, Math.round((1 - estimatedTokens / estimatedFullScanTokens) * 1000) / 10)
      : 0;

  return {
    id,
    title,
    purpose,
    estimatedTokens,
    savingsVsFullScanPct,
    files: sorted,
  };
}

function buildSummary(repoRoot) {
  const files = walk(repoRoot);
  const signals = files.map((file) => classify(file.path, file.bytes));
  const indexable = signals.filter((file) => file.includeByDefault);
  const estimatedFullScanTokens = signals.reduce((sum, file) => sum + file.estimatedTokens, 0);
  const roleCounts = signals.reduce((counts, file) => {
    counts[file.role] = (counts[file.role] ?? 0) + 1;
    return counts;
  }, {});

  return {
    repoRoot,
    totalFiles: signals.length,
    indexedFiles: indexable.length,
    estimatedFullScanTokens,
    roleCounts,
    packs: [
      pack(
        "implementation",
        "Implementation Pack",
        "Source files likely needed feature work.",
        indexable.filter((file) => file.role === "source" || file.role === "config"),
        estimatedFullScanTokens,
      ),
      pack(
        "verification",
        "Verification Pack",
        "Tests, scripts, config likely needed before committing.",
        indexable.filter((file) => file.role === "test" || file.role === "config"),
        estimatedFullScanTokens,
      ),
      pack(
        "handoff",
        "Handoff Pack",
        "Docs project metadata useful for another agent or maintainer.",
        indexable.filter((file) => file.role === "docs" || file.role === "config"),
        estimatedFullScanTokens,
      ),
    ],
  };
}

function formatSinglePackMarkdown(summary, selectedPack) {
  const files = selectedPack.files.map(
    (file) =>
      `- ${file.path} (${file.role}, ${file.language}, ~${file.estimatedTokens.toLocaleString()} tokens)`,
  );

  return [
    `# ${selectedPack.title}: ${summary.repoRoot}`,
    "",
    selectedPack.purpose,
    `Estimated full scan tokens: ${summary.estimatedFullScanTokens.toLocaleString()}`,
    `Estimated pack tokens: ${selectedPack.estimatedTokens.toLocaleString()}`,
    `Estimated tokens avoided: ${Math.max(0, summary.estimatedFullScanTokens - selectedPack.estimatedTokens).toLocaleString()}`,
    `Estimated savings vs full scan: ${selectedPack.savingsVsFullScanPct.toFixed(1)}%`,
    "",
    "## Files",
    ...files,
  ].join("\n");
}

const options = parseArgs(process.argv.slice(2));

if (!fs.existsSync(options.repoRoot) || !fs.statSync(options.repoRoot).isDirectory()) {
  console.error(`Repository path does not exist or is not a directory: ${options.repoRoot}`);
  process.exit(1);
}

if (!["json", "markdown"].includes(options.format)) {
  console.error(`Unsupported format: ${options.format}. Use json or markdown.`);
  process.exit(1);
}

const summary = buildSummary(options.repoRoot);

if (options.listPacks) {
  console.log(summary.packs.map((contextPack) => contextPack.id).join("\n"));
  process.exit(0);
}

if (options.packId) {
  const selectedPack = summary.packs.find((contextPack) => contextPack.id === options.packId);
  if (!selectedPack) {
    console.error(
      `Unknown pack: ${options.packId}. Available packs: ${summary.packs
        .map((contextPack) => contextPack.id)
        .join(", ")}`,
    );
    process.exit(1);
  }
  if (options.format === "markdown") {
    console.log(formatSinglePackMarkdown(summary, selectedPack));
  } else {
    console.log(JSON.stringify({ repoRoot: summary.repoRoot, pack: selectedPack }, null, 2));
  }
} else if (options.format === "markdown") {
  for (const contextPack of summary.packs) {
    console.log(formatSinglePackMarkdown(summary, contextPack));
    console.log("\n---\n");
  }
} else {
  console.log(JSON.stringify(summary, null, 2));
}
