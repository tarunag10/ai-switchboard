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

export interface RepoIntelligenceSummary {
  indexedAt?: string;
  repoRoot?: string;
  totalFiles: number;
  indexedFiles: number;
  skippedFiles?: number;
  estimatedFullScanTokens: number;
  roleCounts: Record<RepoFileRole, number>;
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

  return [...overview, ...packSections].join("\n\n").trim();
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
    "## Files",
    ...files,
  ]
    .filter((line) => line !== null)
    .join("\n")
    .trim();
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
