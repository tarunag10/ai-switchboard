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

export type RepoGraphEdgeKind =
  | "test_to_source"
  | "entrypoint_to_config"
  | "source_to_dependency_hub";

export interface RepoGraphEdge {
  from: string;
  to: string;
  kind: RepoGraphEdgeKind;
  reason: string;
}

export interface RepoGraphSummary {
  topDirectories: RepoGraphNode[];
  topLanguages: RepoGraphNode[];
  entrypoints: RepoFileSignal[];
  likelyTests: RepoFileSignal[];
  configHubs: RepoFileSignal[];
  dependencyHubs?: RepoFileSignal[];
  importEdges?: RepoGraphEdge[];
  reverseDependencyHubs?: RepoGraphNode[];
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
    importEdgeCount: number;
    reverseDependencyHubCount: number;
    importEdges: RepoGraphEdge[];
    reverseDependencyHubs: RepoGraphNode[];
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
  agentRecipes: Array<{
    id: string;
    label: string;
    tools: string[];
    packIds: string[];
    instruction: string;
    command: string;
  }>;
  safety: {
    readOnly: true;
    excludesSecretLikePaths: true;
    modifiesRepository: false;
  };
}

export interface RepoAgentHandoffPayload {
  schemaVersion: 1;
  kind: "mac_ai_switchboard.repo_agent_handoff";
  repoRoot: string;
  agent: {
    id: RepoAgentHandoffTarget;
    label: string;
    toolKind: RepoAgentHandoffProfile["toolKind"];
    guidance: string;
  };
  pack: {
    id: RepoContextPack["id"];
    title: string;
    purpose: string;
    estimatedTokens: number;
    estimatedTokensAvoided: number;
    savingsVsFullScanPct: number;
    files: Array<{
      path: string;
      role: RepoFileRole;
      language: string;
      estimatedTokens: number;
      reasons: string[];
    }>;
  };
  graph: {
    available: boolean;
    dependencyHubs: RepoFileSignal[];
    importEdges: RepoGraphEdge[];
    reverseDependencyHubs: RepoGraphNode[];
  };
  safety: {
    readOnly: true;
    excludesSecretLikePaths: true;
    modifiesRepository: false;
    manualProviderRouting: true;
  };
}

export type RepoAgentHandoffTarget =
  | "claude"
  | "codex"
  | "gemini"
  | "opencode"
  | "aider"
  | "goose"
  | "cursor"
  | "continue"
  | "grok"
  | "qwen"
  | "amazonq"
  | "windsurf"
  | "zed";

export interface RepoAgentHandoffProfile {
  id: RepoAgentHandoffTarget;
  label: string;
  toolKind: "cli" | "editor" | "chat";
  defaultPackId: "implementation" | "verification" | "handoff";
  guidance: string;
}

