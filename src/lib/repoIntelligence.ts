export type RepoFileRole =
  | "source"
  | "test"
  | "config"
  | "docs"
  | "asset"
  | "lockfile"
  | "generated"
  | "unknown";

export interface RepoFileSignal {
  path: string;
  role: RepoFileRole;
  language: string;
  estimatedTokens: number;
  includeByDefault: boolean;
  reasons: string[];
}

export interface RepoContextPack {
  id: string;
  title: string;
  purpose: string;
  files: RepoFileSignal[];
  estimatedTokens: number;
  savingsVsFullScanPct: number;
}

export interface RepoGraphNode {
  label: string;
  count: number;
  estimatedTokens: number;
  examples: string[];
}

export interface RepoGraphSummary {
  topDirectories: RepoGraphNode[];
  topLanguages: RepoGraphNode[];
  entrypoints: RepoFileSignal[];
  likelyTests: RepoFileSignal[];
  configHubs: RepoFileSignal[];
  dependencyHubs?: RepoFileSignal[];
}

export interface RepoIntelligenceSummary {
  indexedAt?: string;
  repoRoot?: string;
  totalFiles: number;
  indexedFiles: number;
  skippedFiles?: number;
  estimatedFullScanTokens: number;
  roleCounts: Record<RepoFileRole, number>;
  graph?: RepoGraphSummary;
  packs: RepoContextPack[];
}

export interface RepoSavingsEstimate {
  fullScanTokens: number;
  bestPackTokens: number;
  bestPackTokensAvoided: number;
  bestPackSavingsPct: number;
  allPacksTokens: number;
  allPacksTokensAvoided: number;
  allPacksSavingsPct: number;
  bestPack?: RepoContextPack;
}

export interface RepoAgentManifest {
  schemaVersion: 1;
  kind: "mac_ai_switchboard.repo_intelligence_manifest";
  repoRoot: string;
  generatedAt: string;
  totals: {
    totalFiles: number;
    indexedFiles: number;
    estimatedFullScanTokens: number;
    roleCounts: Record<RepoFileRole, number>;
  };
  graph: {
    available: boolean;
    topDirectories: RepoGraphNode[];
    topLanguages: RepoGraphNode[];
    entrypointCount: number;
    likelyTestCount: number;
    configHubCount: number;
    dependencyHubCount: number;
  };
  packs: Array<{
    id: string;
    title: string;
    purpose: string;
    fileCount: number;
    estimatedTokens: number;
    estimatedTokensAvoided: number;
    savingsVsFullScanPct: number;
    command: string;
  }>;
  safety: {
    readOnly: true;
    excludesSecretLikePaths: true;
    modifiesRepository: false;
  };
}

const generatedPathPatterns = [
  /(^|\/)dist\//,
  /(^|\/)build\//,
  /(^|\/)coverage\//,
  /(^|\/)node_modules\//,
  /(^|\/)target\//,
  /(^|\/)\.next\//,
  /(^|\/)\.turbo\//,
  /(^|\/)vendor\//,
];

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

