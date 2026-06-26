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
const secretFileNames = new Set([
  ".env",
  ".env.local",
  ".env.production",
  ".npmrc",
  ".pypirc",
  "id_rsa",
  "id_ed25519",
]);
const secretPathPatterns = [
  /(^|\/)\.secrets?\//,
  /(^|\/)secrets?\//,
  /(^|\/)private_keys?\//,
  /(^|\/)\.private_keys?\//,
  /(^|\/)authkey_[^/]+\.p8$/i,
  /\.(pem|p8|p12|key|crt|cer)$/i,
];

const repoAgentRecipeTemplates = [
  {
    id: "cli_implementation",
    label: "CLI implementation handoff",
tools: ["Claude Code", "Gemini CLI", "OpenCode", "Aider", "Goose", "Qwen Code"],
    packIds: ["implementation"],
    instruction:
      "Copy the implementation pack into the CLI agent before asking for feature or bug-fix work.",
  },
  {
    id: "cli_verification",
    label: "CLI verification handoff",
tools: ["Codex", "Gemini CLI", "OpenCode", "Aider", "Goose", "Amazon Q Developer CLI"],
    packIds: ["verification"],
    instruction:
      "Copy the verification pack into the CLI agent before asking for test, build, or release checks.",
  },
  {
    id: "editor_context",
    label: "Editor assistant context",
tools: ["Cursor", "Continue", "Windsurf", "Zed AI"],
    packIds: ["implementation", "handoff"],
    instruction:
      "Use these packs as read-only context in editor assistants while provider routing remains manual.",
  },
];

const agentHandoffProfiles = [
  {
    id: "claude",
    label: "Claude Code",
    toolKind: "cli",
    defaultPackId: "implementation",
    guidance:
      "Paste before task in Claude Code when you want bounded repo context without re-scanning the whole tree.",
  },
  {
    id: "codex",
    label: "Codex",
    toolKind: "cli",
    defaultPackId: "verification",
    guidance:
      "Paste before Codex verification or implementation work to avoid repeated broad repo discovery.",
  },
  {
    id: "gemini",
    label: "Gemini CLI",
    toolKind: "cli",
    defaultPackId: "implementation",
    guidance: "Paste this before the task. Keep provider routing manual.",
  },
  {
    id: "opencode",
    label: "OpenCode",
    toolKind: "cli",
    defaultPackId: "implementation",
    guidance: "Paste this into the session as bounded repo context before editing.",
  },
  {
    id: "aider",
    label: "Aider",
    toolKind: "cli",
    defaultPackId: "implementation",
    guidance: "Use this to choose files intentionally before adding them to an Aider chat.",
  },
  {
    id: "goose",
    label: "Goose",
    toolKind: "cli",
    defaultPackId: "verification",
    guidance: "Use this for test, build, and release-check tasks with minimal context.",
  },
  {
    id: "cursor",
    label: "Cursor",
    toolKind: "editor",
    defaultPackId: "handoff",
    guidance: "Paste into the editor assistant as read-only project context.",
  },
  {
    id: "continue",
    label: "Continue",
    toolKind: "editor",
    defaultPackId: "handoff",
    guidance: "Paste into Continue chat as read-only context; do not auto-write config.",
  },
{
    id: "grok",
    label: "Grok / xAI CLI",
    toolKind: "chat",
    defaultPackId: "implementation",
    guidance: "Use this as compact task context where local CLI integration remains manual.",
  },
  {
    id: "qwen",
    label: "Qwen Code",
    toolKind: "cli",
    defaultPackId: "implementation",
    guidance: "Paste into Qwen Code as bounded repo context; keep provider and account routing manual.",
  },
  {
    id: "amazonq",
    label: "Amazon Q Developer CLI",
    toolKind: "cli",
    defaultPackId: "verification",
    guidance:
      "Paste verification packs for build, test, and AWS-adjacent repo questions without exposing account state.",
  },
  {
    id: "windsurf",
    label: "Windsurf",
    toolKind: "editor",
    defaultPackId: "handoff",
    guidance:
      "Paste into Windsurf chat as read-only project context; do not auto-write editor provider settings.",
  },
  {
    id: "zed",
    label: "Zed AI",
    toolKind: "editor",
    defaultPackId: "handoff",
    guidance: "Paste into Zed assistant as read-only context while model/provider selection stays manual.",
  },
];