export const repoAgentHandoffProfiles: RepoAgentHandoffProfile[] = [
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
] as const;

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
      importEdgeCount: summary.graph?.importEdges?.length ?? 0,
      reverseDependencyHubCount: summary.graph?.reverseDependencyHubs?.length ?? 0,
      importEdges: summary.graph?.importEdges ?? [],
      reverseDependencyHubs: summary.graph?.reverseDependencyHubs ?? [],
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
    agentRecipes: repoAgentRecipeTemplates.map((recipe) => ({
      ...recipe,
      tools: [...recipe.tools],
      packIds: [...recipe.packIds],
      command: `npm run repo:intelligence -- ${JSON.stringify(repoRoot || ".")} --pack ${recipe.packIds[0]} --format markdown`,
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

export function buildRepoAgentHandoffPayload(
  summary: RepoIntelligenceSummary,
  target: RepoAgentHandoffTarget,
  packId?: string,
): RepoAgentHandoffPayload {
  const profile = repoAgentHandoffProfiles.find((candidate) => candidate.id === target);
  if (!profile) {
    throw new Error(`Unknown agent handoff target: ${target}`);
  }

  const selectedPack =
    summary.packs.find((pack) => pack.id === (packId ?? profile.defaultPackId)) ??
    summary.packs.find((pack) => pack.id === profile.defaultPackId) ??
    summary.packs[0];

  if (!selectedPack) {
    throw new Error("No repo intelligence packs available.");
  }

  return {
    schemaVersion: 1,
    kind: "mac_ai_switchboard.repo_agent_handoff",
    repoRoot: summary.repoRoot ?? "",
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
        reasons: [...file.reasons],
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

export function formatRepoAgentHandoffMarkdown(
  summary: RepoIntelligenceSummary,
  target: RepoAgentHandoffTarget,
  packId?: string,
): string {
  const profile = repoAgentHandoffProfiles.find((candidate) => candidate.id === target);
  if (!profile) {
    throw new Error(`Unknown agent handoff target: ${target}`);
  }

  const selectedPack =
    summary.packs.find((pack) => pack.id === (packId ?? profile.defaultPackId)) ??
    summary.packs.find((pack) => pack.id === profile.defaultPackId) ??
    summary.packs[0];

  if (!selectedPack) {
    throw new Error("No repo intelligence packs are available.");
  }

  const repoLabel = summary.repoRoot ?? "current repository";
  const packMarkdown = formatSingleRepoContextPackMarkdown(summary, selectedPack);

  return [
    `# ${profile.label} Handoff`,
    "",
    `Repository: ${repoLabel}`,
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
    packMarkdown,
  ].join("\n");
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
  const importEdges = (graph.importEdges ?? [])
    .map((edge) => `- ${edge.from} -> ${edge.to} (${edge.kind}: ${edge.reason})`)
    .slice(0, 8);
  const reverseDependencyHubs = (graph.reverseDependencyHubs ?? [])
    .map((node) => `- ${node.label}: ${node.count} inbound links`)
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
  if (importEdges.length) {
    lines.push("", "Import and dependency edges", ...importEdges);
  }
  if (reverseDependencyHubs.length) {
    lines.push("", "Reverse dependency hubs", ...reverseDependencyHubs);
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
  const importEdges = buildRepoGraphEdges(included);

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
    importEdges,
    reverseDependencyHubs: buildReverseDependencyHubs(included, importEdges),
  };
}

function buildRepoGraphEdges(files: RepoFileSignal[]): RepoGraphEdge[] {
  const byPath = new Map(files.map((file) => [file.path, file]));
  const dependencyHubs = files.filter(isDependencyHub);
  const configHubs = files.filter((file) => file.role === "config");
  const edges: RepoGraphEdge[] = [];
  const pushEdge = (edge: RepoGraphEdge) => {
    if (edge.from === edge.to || edges.length >= 24) {
      return;
    }
    if (
      edges.some(
        (existing) =>
          existing.from === edge.from &&
          existing.to === edge.to &&
          existing.kind === edge.kind,
      )
    ) {
      return;
    }
    edges.push(edge);
  };

  for (const file of files) {
    if (file.role === "test") {
      const target = findTestTarget(file, byPath);
      if (target) {
        pushEdge({
          from: file.path,
          to: target.path,
          kind: "test_to_source",
          reason: "test filename matches source module",
        });
      }
    }

    if (isLikelyEntrypoint(file)) {
      const config = findNearestConfigHub(file, configHubs);
      if (config) {
        pushEdge({
          from: file.path,
          to: config.path,
          kind: "entrypoint_to_config",
          reason: "entrypoint shares closest config surface",
        });
      }
    }

    if (file.role === "source") {
      const dependencyHub = findNearestDependencyHub(file, dependencyHubs);
      if (dependencyHub) {
        pushEdge({
          from: file.path,
          to: dependencyHub.path,
          kind: "source_to_dependency_hub",
          reason: "source file belongs to dependency hub scope",
        });
      }
    }
  }

  return edges;
}

function buildReverseDependencyHubs(
  files: RepoFileSignal[],
  edges: RepoGraphEdge[],
): RepoGraphNode[] {
  const byPath = new Map(files.map((file) => [file.path, file]));
  const inbound = new Map<string, RepoGraphNode>();
  for (const edge of edges) {
    const target = byPath.get(edge.to);
    const node = inbound.get(edge.to) ?? {
      label: edge.to,
      count: 0,
      estimatedTokens: target?.estimatedTokens ?? 0,
      examples: [],
    };
    node.count += 1;
    if (node.examples.length < 4) {
      node.examples.push(edge.from);
    }
    inbound.set(edge.to, node);
  }

  return [...inbound.values()]
    .sort(
      (a, b) =>
        b.count - a.count ||
        b.estimatedTokens - a.estimatedTokens ||
        a.label.localeCompare(b.label),
    )
    .slice(0, 12);
}


function findTestTarget(
  testFile: RepoFileSignal,
  byPath: Map<string, RepoFileSignal>,
): RepoFileSignal | undefined {
  const candidates = testTargetCandidates(testFile.path);
  return candidates.map((candidate) => byPath.get(candidate)).find(Boolean);
}

function testTargetCandidates(path: string): string[] {
  const extension = extensionForPath(path);
  const withoutExtension = extension ? path.slice(0, -extension.length) : path;
  const base = withoutExtension.replace(/\.(test|spec)$/i, "");
  if (base === withoutExtension) {
    return [];
  }
  const extensions = [extension, ".tsx", ".ts", ".jsx", ".js", ".rs"].filter(Boolean);
  return [...new Set(extensions.map((candidateExtension) => `${base}${candidateExtension}`))];
}

function findNearestConfigHub(
  file: RepoFileSignal,
  configHubs: RepoFileSignal[],
): RepoFileSignal | undefined {
  return nearestScopedFile(file, configHubs) ?? configHubs.find((candidate) => !candidate.path.includes("/"));
}

function findNearestDependencyHub(
  file: RepoFileSignal,
  dependencyHubs: RepoFileSignal[],
): RepoFileSignal | undefined {
  return nearestScopedFile(file, dependencyHubs) ?? dependencyHubs.find((candidate) => !candidate.path.includes("/"));
}

function nearestScopedFile(
  file: RepoFileSignal,
  candidates: RepoFileSignal[],
): RepoFileSignal | undefined {
  const scoped = candidates
    .filter((candidate) => candidate.path !== file.path)
    .map((candidate) => ({
      candidate,
      score: sharedPathPrefixScore(file.path, candidate.path),
    }))
    .filter((item) => item.score > 0)
    .sort(
      (a, b) =>
        b.score - a.score ||
        a.candidate.path.split("/").length - b.candidate.path.split("/").length ||
        a.candidate.path.localeCompare(b.candidate.path),
    );
  return scoped[0]?.candidate;
}

function sharedPathPrefixScore(left: string, right: string): number {
  const leftParts = left.split("/");
  const rightParts = right.split("/");
  let score = 0;
  while (leftParts[score] && leftParts[score] === rightParts[score]) {
    score += 1;
  }
  if (!right.includes("/") && leftParts.length > 1) {
    return 1;
  }
  return score;
}

function extensionForPath(filePath: string): string {
  const name = filePath.split("/").pop() ?? filePath;
  const dot = name.lastIndexOf(".");
  return dot >= 0 ? name.slice(dot) : "";
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