const languageByExtension: Record<string, string> = {
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

export function estimateRepoTokens(bytes: number): number {
  return Math.max(1, Math.ceil(bytes / 4));
}

export function isSecretLikeRepoPath(path: string): boolean {
  const normalized = path.replace(/\\/g, "/");
  const name = normalized.split("/").pop()?.toLowerCase() ?? normalized.toLowerCase();
  return (
    secretFileNames.has(name) ||
    name.startsWith(".env.") ||
    secretPathPatterns.some((pattern) => pattern.test(normalized))
  );
}

export function classifyRepoFile(path: string, bytes = 0): RepoFileSignal {
  const normalized = path.replace(/\\/g, "/");
  const name = normalized.split("/").pop() ?? normalized;
  const lower = normalized.toLowerCase();
  const extensionMatch = name.match(/(\.[^.]+)$/);
  const extension = extensionMatch?.[1]?.toLowerCase() ?? "";
  const reasons: string[] = [];
  let role: RepoFileRole = "unknown";

  if (generatedPathPatterns.some((pattern) => pattern.test(normalized))) {
    role = "generated";
    reasons.push("generated or dependency output");
  } else if (lockfileNames.has(name)) {
    role = "lockfile";
    reasons.push("package lockfile");
  } else if (lower.includes(".test.") || lower.includes(".spec.") || lower.includes("/tests/")) {
    role = "test";
    reasons.push("test path");
  } else if (lower.endsWith(".md") || lower.startsWith("docs/") || lower.includes("/docs/")) {
    role = "docs";
    reasons.push("documentation");
  } else if (
    name.startsWith(".") ||
    lower.endsWith(".toml") ||
    lower.endsWith(".json") ||
    lower.endsWith(".yml") ||
    lower.endsWith(".yaml")
  ) {
    role = "config";
    reasons.push("configuration");
  } else if ([".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".webp"].includes(extension)) {
    role = "asset";
    reasons.push("static asset");
  } else if (languageByExtension[extension]) {
    role = "source";
    reasons.push("source file");
  }

  const estimatedTokens = estimateRepoTokens(bytes);
  const secretLike = isSecretLikeRepoPath(normalized);
  if (secretLike) {
    reasons.push("secret-like path excluded");
  }
  const includeByDefault =
    !secretLike &&
    (role === "source" || role === "test" || role === "config" || role === "docs");

  return {
    path: normalized,
    role,
    language: languageByExtension[extension] ?? "Unknown",
    estimatedTokens,
    includeByDefault,
    reasons,
  };
}

export function buildRepoIntelligenceSummary(
  files: Array<{ path: string; bytes?: number }>,
): RepoIntelligenceSummary {
  const signals = files.map((file) => classifyRepoFile(file.path, file.bytes ?? 0));
  const indexed = signals.filter((signal) => signal.includeByDefault);
  const estimatedFullScanTokens = signals.reduce(
    (sum, signal) => sum + signal.estimatedTokens,
    0,
  );
  const roleCounts = signals.reduce(
    (counts, signal) => {
      counts[signal.role] += 1;
      return counts;
    },
    {
      source: 0,
      test: 0,
      config: 0,
      docs: 0,
      asset: 0,
      lockfile: 0,
      generated: 0,
      unknown: 0,
    } satisfies Record<RepoFileRole, number>,
  );

  const graph = buildRepoGraphSummary(indexed);
  const packs = [
    buildContextPack(
      "implementation",
      "Implementation Pack",
      "Source files likely needed for feature work.",
      indexed.filter((signal) => signal.role === "source" || signal.role === "config"),
      estimatedFullScanTokens,
    ),
    buildContextPack(
      "verification",
      "Verification Pack",
      "Tests, scripts, and config likely needed before committing.",
      indexed.filter((signal) => signal.role === "test" || signal.role === "config"),
      estimatedFullScanTokens,
    ),
    buildContextPack(
      "handoff",
      "Handoff Pack",
      "Docs and project metadata useful for another agent or maintainer.",
      indexed.filter((signal) => signal.role === "docs" || signal.role === "config"),
      estimatedFullScanTokens,
    ),
  ];

  return {
    totalFiles: signals.length,
    indexedFiles: indexed.length,
    estimatedFullScanTokens,
    roleCounts,
    graph,
    packs,
  };
}

export function formatRepoContextPackMarkdown(summary: RepoIntelligenceSummary): string {
  const title = summary.repoRoot
    ? `# Repo Intelligence Context Pack: ${summary.repoRoot}`
    : "# Repo Intelligence Context Pack";
  const indexedAt = summary.indexedAt ? `\nIndexed at: ${summary.indexedAt}` : "";
  const overview = [
    title,
    indexedAt.trim(),
    "",
    `Files scanned: ${summary.totalFiles}`,
    `Indexed signals: ${summary.indexedFiles}`,
    `Estimated full scan tokens: ${summary.estimatedFullScanTokens.toLocaleString()}`,
    "",
  ].filter(Boolean);
  const graphSection = formatRepoGraphMarkdown(summary.graph);

  const packSections = summary.packs.map((pack) => {
    const files = pack.files
      .slice(0, 12)
      .map(
        (file) =>
          `- ${file.path} (${file.role}, ${file.language}, ~${file.estimatedTokens.toLocaleString()} tokens)`,
      );
    return [
      `## ${pack.title}`,
      pack.purpose,
      `Estimated pack tokens: ${pack.estimatedTokens.toLocaleString()}`,
      `Estimated savings vs full scan: ${pack.savingsVsFullScanPct.toFixed(1)}%`,
      "",
      ...files,
    ].join("\n");
  });

  return [...overview, graphSection, ...packSections].filter(Boolean).join("\n\n").trim();
}

export function formatSingleRepoContextPackMarkdown(
summary: RepoIntelligenceSummary,
pack: RepoContextPack,
): string {
  const title = summary.repoRoot
    ? `# ${pack.title}: ${summary.repoRoot}`
    : `# ${pack.title}`;
  const indexedAt = summary.indexedAt ? `Indexed at: ${summary.indexedAt}` : null;
  const files = pack.files.slice(0, 40).map(
    (file) =>
      `- ${file.path} (${file.role}, ${file.language}, ~${file.estimatedTokens.toLocaleString()} tokens)`,
  );

  return [
    title,
    indexedAt,
    "",
    pack.purpose,
    `Estimated full scan tokens: ${summary.estimatedFullScanTokens.toLocaleString()}`,
    `Estimated pack tokens: ${pack.estimatedTokens.toLocaleString()}`,
    `Estimated tokens avoided: ${Math.max(0, summary.estimatedFullScanTokens - pack.estimatedTokens).toLocaleString()}`,
    `Estimated savings vs full scan: ${pack.savingsVsFullScanPct.toFixed(1)}%`,
    "",
    formatRepoGraphMarkdown(summary.graph),
    "",
    "## Files",
    ...files,
  ]
    .filter((line) => line !== null)
    .join("\n")
.trim();
}

export function buildRepoAgentManifest(
  summary: RepoIntelligenceSummary,
  generatedAt = new Date().toISOString(),
): RepoAgentManifest {
  const repoRoot = summary.repoRoot ?? "";
  const fullScanTokens = summary.estimatedFullScanTokens;
  return {
    schemaVersion: 1,
    kind: "mac_ai_switchboard.repo_intelligence_manifest",
    repoRoot,
    generatedAt,
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
    },
    packs: summary.packs.map((pack) => ({
      id: pack.id,
      title: pack.title,
      purpose: pack.purpose,
      fileCount: pack.files.length,
      estimatedTokens: pack.estimatedTokens,
      estimatedTokensAvoided: Math.max(0, fullScanTokens - pack.estimatedTokens),
      savingsVsFullScanPct: pack.savingsVsFullScanPct,
      command: `npm run repo:intelligence -- ${JSON.stringify(repoRoot || ".")} --pack ${pack.id} --format markdown`,
    })),
    safety: {
      readOnly: true,
      excludesSecretLikePaths: true,
      modifiesRepository: false,
    },
  };
}

export function formatRepoAgentManifestJson(
  summary: RepoIntelligenceSummary,
  generatedAt?: string,
): string {
  return `${JSON.stringify(buildRepoAgentManifest(summary, generatedAt), null, 2)}\n`;
}

export function estimateRepoIntelligenceSavings(
  summary: RepoIntelligenceSummary,
): RepoSavingsEstimate {
  const fullScanTokens = Math.max(0, summary.estimatedFullScanTokens);
  const sortedPacks = [...summary.packs].sort(
    (a, b) =>
      b.savingsVsFullScanPct - a.savingsVsFullScanPct ||
      a.estimatedTokens - b.estimatedTokens ||
      a.title.localeCompare(b.title),
  );
  const bestPack = sortedPacks[0];
  const bestPackTokens = bestPack?.estimatedTokens ?? 0;
  const allPacksTokens = summary.packs.reduce(
    (sum, pack) => sum + pack.estimatedTokens,
    0,
  );
  const bestPackTokensAvoided = Math.max(0, fullScanTokens - bestPackTokens);
  const allPacksTokensAvoided = Math.max(0, fullScanTokens - allPacksTokens);

  return {
    fullScanTokens,
    bestPackTokens,
    bestPackTokensAvoided,
    bestPackSavingsPct:
      fullScanTokens > 0
        ? Math.round((bestPackTokensAvoided / fullScanTokens) * 1000) / 10
        : 0,
    allPacksTokens,
    allPacksTokensAvoided,
    allPacksSavingsPct:
      fullScanTokens > 0
        ? Math.round((allPacksTokensAvoided / fullScanTokens) * 1000) / 10
        : 0,
    bestPack,
  };
}

function formatRepoGraphMarkdown(graph: RepoGraphSummary | undefined): string {
  if (!graph) {
    return "";
  }

  const lines = ["## Repo Graph Summary"];
  const directories = graph.topDirectories
    .map((node) => `- ${node.label}: ${node.count} files, ~${node.estimatedTokens.toLocaleString()} tokens`)
    .slice(0, 6);
  const languages = graph.topLanguages
    .map((node) => `- ${node.label}: ${node.count} files`)
    .slice(0, 6);
  const entrypoints = graph.entrypoints
    .map((file) => `- ${file.path} (${file.language})`)
    .slice(0, 8);
  const tests = graph.likelyTests
    .map((file) => `- ${file.path}`)
    .slice(0, 8);
  const config = graph.configHubs
    .map((file) => `- ${file.path}`)
    .slice(0, 8);
  const dependencies = (graph.dependencyHubs ?? [])
    .map((file) => `- ${file.path}`)
    .slice(0, 8);

  if (directories.length) {
    lines.push("", "Top directories", ...directories);
  }
  if (languages.length) {
    lines.push("", "Top languages", ...languages);
  }
  if (entrypoints.length) {
    lines.push("", "Likely entrypoints", ...entrypoints);
  }
  if (tests.length) {
    lines.push("", "Likely tests", ...tests);
  }
  if (config.length) {
    lines.push("", "Config hubs", ...config);
  }
  if (dependencies.length) {
    lines.push("", "Dependency hubs", ...dependencies);
  }

  return lines.join("\n");
}

function buildContextPack(
  id: string,
  title: string,
  purpose: string,
  files: RepoFileSignal[],
  estimatedFullScanTokens: number,
): RepoContextPack {
  const sorted = [...files]
    .sort((a, b) => a.estimatedTokens - b.estimatedTokens || a.path.localeCompare(b.path))
    .slice(0, 40);
  const estimatedTokens = sorted.reduce((sum, signal) => sum + signal.estimatedTokens, 0);
  const savingsVsFullScanPct =
    estimatedFullScanTokens > 0
      ? Math.max(0, Math.round((1 - estimatedTokens / estimatedFullScanTokens) * 1000) / 10)
      : 0;

  return {
    id,
    title,
    purpose,
    files: sorted,
    estimatedTokens,
    savingsVsFullScanPct,
  };
}

function buildRepoGraphSummary(files: RepoFileSignal[]): RepoGraphSummary {
  const included = files.filter((file) => file.includeByDefault);
  const sourceAndConfig = included.filter(
    (file) => file.role === "source" || file.role === "config",
  );

  return {
    topDirectories: summarizeGraphNodes(
      included,
      (file) => topDirectory(file.path),
      6,
    ),
    topLanguages: summarizeGraphNodes(
      included.filter((file) => file.language !== "Unknown"),
      (file) => file.language,
      6,
    ),
    entrypoints: sourceAndConfig.filter(isLikelyEntrypoint).slice(0, 12),
    likelyTests: included.filter((file) => file.role === "test").slice(0, 12),
    configHubs: included.filter((file) => file.role === "config").slice(0, 12),
    dependencyHubs: files.filter(isDependencyHub).slice(0, 12),
  };
}

function summarizeGraphNodes(
  files: RepoFileSignal[],
  labelForFile: (file: RepoFileSignal) => string,
  limit: number,
): RepoGraphNode[] {
  const nodes = new Map<string, RepoGraphNode>();

  for (const file of files) {
    const label = labelForFile(file);
    const node =
      nodes.get(label) ??
      ({
        label,
        count: 0,
        estimatedTokens: 0,
        examples: [],
      } satisfies RepoGraphNode);

    node.count += 1;
    node.estimatedTokens += file.estimatedTokens;
    if (node.examples.length < 4) {
      node.examples.push(file.path);
    }
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

function topDirectory(filePath: string): string {
  const [first, second] = filePath.split("/");
  if (!second) {
    return ".";
  }
  return first;
}

function isDependencyHub(file: RepoFileSignal): boolean {
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

function isLikelyEntrypoint(file: RepoFileSignal): boolean {
  const normalized = file.path.toLowerCase();
  const name = normalized.split("/").pop() ?? normalized;

  return (
    file.role === "source" &&
    (name === "main.ts" ||
      name === "main.tsx" ||
      name === "main.js" ||
      name === "index.ts" ||
      name === "index.tsx" ||
      name === "index.js" ||
      name === "app.tsx" ||
      name === "app.ts" ||
      name === "lib.rs" ||
      name === "main.rs" ||
      normalized.endsWith("/src-tauri/src/lib.rs"))
  );
}