function parseArgs(argv) {
  const options = {
    repoRoot: process.cwd(),
    packId: null,
    agent: null,
    format: "json",
    formatProvided: false,
    listPacks: false,
    listAgents: false,
    manifest: false,
  };
  const positional = [];

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--pack") {
      options.packId = argv[index + 1] ?? null;
      index += 1;
    } else if (arg.startsWith("--pack=")) {
      options.packId = arg.slice("--pack=".length);
    } else if (arg === "--agent") {
      options.agent = argv[index + 1] ?? null;
      index += 1;
    } else if (arg.startsWith("--agent=")) {
      options.agent = arg.slice("--agent=".length);
    } else if (arg === "--format") {
      options.format = argv[index + 1] ?? "json";
      options.formatProvided = true;
      index += 1;
    } else if (arg.startsWith("--format=")) {
      options.format = arg.slice("--format=".length);
      options.formatProvided = true;
    } else if (arg === "--list-packs") {
      options.listPacks = true;
    } else if (arg === "--list-agents") {
      options.listAgents = true;
    } else if (arg === "--manifest") {
      options.manifest = true;
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
  --agent <id>         Print agent handoff: claude, codex, gemini, opencode, aider, goose, cursor, continue, grok, qwen, amazonq, windsurf, zed
  --format <format>   json or markdown
  --list-packs        Print available pack ids
  --list-agents       Print available agent handoff ids
  --manifest          Print agent-readable pack manifest JSON
  --help              Show this help

Examples:
  npm run repo:intelligence -- .
  npm run repo:intelligence -- . --manifest
  npm run repo:intelligence -- . --list-agents
  npm run repo:intelligence -- . --pack implementation --format markdown
  npm run repo:intelligence -- . --agent codex --format markdown
  npm run repo:intelligence -- . --agent gemini --format json`);
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

function isSecretLikePath(filePath) {
  const normalized = filePath.replace(/\\/g, "/");
  const name = normalized.split("/").pop()?.toLowerCase() ?? normalized.toLowerCase();
  return (
    secretFileNames.has(name) ||
    name.startsWith(".env.") ||
    secretPathPatterns.some((pattern) => pattern.test(normalized))
  );
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
    includeByDefault: role !== "asset" && role !== "lockfile" && !isSecretLikePath(filePath),
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

function buildGraphSummary(files) {
  const included = files.filter((file) => file.includeByDefault);
  const sourceAndConfig = included.filter(
    (file) => file.role === "source" || file.role === "config",
  );
  const importEdges = buildGraphEdges(included);
  return {
    topDirectories: summarizeGraphNodes(included, (file) => topDirectory(file.path), 6),
    topLanguages: summarizeGraphNodes(
      included.filter((file) => file.language !== "Unknown"),
      (file) => file.language,
      6,
    ),
    entrypoints: sourceAndConfig.filter(isLikelyEntrypoint).slice(0, 12),
    likelyTests: included.filter((file) => file.role === "test").slice(0, 12),
    configHubs: included.filter((file) => file.role === "config").slice(0, 12),
    dependencyHubs: files.filter(isDependencyHub).slice(0, 12),
    importEdges,
    reverseDependencyHubs: buildReverseDependencyHubs(included, importEdges),
  };
}

function buildGraphEdges(files) {
  const byPath = new Map(files.map((file) => [file.path, file]));
  const dependencyHubs = files.filter(isDependencyHub);
  const configHubs = files.filter((file) => file.role === "config");
  const edges = [];
  const pushEdge = (edge) => {
    if (edge.from === edge.to || edges.length >= 24) return;
    if (edges.some((existing) => existing.from === edge.from && existing.to === edge.to && existing.kind === edge.kind)) return;
    edges.push(edge);
  };
  for (const file of files) {
    if (file.role === "test") {
      const target = findTestTarget(file, byPath);
      if (target) pushEdge({ from: file.path, to: target.path, kind: "test_to_source", reason: "test filename matches source module" });
    }
    if (isLikelyEntrypoint(file)) {
      const config = findNearestConfigHub(file, configHubs);
      if (config) pushEdge({ from: file.path, to: config.path, kind: "entrypoint_to_config", reason: "entrypoint shares closest config surface" });
    }
    if (file.role === "source") {
      const dependencyHub = findNearestDependencyHub(file, dependencyHubs);
      if (dependencyHub) pushEdge({ from: file.path, to: dependencyHub.path, kind: "source_to_dependency_hub", reason: "source file belongs to dependency hub scope" });
    }
  }
  return edges;
}

function buildReverseDependencyHubs(files, edges) {
  const byPath = new Map(files.map((file) => [file.path, file]));
  const inbound = new Map();
  for (const edge of edges) {
    const target = byPath.get(edge.to);
    const node = inbound.get(edge.to) ?? { label: edge.to, count: 0, estimatedTokens: target?.estimatedTokens ?? 0, examples: [] };
    node.count += 1;
    if (node.examples.length < 4) node.examples.push(edge.from);
    inbound.set(edge.to, node);
  }
  return [...inbound.values()]
    .sort((a, b) => b.count - a.count || b.estimatedTokens - a.estimatedTokens || a.label.localeCompare(b.label))
    .slice(0, 12);
}

function findTestTarget(testFile, byPath) {
  return testTargetCandidates(testFile.path).map((candidate) => byPath.get(candidate)).find(Boolean);
}

function testTargetCandidates(filePath) {
  const extension = extensionForPath(filePath);
  const withoutExtension = extension ? filePath.slice(0, -extension.length) : filePath;
  const base = withoutExtension.replace(/\.(test|spec)$/i, "");
  if (base === withoutExtension) return [];
  const extensions = [extension, ".tsx", ".ts", ".jsx", ".js", ".rs"].filter(Boolean);
  return [...new Set(extensions.map((candidateExtension) => `${base}${candidateExtension}`))];
}

function findNearestConfigHub(file, configHubs) {
  return nearestScopedFile(file, configHubs) ?? configHubs.find((candidate) => !candidate.path.includes("/"));
}

function findNearestDependencyHub(file, dependencyHubs) {
  return nearestScopedFile(file, dependencyHubs) ?? dependencyHubs.find((candidate) => !candidate.path.includes("/"));
}

function nearestScopedFile(file, candidates) {
  return candidates
    .filter((candidate) => candidate.path !== file.path)
    .map((candidate) => ({ candidate, score: sharedPathPrefixScore(file.path, candidate.path) }))
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score || a.candidate.path.split("/").length - b.candidate.path.split("/").length || a.candidate.path.localeCompare(b.candidate.path))[0]?.candidate;
}

function sharedPathPrefixScore(left, right) {
  const leftParts = left.split("/");
  const rightParts = right.split("/");
  let score = 0;
  while (leftParts[score] && leftParts[score] === rightParts[score]) score += 1;
  if (!right.includes("/") && leftParts.length > 1) return 1;
  return score;
}

function extensionForPath(filePath) {
  const name = filePath.split("/").pop() ?? filePath;
  const dot = name.lastIndexOf(".");
  return dot >= 0 ? name.slice(dot) : "";
}


function summarizeGraphNodes(files, labelForFile, limit) {
  const nodes = new Map();
  for (const file of files) {
    const label = labelForFile(file);
    const node = nodes.get(label) ?? {
      label,
      count: 0,
      estimatedTokens: 0,
      examples: [],
    };
    node.count += 1;
    node.estimatedTokens += file.estimatedTokens;
    if (node.examples.length < 4) node.examples.push(file.path);
    nodes.set(label, node);
  }
  return [...nodes.values()]
    .sort(
      (a, b) =>
        b.count - a.count ||
        b.estimatedTokens - a.estimatedTokens ||
        a.label.localeCompare(b.label),
    )
    .slice(0, limit);
}

function topDirectory(filePath) {
  const [first, second] = filePath.split("/");
  return second ? first : ".";
}

function isDependencyHub(file) {
  const name = file.path.split("/").pop()?.toLowerCase() ?? file.path.toLowerCase();
  return (
    file.role === "lockfile" ||
    name === "package.json" ||
    name === "pyproject.toml" ||
    name === "requirements.txt" ||
    name === "cargo.toml" ||
    name === "go.mod" ||
    name === "gemfile" ||
    name === "podfile"
  );
}


function isLikelyEntrypoint(file) {
  const normalized = file.path.toLowerCase();
  const name = normalized.split("/").pop() ?? normalized;
  return (
    file.role === "source" &&
    [
      "main.ts",
      "main.tsx",
      "main.js",
      "index.ts",
      "index.tsx",
      "index.js",
      "app.tsx",
      "app.ts",
      "lib.rs",
      "main.rs",
    ].includes(name)
  );
}

function formatGraphMarkdown(graph) {
  if (!graph) return "";
  const lines = ["## Repo Graph Summary"];
  const directories = graph.topDirectories.map(
    (node) => "- " + node.label + ": " + node.count + " files, ~" + node.estimatedTokens.toLocaleString() + " tokens",
  );
  const languages = graph.topLanguages.map((node) => "- " + node.label + ": " + node.count + " files");
  const entrypoints = graph.entrypoints.map((file) => "- " + file.path + " (" + file.language + ")");
  const tests = graph.likelyTests.map((file) => "- " + file.path);
  const config = graph.configHubs.map((file) => "- " + file.path);
  const dependencies = (graph.dependencyHubs ?? []).map((file) => "- " + file.path);
  const importEdges = (graph.importEdges ?? []).map((edge) => "- " + edge.from + " -> " + edge.to + " (" + edge.kind + ": " + edge.reason + ")");
  const reverseDependencyHubs = (graph.reverseDependencyHubs ?? []).map((node) => "- " + node.label + ": " + node.count + " inbound links");
  if (directories.length) lines.push("", "Top directories", ...directories);
  if (languages.length) lines.push("", "Top languages", ...languages);
  if (entrypoints.length) lines.push("", "Likely entrypoints", ...entrypoints);
  if (tests.length) lines.push("", "Likely tests", ...tests);
  if (config.length) lines.push("", "Config hubs", ...config);
  if (dependencies.length) lines.push("", "Dependency hubs", ...dependencies);
  if (importEdges.length) lines.push("", "Import and dependency edges", ...importEdges.slice(0, 8));
  if (reverseDependencyHubs.length) lines.push("", "Reverse dependency hubs", ...reverseDependencyHubs.slice(0, 8));
  return lines.join("\n");
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
    graph: buildGraphSummary(indexable),
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
    formatGraphMarkdown(summary.graph),
    "",
    "## Files",
    ...files,
  ].join("\n");
}

function formatAgentHandoffMarkdown(summary, agentId, requestedPackId) {
  const profile = agentHandoffProfiles.find((candidate) => candidate.id === agentId);
  if (!profile) {
    throw new Error(
      `Unknown agent: ${agentId}. Available agents: ${agentHandoffProfiles
        .map((candidate) => candidate.id)
        .join(", ")}`,
    );
  }

  const selectedPack =
    summary.packs.find(
      (contextPack) => contextPack.id === (requestedPackId ?? profile.defaultPackId),
    ) ??
    summary.packs.find((contextPack) => contextPack.id === profile.defaultPackId) ??
    summary.packs[0];

  if (!selectedPack) {
    throw new Error("No repo intelligence packs are available.");
  }

  return [
    `# ${profile.label} Handoff`,
    "",
    `Repository: ${summary.repoRoot}`,
    `Tool kind: ${profile.toolKind}`,
    `Selected pack: ${selectedPack.title}`,
    `Estimated context tokens: ${selectedPack.estimatedTokens.toLocaleString()}`,
    `Estimated tokens avoided: ${Math.max(
      0,
      summary.estimatedFullScanTokens - selectedPack.estimatedTokens,
    ).toLocaleString()}`,
    "",
    "## Use",
    profile.guidance,
    "Treat this as read-only planning context unless the user explicitly asks for edits.",
    "Secret-like paths and generated folders are excluded from this handoff.",
    "",
    formatSinglePackMarkdown(summary, selectedPack),
  ].join("\n");
}

