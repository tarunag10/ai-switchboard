import type { SwitchboardMode } from "./types";

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
  | "source_to_dependency_hub"
  | "symbol_reference"
  | "import_reference"
  | "call_reference";

export interface RepoGraphEdge {
  from: string;
  to: string;
  kind: RepoGraphEdgeKind;
  reason: string;
}

export type RepoSymbolKind =
  "function" | "class" | "struct" | "enum" | "trait" | "const";

export interface RepoSymbol {
  name: string;
  kind: RepoSymbolKind;
  file: string;
  line: number;
  parent?: string | null;
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
  symbols?: RepoSymbol[];
  symbolEdges?: RepoGraphEdge[];
}

export interface RepoFileIndexEntry {
  path: string;
  bytes: number;
  modifiedUnixMs: number;
  fingerprint: string;
}

export interface RepoSkippedIndexEntry {
  path: string;
  role: RepoFileRole;
  reasons: string[];
}

export interface RepoGraphInputEntry {
  path: string;
  role: RepoFileRole;
  language: string;
  bytes: number;
  fingerprint: string;
}

export interface RepoIndexMetadata {
  schemaVersion: number;
  indexerVersion: string;
  parserVersion: string;
  cacheKey: string;
  cacheState: "new" | "unchanged" | "changed";
  generatedAt: string;
  previousIndexedAt?: string | null;
  fileCount: number;
  indexedFileCount: number;
  skippedFileCount: number;
  fileFingerprints: RepoFileIndexEntry[];
  skippedFiles: RepoSkippedIndexEntry[];
  graphInputs: RepoGraphInputEntry[];
}