function buildAgentHandoffPayload(summary, agentId, requestedPackId) {
  const profile = agentHandoffProfiles.find((candidate) => candidate.id === agentId);
  if (!profile) {
    throw new Error(
      `Unknown agent: ${agentId}. Available agents: ${agentHandoffProfiles
        .map((candidate) => candidate.id)
        .join(", ")}`,
    );
  }

  const selectedPack =
    summary.packs.find(
      (contextPack) => contextPack.id === (requestedPackId ?? profile.defaultPackId),
    ) ??
    summary.packs.find((contextPack) => contextPack.id === profile.defaultPackId) ??
    summary.packs[0];

  if (!selectedPack) {
    throw new Error("No repo intelligence packs available.");
  }

  return {
    schemaVersion: 1,
    kind: "mac_ai_switchboard.repo_agent_handoff",
    repoRoot: summary.repoRoot,
    agent: {
      id: profile.id,
      label: profile.label,
      toolKind: profile.toolKind,
      guidance: profile.guidance,
    },
    pack: {
      id: selectedPack.id,
      title: selectedPack.title,
      purpose: selectedPack.purpose,
      estimatedTokens: selectedPack.estimatedTokens,
      estimatedTokensAvoided: Math.max(
        0,
        summary.estimatedFullScanTokens - selectedPack.estimatedTokens,
      ),
      savingsVsFullScanPct: selectedPack.savingsVsFullScanPct,
      files: selectedPack.files.map((file) => ({
        path: file.path,
        role: file.role,
        language: file.language,
        estimatedTokens: file.estimatedTokens,
        reasons: file.reasons,
      })),
    },
    graph: {
      available: Boolean(summary.graph),
      dependencyHubs: summary.graph?.dependencyHubs ?? [],
      importEdges: summary.graph?.importEdges ?? [],
      reverseDependencyHubs: summary.graph?.reverseDependencyHubs ?? [],
    },
    safety: {
      readOnly: true,
      excludesSecretLikePaths: true,
      modifiesRepository: false,
      manualProviderRouting: true,
    },
  };
}