export interface RepoIntelligenceSummary {
  indexedAt?: string;
  repoRoot?: string;
  indexerVersion?: string;
  totalFiles: number;
  indexedFiles: number;
  skippedFiles?: number;
  estimatedFullScanTokens: number;
  roleCounts: Record<RepoFileRole, number>;
  indexMetadata?: RepoIndexMetadata | null;
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

export interface RepoIndexFreshness {
  status: "none" | "fresh" | "unchanged_cache" | "changed_cache" | "unknown";
  label: string;
  detail: string;
}

export type AgentSessionTaskType =
  | "implementation"
  | "verification"
  | "handoff"
  | "risk_review"
  | "release_handoff";

export interface AgentSessionModeInputs {
  headroomHealthy: boolean;
  rtkHealthy: boolean;
  providerRoutingSafe: boolean;
  headroomCompressionRisk?: boolean;
  cleanPassThrough?: boolean;
}

export interface AgentSessionPreparationOptions {
  target: RepoAgentHandoffTarget;
  taskType?: AgentSessionTaskType;
  modeInputs: AgentSessionModeInputs;
  generatedAt?: string;
}

export type AgentSessionCopyStatus = "ready" | "warn" | "blocked";

export interface AgentSessionPreparation {
  target: RepoAgentHandoffProfile;
  taskType: AgentSessionTaskType;
  packId: string;
  freshness: RepoIndexFreshness;
  copyStatus: AgentSessionCopyStatus;
  copyDetail: string;
  recommendedMode: SwitchboardMode;
  recommendedModeReason: string;
  handoffMarkdown: string | null;
  handoffPayload: RepoAgentHandoffPayload | null;
  manifest: RepoAgentManifest;
}

export interface RepoAgentManifest {
  schemaVersion: 1;
  kind: "mac_ai_switchboard.repo_intelligence_manifest";
  repoRoot: string;
  generatedAt: string;
  totals: {
    totalFiles: number;
    indexedFiles: number;
    indexerVersion: string;
    estimatedFullScanTokens: number;
    roleCounts: Record<RepoFileRole, number>;
    indexMetadata?: RepoIndexMetadata | null;
  };
  graph: {
    available: boolean;
    topDirectories: RepoGraphNode[];
    topLanguages: RepoGraphNode[];
    entrypointCount: number;
    likelyTestCount: number;
    configHubCount: number;
    dependencyHubCount: number;
    symbolCount: number;
    symbolEdgeCount: number;
    importEdgeCount: number;
    reverseDependencyHubCount: number;
    symbols: RepoSymbol[];
    symbolEdges: RepoGraphEdge[];
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

export function getRepoIndexFreshness(
  summary: Pick<
    RepoIntelligenceSummary,
    "indexedAt" | "indexMetadata" | "indexerVersion"
  >,
): RepoIndexFreshness {
  if (!summary.indexedAt) {
    return {
      status: "none",
      label: "No repo indexed",
      detail: "Index a local repository to create a persistent metadata cache.",
    };
  }
  const metadata = summary.indexMetadata;
  if (!metadata) {
    return {
      status: "unknown",
      label: "Indexed without cache metadata",
      detail: "Re-index this repo to add persistent freshness metadata.",
    };
  }
  if (metadata.cacheState === "unchanged") {
    return {
      status: "unchanged_cache",
      label: "Unchanged local index",
      detail: metadata.previousIndexedAt
        ? `Same cache key as ${new Date(metadata.previousIndexedAt).toLocaleString()}.`
        : "Same cache key as the previous saved index.",
    };
  }
  if (metadata.cacheState === "changed") {
    return {
      status: "changed_cache",
      label: "Changed local index",
      detail: "Repo metadata changed since the previous saved index.",
    };
  }
  return {
    status: "fresh",
    label: "Fresh local index",
    detail: `Indexed with ${metadata.parserVersion}.`,
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
    symbols: RepoSymbol[];
    symbolEdges: RepoGraphEdge[];
    importEdges: RepoGraphEdge[];
    reverseDependencyHubs: RepoGraphNode[];
  };
  safety: {
    readOnly: true;
    excludesSecretLikePaths: true;
    modifiesRepository: false;
    manualProviderRouting: true;
  };
  configReadiness?: RepoAgentConfigReadiness;
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

export interface RepoAgentConfigReadiness {
  plannedConnectorId: string;
  automationEnabled: false;
  safetyNote: string;
  gatedSteps: Array<{
    id: string;
    label: string;
  }>;
}

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
    guidance:
      "Paste this into the session as bounded repo context before editing.",
  },
  {
    id: "aider",
    label: "Aider",
    toolKind: "cli",
    defaultPackId: "implementation",
    guidance:
      "Use this to choose files intentionally before adding them to an Aider chat.",
  },
  {
    id: "goose",
    label: "Goose",
    toolKind: "cli",
    defaultPackId: "verification",
    guidance:
      "Use this for test, build, and release-check tasks with minimal context.",
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
    guidance:
      "Paste into Continue chat as read-only context; do not auto-write config.",
  },
  {
    id: "grok",
    label: "Grok / xAI CLI",
    toolKind: "chat",
    defaultPackId: "implementation",
    guidance:
      "Use this as compact task context where local CLI integration remains manual.",
  },
  {
    id: "qwen",
    label: "Qwen Code",
    toolKind: "cli",
    defaultPackId: "implementation",
    guidance:
      "Paste into Qwen Code as bounded repo context; keep provider and account routing manual.",
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
    guidance:
      "Paste into Zed assistant as read-only context while model/provider selection stays manual.",
  },
];

const plannedConnectorIdByAgentTarget: Partial<
  Record<RepoAgentHandoffTarget, string>
> = {
  gemini: "gemini_cli",
  opencode: "opencode",
  aider: "aider",
  goose: "goose",
  cursor: "cursor",
  continue: "continue",
  grok: "grok_cli",
  qwen: "qwen_code",
  amazonq: "amazon_q",
  windsurf: "windsurf",
  zed: "zed_ai",
};

const plannedConnectorConfigGateSteps = [
  { id: "detect", label: "Detect config surface" },
  { id: "dryRunDiff", label: "Show dry-run diff" },
  { id: "backup", label: "Create backup" },
  { id: "apply", label: "Apply with consent" },
  { id: "verify", label: "Verify in Doctor" },
  { id: "rollback", label: "Rollback safely" },
  { id: "offCleanup", label: "Clean up in Off mode" },
];

function buildRepoAgentConfigReadiness(
  target: RepoAgentHandoffTarget,
): RepoAgentConfigReadiness | undefined {
  const plannedConnectorId = plannedConnectorIdByAgentTarget[target];
  if (!plannedConnectorId) {
    return undefined;
  }

  return {
    plannedConnectorId,
    automationEnabled: false,
    safetyNote:
      "Planned connector config creation stays disabled until detection, dry-run diff, backup, apply, verify, rollback, and Off cleanup are implemented and tested.",
    gatedSteps: plannedConnectorConfigGateSteps.map((step) => ({ ...step })),
  };
}

const repoAgentRecipeTemplates = [
  {
    id: "cli_implementation",
    label: "CLI implementation handoff",
    tools: [
      "Claude Code",
      "Gemini CLI",
      "OpenCode",
      "Aider",
      "Goose",
      "Qwen Code",
    ],
    packIds: ["implementation"],
    instruction:
      "Copy the implementation pack into the CLI agent before asking for feature or bug-fix work.",
  },
  {
    id: "cli_verification",
    label: "CLI verification handoff",
    tools: [
      "Codex",
      "Gemini CLI",
      "OpenCode",
      "Aider",
      "Goose",
      "Amazon Q Developer CLI",
    ],
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
export const repoIntelligenceIndexerVersion = "path-graph-v2";

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
  ".envrc",
  ".git-credentials",
  ".netrc",
  "settings.local.json",
  "credentials.toml",
  ".npmrc",
  ".pypirc",
  "headroom_memory.db",
  "id_rsa",
  "id_ed25519",
]);
const secretPathPatterns = [
  /(^|\/)\.secrets?\//,
  /(^|\/)secrets?\//,
  /(^|\/)private_keys?\//,
  /(^|\/)\.private_keys?\//,
  /(^|\/)\.aws\//,
  /(^|\/)\.azure\//,
  /(^|\/)\.cargo\/credentials(?:\.toml)?$/i,
  /(^|\/)\.config\/gh\//,
  /(^|\/)\.gnupg\//,
  /(^|\/)\.ssh\//,
  /(^|\/)\.playwright-mcp\//,
  /(^|\/)authkey_[^/]+\.p8$/i,
  /\.(db|sqlite|sqlite3|log)$/i,
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
  const name =
    normalized.split("/").pop()?.toLowerCase() ?? normalized.toLowerCase();
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
  } else if (
    lower.includes(".test.") ||
    lower.includes(".spec.") ||
    lower.includes("/tests/")
  ) {
    role = "test";
    reasons.push("test path");
  } else if (
    lower.endsWith(".md") ||
    lower.startsWith("docs/") ||
    lower.includes("/docs/")
  ) {
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
  } else if (
    [".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".webp"].includes(
      extension,
    )
  ) {
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
    (role === "source" ||
      role === "test" ||
      role === "config" ||
      role === "docs");

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
  files: Array<{ path: string; bytes?: number; content?: string }>,
): RepoIntelligenceSummary {
  const signals = files.map((file) =>
    classifyRepoFile(file.path, file.bytes ?? 0),
  );
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

  const contentByPath = new Map(
    files
      .filter((file) => typeof file.content === "string")
      .map((file) => [file.path.replace(/\\/g, "/"), file.content ?? ""]),
  );
  const graph = buildRepoGraphSummary(indexed, contentByPath);
  const packs = [
    buildContextPack(
      "implementation",
      "Implementation Pack",
      "Source files likely needed for feature work.",
      indexed.filter(
        (signal) => signal.role === "source" || signal.role === "config",
      ),
      estimatedFullScanTokens,
    ),
    buildContextPack(
      "verification",
      "Verification Pack",
      "Tests, scripts, and config likely needed before committing.",
      indexed.filter(
        (signal) => signal.role === "test" || signal.role === "config",
      ),
      estimatedFullScanTokens,
    ),
    buildContextPack(
      "handoff",
      "Handoff Pack",
      "Docs and project metadata useful for another agent or maintainer.",
      indexed.filter(
        (signal) => signal.role === "docs" || signal.role === "config",
      ),
      estimatedFullScanTokens,
    ),
    buildContextPack(
      "risk_review",
      "Risk Review Pack",
      "Source, tests, and config likely needed for regression or security review.",
      indexed.filter(
        (signal) =>
          signal.role === "source" ||
          signal.role === "test" ||
          signal.role === "config",
      ),
      estimatedFullScanTokens,
    ),
    buildContextPack(
      "release_handoff",
      "Release Handoff Pack",
      "Verification, docs, and config useful for release readiness handoff.",
      indexed.filter(
        (signal) =>
          signal.role === "test" ||
          signal.role === "docs" ||
          signal.role === "config",
      ),
      estimatedFullScanTokens,
    ),
  ];

  const indexMetadata = buildRepoIndexMetadata(files, signals);
  return {
    totalFiles: signals.length,
    indexedFiles: indexed.length,
    indexerVersion: repoIntelligenceIndexerVersion,
    estimatedFullScanTokens,
    roleCounts,
    indexMetadata,
    graph,
    packs,
  };
}

function buildRepoIndexMetadata(
  files: Array<{ path: string; bytes?: number; content?: string }>,
  signals: RepoFileSignal[],
): RepoIndexMetadata {
  const includeByPath = new Map(
    signals.map((signal) => [signal.path, signal.includeByDefault]),
  );
  const fileFingerprints = files
    .map((file) => {
      const normalizedPath = file.path.replace(/\\/g, "/");
      const bytes = file.bytes ?? 0;
      const contentHash =
        typeof file.content === "string" ? hashString(file.content) : "";
      return {
        path: normalizedPath,
        bytes,
        modifiedUnixMs: 0,
        fingerprint: hashString(`${normalizedPath}:${bytes}:${contentHash}`),
      };
    })
    .filter((entry) => includeByPath.get(entry.path) === true)
    .sort((a, b) => a.path.localeCompare(b.path));
  const fingerprintByPath = new Map(
    fileFingerprints.map((entry) => [entry.path, entry]),
  );
  const skippedFiles = signals
    .filter((signal) => !signal.includeByDefault)
    .map((signal) => ({
      path: signal.reasons.includes("secret-like path excluded")
        ? "<secret-like path>"
        : signal.path,
      role: signal.role,
      reasons: signal.reasons.length
        ? signal.reasons
        : ["not included in default repo index"],
    }))
    .sort((a, b) => a.path.localeCompare(b.path));
  const graphInputs = signals
    .filter(
      (signal) =>
        signal.includeByDefault &&
        (signal.role === "source" ||
          signal.role === "test" ||
          signal.role === "config"),
    )
    .map((signal) => {
      const fingerprint = fingerprintByPath.get(signal.path);
      return {
        path: signal.path,
        role: signal.role,
        language: signal.language,
        bytes: fingerprint?.bytes ?? 0,
        fingerprint: fingerprint?.fingerprint ?? "",
      };
    })
    .sort((a, b) => a.path.localeCompare(b.path));
  const cacheKey = hashString(
    [
      "1",
      repoIntelligenceIndexerVersion,
      "metadata-fingerprint-v1",
      ...fileFingerprints.map(
        (entry) => `${entry.path}:${entry.bytes}:${entry.fingerprint}`,
      ),
      ...graphInputs.map(
        (entry) => `graph:${entry.path}:${entry.role}:${entry.fingerprint}`,
      ),
    ].join("|"),
  );

  return {
    schemaVersion: 1,
    indexerVersion: repoIntelligenceIndexerVersion,
    parserVersion: "metadata-fingerprint-v1",
    cacheKey,
    cacheState: "new",
    generatedAt: new Date().toISOString(),
    previousIndexedAt: null,
    fileCount: files.length,
    indexedFileCount: signals.filter((signal) => signal.includeByDefault)
      .length,
    skippedFileCount: signals.filter((signal) => !signal.includeByDefault)
      .length,
    fileFingerprints,
    skippedFiles,
    graphInputs,
  };
}

function hashString(value: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

export function formatRepoContextPackMarkdown(
  summary: RepoIntelligenceSummary,
): string {
  const title = summary.repoRoot
    ? `# Repo Intelligence Context Pack: ${summary.repoRoot}`
    : "# Repo Intelligence Context Pack";
  const indexedAt = summary.indexedAt
    ? `\nIndexed at: ${summary.indexedAt}`
    : "";
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

  return [...overview, graphSection, ...packSections]
    .filter(Boolean)
    .join("\n\n")
    .trim();
}

export function formatSingleRepoContextPackMarkdown(
  summary: RepoIntelligenceSummary,
  pack: RepoContextPack,
): string {
  const title = summary.repoRoot
    ? `# ${pack.title}: ${summary.repoRoot}`
    : `# ${pack.title}`;
  const indexedAt = summary.indexedAt
    ? `Indexed at: ${summary.indexedAt}`
    : null;
  const files = pack.files
    .slice(0, 40)
    .map(
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
      indexerVersion: summary.indexerVersion ?? "unknown",
      estimatedFullScanTokens: fullScanTokens,
      roleCounts: summary.roleCounts,
      indexMetadata: summary.indexMetadata ?? null,
    },
    graph: {
      available: Boolean(summary.graph),
      topDirectories: summary.graph?.topDirectories ?? [],
      topLanguages: summary.graph?.topLanguages ?? [],
      entrypointCount: summary.graph?.entrypoints.length ?? 0,
      likelyTestCount: summary.graph?.likelyTests.length ?? 0,
      configHubCount: summary.graph?.configHubs.length ?? 0,
      dependencyHubCount: summary.graph?.dependencyHubs?.length ?? 0,
      symbolCount: summary.graph?.symbols?.length ?? 0,
      symbolEdgeCount: summary.graph?.symbolEdges?.length ?? 0,
      importEdgeCount: summary.graph?.importEdges?.length ?? 0,
      reverseDependencyHubCount:
        summary.graph?.reverseDependencyHubs?.length ?? 0,
      symbols: summary.graph?.symbols ?? [],
      symbolEdges: summary.graph?.symbolEdges ?? [],
      importEdges: summary.graph?.importEdges ?? [],
      reverseDependencyHubs: summary.graph?.reverseDependencyHubs ?? [],
    },
    packs: summary.packs.map((pack) => ({
      id: pack.id,
      title: pack.title,
      purpose: pack.purpose,
      fileCount: pack.files.length,
      estimatedTokens: pack.estimatedTokens,
      estimatedTokensAvoided: Math.max(
        0,
        fullScanTokens - pack.estimatedTokens,
      ),
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
  const profile = repoAgentHandoffProfiles.find(
    (candidate) => candidate.id === target,
  );
  if (!profile) {
    throw new Error(`Unknown agent handoff target: ${target}`);
  }

  const selectedPack =
    summary.packs.find(
      (pack) => pack.id === (packId ?? profile.defaultPackId),
    ) ??
    summary.packs.find((pack) => pack.id === profile.defaultPackId) ??
    summary.packs[0];

  if (!selectedPack) {
    throw new Error("No repo intelligence packs available.");
  }
  const configReadiness = buildRepoAgentConfigReadiness(profile.id);

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
      symbols: summary.graph?.symbols ?? [],
      symbolEdges: summary.graph?.symbolEdges ?? [],
      importEdges: summary.graph?.importEdges ?? [],
      reverseDependencyHubs: summary.graph?.reverseDependencyHubs ?? [],
    },
    safety: {
      readOnly: true,
      excludesSecretLikePaths: true,
      modifiesRepository: false,
      manualProviderRouting: true,
    },
    ...(configReadiness ? { configReadiness } : {}),
  };
}

export function formatRepoAgentHandoffMarkdown(
  summary: RepoIntelligenceSummary,
  target: RepoAgentHandoffTarget,
  packId?: string,
): string {
  const profile = repoAgentHandoffProfiles.find(
    (candidate) => candidate.id === target,
  );
  if (!profile) {
    throw new Error(`Unknown agent handoff target: ${target}`);
  }

  const selectedPack =
    summary.packs.find(
      (pack) => pack.id === (packId ?? profile.defaultPackId),
    ) ??
    summary.packs.find((pack) => pack.id === profile.defaultPackId) ??
    summary.packs[0];

  if (!selectedPack) {
    throw new Error("No repo intelligence packs are available.");
  }

  const repoLabel = summary.repoRoot ?? "current repository";
  const packMarkdown = formatSingleRepoContextPackMarkdown(
    summary,
    selectedPack,
  );
  const configReadiness = buildRepoAgentConfigReadiness(profile.id);
  const configReadinessMarkdown = configReadiness
    ? [
        "## Connector Config Readiness",
        `Planned connector: ${configReadiness.plannedConnectorId}`,
        `Automation enabled: ${configReadiness.automationEnabled ? "yes" : "no"}`,
        configReadiness.safetyNote,
        "Gated steps:",
        ...configReadiness.gatedSteps.map((step) => `- ${step.label}`),
        "",
      ].join("\n")
    : "";

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
    configReadiness
      ? "Do not create or modify this connector's config unless every gated config-creation step is implemented and verified."
      : "",
    "",
    configReadinessMarkdown,
    packMarkdown,
  ]
    .filter((line) => line !== "")
    .join("\n");
}

function packIdForAgentSessionTask(
  profile: RepoAgentHandoffProfile,
  taskType: AgentSessionTaskType,
): RepoContextPack["id"] {
  if (taskType === "verification") {
    return "verification";
  }
  if (taskType === "handoff") {
    return "handoff";
  }
  if (taskType === "risk_review") {
    return "risk_review";
  }
  if (taskType === "release_handoff") {
    return "release_handoff";
  }
  return profile.defaultPackId;
}

export function recommendAgentSessionMode({
  headroomHealthy,
  rtkHealthy,
  providerRoutingSafe,
  headroomCompressionRisk = false,
  cleanPassThrough = false,
}: AgentSessionModeInputs): {
  mode: SwitchboardMode;
  reason: string;
} {
  if (cleanPassThrough) {
    return {
      mode: "off",
      reason: "Clean pass-through requested for debugging.",
    };
  }

  if (!providerRoutingSafe || headroomCompressionRisk) {
    if (rtkHealthy) {
      return {
        mode: "rtk",
        reason:
          "Provider routing is unsafe or Headroom compression risk is high; keep shell-output compression only.",
      };
    }

    return {
      mode: "off",
      reason:
        "Provider routing is unsafe and RTK is unavailable, so use clean pass-through.",
    };
  }

  if (headroomHealthy && rtkHealthy) {
    return {
      mode: "full",
      reason: "Headroom engine and RTK are healthy.",
    };
  }

  if (headroomHealthy) {
    return {
      mode: "headroom",
      reason: "Headroom engine is healthy; RTK is unavailable.",
    };
  }

  if (rtkHealthy) {
    return {
      mode: "rtk",
      reason: "RTK is healthy; Headroom engine is unavailable.",
    };
  }

  return {
    mode: "off",
    reason: "No optimization dependency is currently healthy.",
  };
}

function getAgentSessionCopyState(
  summary: RepoIntelligenceSummary,
  freshness: RepoIndexFreshness,
): {
  status: AgentSessionCopyStatus;
  detail: string;
} {
  if (summary.packs.length === 0 || summary.indexedFiles === 0) {
    return {
      status: "blocked",
      detail: "Index a real local repo before copying agent context.",
    };
  }

  if (freshness.status === "changed_cache" || freshness.status === "unknown") {
    return {
      status: "warn",
      detail: `${freshness.label}: refresh before relying on this handoff for current code.`,
    };
  }

  return {
    status: "ready",
    detail: freshness.label,
  };
}

export function buildAgentSessionPreparation(
  summary: RepoIntelligenceSummary,
  options: AgentSessionPreparationOptions,
): AgentSessionPreparation {
  const profile = repoAgentHandoffProfiles.find(
    (candidate) => candidate.id === options.target,
  );
  if (!profile) {
    throw new Error(`Unknown agent handoff target: ${options.target}`);
  }

  const taskType = options.taskType ?? profile.defaultPackId;
  const packId = packIdForAgentSessionTask(profile, taskType);
  const freshness = getRepoIndexFreshness(summary);
  const copyState = getAgentSessionCopyState(summary, freshness);
  const modeRecommendation = recommendAgentSessionMode(options.modeInputs);

  return {
    target: profile,
    taskType,
    packId,
    freshness,
    copyStatus: copyState.status,
    copyDetail: copyState.detail,
    recommendedMode: modeRecommendation.mode,
    recommendedModeReason: modeRecommendation.reason,
    handoffMarkdown:
      copyState.status === "blocked"
        ? null
        : formatRepoAgentHandoffMarkdown(summary, profile.id, packId),
    handoffPayload:
      copyState.status === "blocked"
        ? null
        : buildRepoAgentHandoffPayload(summary, profile.id, packId),
    manifest: buildRepoAgentManifest(summary, options.generatedAt),
  };
}

export function formatAgentSessionPreparationJson(
  preparation: Pick<
    AgentSessionPreparation,
    "handoffPayload" | "copyStatus" | "copyDetail"
  >,
): string | null {
  if (!preparation.handoffPayload || preparation.copyStatus === "blocked") {
    return null;
  }

  return JSON.stringify(preparation.handoffPayload, null, 2);
}

export function formatAgentSessionSelectedPackMarkdown(
  summary: RepoIntelligenceSummary,
  preparation: Pick<AgentSessionPreparation, "packId" | "copyStatus">,
): string | null {
  if (preparation.copyStatus === "blocked") {
    return null;
  }
  const pack = summary.packs.find(
    (candidate) => candidate.id === preparation.packId,
  );
  if (!pack) {
    return null;
  }

  return formatSingleRepoContextPackMarkdown(summary, pack);
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
    .map(
      (node) =>
        `- ${node.label}: ${node.count} files, ~${node.estimatedTokens.toLocaleString()} tokens`,
    )
    .slice(0, 6);
  const languages = graph.topLanguages
    .map((node) => `- ${node.label}: ${node.count} files`)
    .slice(0, 6);
  const entrypoints = graph.entrypoints
    .map((file) => `- ${file.path} (${file.language})`)
    .slice(0, 8);
  const tests = graph.likelyTests.map((file) => `- ${file.path}`).slice(0, 8);
  const config = graph.configHubs.map((file) => `- ${file.path}`).slice(0, 8);
  const dependencies = (graph.dependencyHubs ?? [])
    .map((file) => `- ${file.path}`)
    .slice(0, 8);
  const importEdges = (graph.importEdges ?? [])
    .map(
      (edge) => `- ${edge.from} -> ${edge.to} (${edge.kind}: ${edge.reason})`,
    )
    .slice(0, 8);
  const symbols = (graph.symbols ?? [])
    .map(
      (symbol) =>
        `- ${symbol.name} (${symbol.kind}) in ${symbol.file}:${symbol.line}`,
    )
    .slice(0, 12);
  const symbolEdges = (graph.symbolEdges ?? [])
    .map(
      (edge) => `- ${edge.from} -> ${edge.to} (${edge.kind}: ${edge.reason})`,
    )
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
  if (symbols.length) {
    lines.push("", "Symbols", ...symbols);
  }
  if (symbolEdges.length) {
    lines.push("", "Symbol edges", ...symbolEdges);
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
    .sort(
      (a, b) =>
        a.estimatedTokens - b.estimatedTokens || a.path.localeCompare(b.path),
    )
    .slice(0, 40);
  const estimatedTokens = sorted.reduce(
    (sum, signal) => sum + signal.estimatedTokens,
    0,
  );
  const savingsVsFullScanPct =
    estimatedFullScanTokens > 0
      ? Math.max(
          0,
          Math.round((1 - estimatedTokens / estimatedFullScanTokens) * 1000) /
            10,
        )
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

function buildRepoGraphSummary(
  files: RepoFileSignal[],
  contentByPath = new Map<string, string>(),
): RepoGraphSummary {
  const included = files.filter((file) => file.includeByDefault);
  const sourceAndConfig = included.filter(
    (file) => file.role === "source" || file.role === "config",
  );
  const importEdges = [
    ...buildRepoGraphEdges(included),
    ...buildImportReferenceEdges(included, contentByPath),
  ];
  const symbols = buildRepoSymbols(included);
  const symbolEdges = [
    ...buildSymbolEdges(included, symbols),
    ...buildCallReferenceEdges(included, symbols, contentByPath),
  ];
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
    symbols,
    symbolEdges,
  };
}

function buildRepoSymbols(files: RepoFileSignal[]): RepoSymbol[] {
  const symbols: RepoSymbol[] = [];
  for (const file of files) {
    if (symbols.length >= 200) break;
    if (file.role !== "source" && file.role !== "test") continue;
    if (
      !["TypeScript", "JavaScript", "React", "Rust", "Python"].includes(
        file.language,
      )
    )
      continue;
    const name =
      file.path
        .split("/")
        .pop()
        ?.replace(/\.[^.]+$/, "") ?? file.path;
    symbols.push({
      name,
      kind: file.language === "Rust" ? "struct" : "function",
      file: file.path,
      line: 1,
      parent: null,
    });
  }
  return symbols;
}

function buildSymbolEdges(
  files: RepoFileSignal[],
  symbols: RepoSymbol[],
): RepoGraphEdge[] {
  const edges: RepoGraphEdge[] = [];
  for (const symbol of symbols.slice(0, 80)) {
    for (const file of files) {
      if (edges.length >= 80) return edges;
      if (file.path === symbol.file) continue;
      const to = `${symbol.file}#${symbol.name}`;
      if (!file.path.toLowerCase().includes(symbol.name.toLowerCase()))
        continue;
      if (
        edges.some(
          (edge) =>
            edge.from === file.path &&
            edge.to === to &&
            edge.kind === "symbol_reference",
        )
      ) {
        continue;
      }
      edges.push({
        from: file.path,
        to,
        kind: "symbol_reference",
        reason: "file path references indexed symbol name",
      });
    }
  }
  return edges;
}

function buildImportReferenceEdges(
  files: RepoFileSignal[],
  contentByPath: Map<string, string>,
): RepoGraphEdge[] {
  const sourceFiles = files.filter(
    (file) => file.role === "source" || file.role === "test",
  );
  const byPath = new Map(files.map((file) => [file.path, file]));
  const edges: RepoGraphEdge[] = [];

  for (const file of sourceFiles) {
    const content = contentByPath.get(file.path);
    if (!content) continue;

    for (const specifier of extractImportSpecifiers(content)) {
      if (!specifier.startsWith(".")) continue;
      const target = resolveImportSpecifier(file.path, specifier, byPath);
      if (!target) continue;
      pushUniqueGraphEdge(edges, {
        from: file.path,
        to: target.path,
        kind: "import_reference",
        reason: `source imports ${specifier}`,
      });
      if (edges.length >= 80) return edges;
    }
  }

  return edges;
}

function buildCallReferenceEdges(
  files: RepoFileSignal[],
  symbols: RepoSymbol[],
  contentByPath: Map<string, string>,
): RepoGraphEdge[] {
  const sourceFiles = files.filter(
    (file) => file.role === "source" || file.role === "test",
  );
  const callableSymbols = symbols.filter(
    (symbol) => symbol.kind === "function" || symbol.kind === "const",
  );
  const edges: RepoGraphEdge[] = [];

  for (const file of sourceFiles) {
    const content = contentByPath.get(file.path);
    if (!content) continue;
    for (const symbol of callableSymbols.slice(0, 120)) {
      if (file.path === symbol.file) continue;
      if (!new RegExp(`\\b${escapeRegExp(symbol.name)}\\s*\\(`).test(content)) {
        continue;
      }
      pushUniqueGraphEdge(edges, {
        from: file.path,
        to: `${symbol.file}#${symbol.name}`,
        kind: "call_reference",
        reason: "source text references callable symbol",
      });
      if (edges.length >= 80) return edges;
    }
  }

  return edges;
}

function extractImportSpecifiers(content: string): string[] {
  const specifiers: string[] = [];
  const patterns = [
    /\bimport\s+(?:type\s+)?(?:[^"']+\s+from\s+)?["']([^"']+)["']/g,
    /\bexport\s+(?:type\s+)?[^"']+\s+from\s+["']([^"']+)["']/g,
    /\brequire\(\s*["']([^"']+)["']\s*\)/g,
    /\buse\s+crate::([A-Za-z0-9_:]+)/g,
    /\bmod\s+([A-Za-z0-9_]+)\s*;/g,
  ];

  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) {
      if (match[1]) specifiers.push(match[1]);
    }
  }

  return specifiers;
}

function resolveImportSpecifier(
  fromPath: string,
  specifier: string,
  byPath: Map<string, RepoFileSignal>,
): RepoFileSignal | null {
  const fromDir = fromPath.split("/").slice(0, -1).join("/");
  const normalized = normalizeRepoPath(`${fromDir}/${specifier}`);
  const candidates = [
    normalized,
    `${normalized}.ts`,
    `${normalized}.tsx`,
    `${normalized}.js`,
    `${normalized}.jsx`,
    `${normalized}.mjs`,
    `${normalized}.rs`,
    `${normalized}/index.ts`,
    `${normalized}/index.tsx`,
    `${normalized}/index.js`,
  ];
  for (const candidate of candidates) {
    const target = byPath.get(candidate);
    if (target) return target;
  }
  return null;
}

function normalizeRepoPath(path: string): string {
  const parts: string[] = [];
  for (const part of path.split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") {
      parts.pop();
      continue;
    }
    parts.push(part);
  }
  return parts.join("/");
}

function pushUniqueGraphEdge(edges: RepoGraphEdge[], edge: RepoGraphEdge) {
  if (edge.from === edge.to) return;
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
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
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
  const extensions = [extension, ".tsx", ".ts", ".jsx", ".js", ".rs"].filter(
    Boolean,
  );
  return [
    ...new Set(
      extensions.map((candidateExtension) => `${base}${candidateExtension}`),
    ),
  ];
}

function findNearestConfigHub(
  file: RepoFileSignal,
  configHubs: RepoFileSignal[],
): RepoFileSignal | undefined {
  return (
    nearestScopedFile(file, configHubs) ??
    configHubs.find((candidate) => !candidate.path.includes("/"))
  );
}

function findNearestDependencyHub(
  file: RepoFileSignal,
  dependencyHubs: RepoFileSignal[],
): RepoFileSignal | undefined {
  return (
    nearestScopedFile(file, dependencyHubs) ??
    dependencyHubs.find((candidate) => !candidate.path.includes("/"))
  );
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
        a.candidate.path.split("/").length -
          b.candidate.path.split("/").length ||
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
  const name =
    file.path.split("/").pop()?.toLowerCase() ?? file.path.toLowerCase();
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