function buildAgentManifest(summary) {
  const fullScanTokens = summary.estimatedFullScanTokens;
  return {
    schemaVersion: 1,
    kind: "mac_ai_switchboard.repo_intelligence_manifest",
    repoRoot: summary.repoRoot,
    generatedAt: new Date().toISOString(),
    totals: {
      totalFiles: summary.totalFiles,
      indexedFiles: summary.indexedFiles,
      estimatedFullScanTokens: fullScanTokens,
      roleCounts: summary.roleCounts,
    },
    graph: {
      available: Boolean(summary.graph),
      topDirectories: summary.graph?.topDirectories ?? [],
      topLanguages: summary.graph?.topLanguages ?? [],
      entrypointCount: summary.graph?.entrypoints.length ?? 0,
      likelyTestCount: summary.graph?.likelyTests.length ?? 0,
      configHubCount: summary.graph?.configHubs.length ?? 0,
      dependencyHubCount: summary.graph?.dependencyHubs?.length ?? 0,
      importEdgeCount: summary.graph?.importEdges?.length ?? 0,
      reverseDependencyHubCount: summary.graph?.reverseDependencyHubs?.length ?? 0,
      importEdges: summary.graph?.importEdges ?? [],
      reverseDependencyHubs: summary.graph?.reverseDependencyHubs ?? [],
    },    packs: summary.packs.map((contextPack) => ({
      id: contextPack.id,
      title: contextPack.title,
      purpose: contextPack.purpose,
      fileCount: contextPack.files.length,
      estimatedTokens: contextPack.estimatedTokens,
      estimatedTokensAvoided: Math.max(
        0,
        fullScanTokens - contextPack.estimatedTokens,
      ),
      savingsVsFullScanPct: contextPack.savingsVsFullScanPct,
      command: `npm run repo:intelligence -- ${JSON.stringify(summary.repoRoot)} --pack ${contextPack.id} --format markdown`,
    })),
    agentRecipes: repoAgentRecipeTemplates.map((recipe) => ({
      ...recipe,
      command: `npm run repo:intelligence -- ${JSON.stringify(summary.repoRoot)} --pack ${recipe.packIds[0]} --format markdown`,
    })),
    safety: {
      readOnly: true,
      excludesSecretLikePaths: true,
      modifiesRepository: false,
    },
  };
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

if (options.listAgents) {
  console.log(agentHandoffProfiles.map((profile) => profile.id).join("\n"));
  process.exit(0);
}

if (options.manifest) {
  console.log(JSON.stringify(buildAgentManifest(summary), null, 2));
  process.exit(0);
}

if (options.agent) {
  try {
    if (options.formatProvided && options.format === "json") {
      console.log(
        JSON.stringify(
          buildAgentHandoffPayload(summary, options.agent, options.packId),
          null,
          2,
        ),
      );
    } else {
      console.log(formatAgentHandoffMarkdown(summary, options.agent, options.packId));
    }
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
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
