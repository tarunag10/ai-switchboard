import type { SwitchboardMode } from "./types";
import {
  getPlannedConnector,
  getPlannedConnectorConfigCreationPlan,
  getPlannedConnectorSafetyDossier,
} from "./plannedConnectors";

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

export interface RepoFileRank {
  path: string;
  score: number;
  estimatedTokens: number;
  reasons: string[];
  risks: string[];
}

export interface RepoTaskContextPack {
  id: string;
  task: string;
  budgetTokens: number;
  files: RepoFileRank[];
  tests: RepoFileRank[];
  commands: string[];
  omitted: RepoFileRank[];
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
  | "package_dependency"
  | "call_reference";

export interface RepoGraphEdge {
  from: string;
  to: string;
  kind: RepoGraphEdgeKind;
  reason: string;
}

export interface RepoTestRelationship {
  testPath: string;
  sourcePath: string;
  reason: string;
}

export type RepoSymbolKind =
  | "function"
  | "class"
  | "struct"
  | "enum"
  | "trait"
  | "const"
  | "heading";

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
  testRelationships?: RepoTestRelationship[];
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
  taskPacks?: RepoTaskContextPack[];
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

export interface RepoIndexRequestValidation {
  repoPath: string;
  error: string | null;
}

export function normalizeRepoIndexRequest(
  repoPath: string,
): RepoIndexRequestValidation {
  const trimmedPath = repoPath.trim();
  return {
    repoPath: trimmedPath,
    error: trimmedPath
      ? null
      : "Enter a local repository folder path first.",
  };
}

export interface RepoIndexFreshness {
  status: "none" | "fresh" | "unchanged_cache" | "changed_cache" | "unknown";
  label: string;
  detail: string;
  apiAvailable: boolean;
  graphAvailable: boolean;
  indexHealth: string;
  parserHealth: string;
  indexerVersion?: string | null;
  parserVersion?: string | null;
  indexedFileCount?: number | null;
  skippedFileCount?: number | null;
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
  taskQuery?: string;
  budgetTokens?: number;
  modeInputs: AgentSessionModeInputs;
  generatedAt?: string;
}

export type AgentSessionCopyStatus = "ready" | "warn" | "blocked";

export interface AgentSessionCopySafety {
  hasRealIndex: boolean;
  allowsCopy: boolean;
  blocksSampleContext: boolean;
  excludesSecretLikePaths: boolean;
  freshnessStatus: RepoIndexFreshness["status"];
  skippedFileCount: number;
  reason: string;
}

export interface AgentSessionCopyArtifact {
  id: "session_summary" | "full_handoff" | "selected_pack" | "json_payload";
  label: string;
  format: "markdown" | "json";
  available: boolean;
  blockedReason: string | null;
}

export interface AgentSessionPreparation {
  target: RepoAgentHandoffProfile;
  taskType: AgentSessionTaskType;
  packId: string;
  freshness: RepoIndexFreshness;
  copySafety: AgentSessionCopySafety;
  copyStatus: AgentSessionCopyStatus;
  copyDetail: string;
  recommendedMode: SwitchboardMode;
  recommendedModeReason: string;
  copyArtifacts: AgentSessionCopyArtifact[];
  handoffMarkdown: string | null;
  handoffPayload: RepoAgentHandoffPayload | null;
  taskContext: RepoTaskContextPack | null;
  configReadiness: RepoAgentConfigReadiness | null;
  manifest: RepoAgentManifest;
}

export interface AgentSessionDisplayState {
  targetLabel: string;
  packLabel: string;
  modeLabel: string;
  freshnessLabel: string;
  freshnessDetailLabel: string;
  contextLabel: string;
  selectedPackTokensLabel: string;
  tokensAvoidedLabel: string;
  skippedFilesLabel: string;
  secretExclusionLabel: string;
  connectorReadinessLabel: string | null;
  connectorReadinessDetailLabel: string | null;
  sampleContextWarning: string | null;
  copyStatus: AgentSessionCopyStatus;
  copyDetail: string;
  canCopySummary: boolean;
  canCopyHandoff: boolean;
  canCopySelectedPack: boolean;
  canCopyJson: boolean;
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
  taskPacks?: Array<{
    id: string;
    task: string;
    budgetTokens: number;
    fileCount: number;
    testCount: number;
    commandCount: number;
    topFiles: RepoFileRank[];
    tests: RepoFileRank[];
    commands: string[];
    omittedCount: number;
  }>;
  agentRecipes: Array<{
    id: string;
    label: string;
    tools: string[];
    packIds: string[];
    instruction: string;
    command: string;
  }>;
  agentSessionRecipes: Array<{
    id: RepoAgentHandoffTarget;
    label: string;
    toolKind: RepoAgentHandoffProfile["toolKind"];
    taskType: AgentSessionTaskType;
    command: string;
    readOnly: true;
    manualProviderRouting: boolean;
    configReadiness: {
      plannedConnectorId: string;
      nextGate: string;
      automationEnabled: boolean;
    } | null;
  }>;
  apiQueries: Array<{
    id: string;
    description: string;
    command: string;
    readOnly: true;
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
    "indexedAt" | "indexMetadata" | "indexerVersion" | "graph"
  >,
): RepoIndexFreshness {
  const metadata = summary.indexMetadata;
  const base = {
    apiAvailable: true,
    graphAvailable: Boolean(summary.graph),
    indexHealth: metadata?.cacheState ?? "metadata_missing",
    parserHealth:
      metadata?.parserVersion === "metadata-fingerprint-v1"
        ? "current"
        : metadata?.parserVersion
          ? "version_mismatch"
          : "unavailable",
    indexerVersion: summary.indexerVersion ?? metadata?.indexerVersion ?? null,
    parserVersion: metadata?.parserVersion ?? null,
    indexedFileCount: metadata?.indexedFileCount ?? null,
    skippedFileCount: metadata?.skippedFileCount ?? null,
  };

  if (!summary.indexedAt) {
    return {
      ...base,
      status: "none",
      label: "No repo indexed",
      detail: "Index a local repository to create a persistent metadata cache.",
    };
  }
  if (!metadata) {
    return {
      ...base,
      status: "unknown",
      label: "Indexed without cache metadata",
      detail: "Re-index this repo to add persistent freshness metadata.",
    };
  }
  if (metadata.cacheState === "unchanged") {
    return {
      ...base,
      status: "unchanged_cache",
      label: "Unchanged local index",
      detail: metadata.previousIndexedAt
        ? `Same cache key as ${new Date(metadata.previousIndexedAt).toLocaleString()}.`
        : "Same cache key as the previous saved index.",
    };
  }
  if (metadata.cacheState === "changed") {
    return {
      ...base,
      status: "changed_cache",
      label: "Changed local index",
      detail: "Repo metadata changed since the previous saved index.",
    };
  }
  return {
    ...base,
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
  indexFreshness: RepoIndexFreshness;
  safety: {
    readOnly: true;
    excludesSecretLikePaths: true;
    modifiesRepository: false;
    manualProviderRouting: boolean;
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
  plannedConnectorName: string;
  automationEnabled: boolean;
  safetyNote: string;
  nextGate: {
    id: string;
    label: string;
  };
  safetyDossier: {
    configPathStrategy: string;
    accountCaveat: string;
    rollbackStrategy: string;
  };
  gatedSteps: Array<{
    id: string;
    label: string;
    requiredEvidence: string[];
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
    guidance:
      "Paste this before the task. Gemini CLI can use managed Switchboard routing when its connector is enabled.",
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

const primaryRepoAgentIds = new Set<RepoAgentHandoffTarget>([
  "claude",
  "codex",
  "gemini",
  "opencode",
  "windsurf",
  "zed",
]);

function buildRepoAgentConfigReadiness(
  target: RepoAgentHandoffTarget,
): RepoAgentConfigReadiness | undefined {
  const plannedConnectorId = plannedConnectorIdByAgentTarget[target];
  if (!plannedConnectorId) {
    return undefined;
  }
  const plannedConnector = getPlannedConnector(plannedConnectorId);
  const dossier = getPlannedConnectorSafetyDossier(plannedConnectorId);
  if (!plannedConnector || !dossier) {
    return undefined;
  }
  const plan = getPlannedConnectorConfigCreationPlan(plannedConnector);
  const nextGate = plan.steps[0];

  return {
    plannedConnectorId,
    plannedConnectorName: plannedConnector.name,
    automationEnabled: plan.automationEnabled,
    safetyNote: plan.safetyNote,
    nextGate: {
      id: nextGate.id,
      label: nextGate.label,
    },
    safetyDossier: {
      configPathStrategy: dossier.configPathStrategy,
      accountCaveat: dossier.accountCaveat,
      rollbackStrategy: dossier.rollbackStrategy,
    },
    gatedSteps: plan.steps.map((step) => ({
      id: step.id,
      label: step.label,
      requiredEvidence: [...step.requiredEvidence],
    })),
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
      "Use these packs as read-only context in editor assistants; follow each connector readiness state before changing provider routing.",
  },
] as const;

function buildRepoAgentSessionRecipes(repoRoot: string) {
  return repoAgentHandoffProfiles.map((profile) => {
    const configReadiness = buildRepoAgentConfigReadiness(profile.id);
    return {
      id: profile.id,
      label: profile.label,
      toolKind: profile.toolKind,
      taskType: profile.defaultPackId,
      command: `npm run repo:intelligence -- ${JSON.stringify(repoRoot || ".")} --session --agent ${profile.id} --task ${profile.defaultPackId} --format markdown`,
      readOnly: true as const,
      manualProviderRouting: !primaryRepoAgentIds.has(profile.id),
      configReadiness: configReadiness
        ? {
            plannedConnectorId: configReadiness.plannedConnectorId,
            nextGate: configReadiness.nextGate.label,
            automationEnabled: configReadiness.automationEnabled,
          }
        : null,
    };
  });
}

const repoAgentApiQueryTemplates = [
  {
    id: "repo_manifest",
    description: "Read the latest saved Repo Intelligence manifest.",
    command: "get_repo_manifest",
  },
  {
    id: "context_pack",
    description: "Read one bounded context pack from the latest saved index.",
    command: "get_repo_pack",
  },
  {
    id: "agent_handoff",
    description:
      "Read a bounded agent-specific handoff from the latest saved index.",
    command: "get_agent_handoff",
  },
  {
    id: "index_freshness",
    description: "Read index freshness and parser metadata without rescanning.",
    command: "get_index_freshness",
  },
  {
    id: "clear_repo_index",
    description: "Clear the saved Repo Intelligence index metadata.",
    command: "clear_repo_index",
  },
  {
    id: "symbol_search",
    description: "Search symbols in the latest saved index without rescanning.",
    command: "search_repo_intelligence_symbols",
  },
  {
    id: "dependents",
    description:
      "Find import and symbol edges related to a target path or symbol.",
    command: "get_repo_intelligence_dependents",
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
export const repoIntelligenceIndexerVersion = "path-graph-v8";

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
  ".py": "Python",
  ".rs": "Rust",
  ".sh": "Shell",
  ".swift": "Swift",
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
  const taskPacks = [
    buildRepoTaskContextPack(
      indexed,
      graph,
      "implementation",
      "implementation app feature UI state",
      8_000,
    ),
    buildRepoTaskContextPack(
      indexed,
      graph,
      "verification",
      "test build smoke release validation",
      6_000,
    ),
  ];
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
    taskPacks,
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
    taskPacks: summary.taskPacks?.map((taskPack) => ({
      id: taskPack.id,
      task: taskPack.task,
      budgetTokens: taskPack.budgetTokens,
      fileCount: taskPack.files.length,
      testCount: taskPack.tests.length,
      commandCount: taskPack.commands.length,
      topFiles: taskPack.files.slice(0, 8),
      tests: taskPack.tests.slice(0, 8),
      commands: [...taskPack.commands],
      omittedCount: taskPack.omitted.length,
    })),
    agentRecipes: repoAgentRecipeTemplates.map((recipe) => ({
      ...recipe,
      tools: [...recipe.tools],
      packIds: [...recipe.packIds],
      command: `npm run repo:intelligence -- ${JSON.stringify(repoRoot || ".")} --pack ${recipe.packIds[0]} --format markdown`,
    })),
    agentSessionRecipes: buildRepoAgentSessionRecipes(repoRoot),
    apiQueries: repoAgentApiQueryTemplates.map((query) => ({
      ...query,
      readOnly: true,
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
  const indexFreshness = getRepoIndexFreshness(summary);

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
    indexFreshness,
    safety: {
      readOnly: true,
      excludesSecretLikePaths: true,
      modifiesRepository: false,
      manualProviderRouting: !primaryRepoAgentIds.has(target),
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
  const indexFreshness = getRepoIndexFreshness(summary);
  const freshnessWarning =
    indexFreshness.status === "changed_cache" || indexFreshness.status === "unknown"
      ? `Warning: ${indexFreshness.label}. ${indexFreshness.detail} Refresh before relying on this handoff for current code.`
      : `${indexFreshness.label}: ${indexFreshness.detail}`;
  const configReadinessMarkdown = configReadiness
    ? [
        "## Connector Config Readiness",
        `Planned connector: ${configReadiness.plannedConnectorName} (${configReadiness.plannedConnectorId})`,
        `Automation enabled: ${configReadiness.automationEnabled ? "yes" : "no"}`,
        `Next gate: ${configReadiness.nextGate.label}`,
        configReadiness.safetyNote,
        `Config path strategy: ${configReadiness.safetyDossier.configPathStrategy}`,
        `Account caveat: ${configReadiness.safetyDossier.accountCaveat}`,
        `Rollback strategy: ${configReadiness.safetyDossier.rollbackStrategy}`,
        "Gated steps:",
        ...configReadiness.gatedSteps.map(
          (step) =>
            `- ${step.label}: evidence required: ${step.requiredEvidence.join(" ")}`,
        ),
        "",
      ].join("\n")
    : "";

  return [
    `# ${profile.label} Handoff`,
    "",
    `Repository: ${repoLabel}`,
    `Tool kind: ${profile.toolKind}`,
    `Selected pack: ${selectedPack.title}`,
    `Index freshness: ${freshnessWarning}`,
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

function buildAgentSessionCopySafety(
  summary: RepoIntelligenceSummary,
  freshness: RepoIndexFreshness,
  copyState: ReturnType<typeof getAgentSessionCopyState>,
): AgentSessionCopySafety {
  const hasRealIndex = summary.packs.length > 0 && summary.indexedFiles > 0;

  return {
    hasRealIndex,
    allowsCopy: copyState.status !== "blocked",
    blocksSampleContext: !hasRealIndex,
    excludesSecretLikePaths: true,
    freshnessStatus: freshness.status,
    skippedFileCount: summary.indexMetadata?.skippedFileCount ?? 0,
    reason: copyState.detail,
  };
}

function buildAgentSessionCopyArtifacts(
  copyState: ReturnType<typeof getAgentSessionCopyState>,
): AgentSessionCopyArtifact[] {
  const available = copyState.status !== "blocked";
  const blockedReason = available ? null : copyState.detail;

  return [
    {
      id: "session_summary",
      label: "Session summary",
      format: "markdown",
      available,
      blockedReason,
    },
    {
      id: "full_handoff",
      label: "Full handoff",
      format: "markdown",
      available,
      blockedReason,
    },
    {
      id: "selected_pack",
      label: "Selected pack",
      format: "markdown",
      available,
      blockedReason,
    },
    {
      id: "json_payload",
      label: "JSON payload",
      format: "json",
      available,
      blockedReason,
    },
  ];
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
  const copySafety = buildAgentSessionCopySafety(
    summary,
    freshness,
    copyState,
  );
  const modeRecommendation = recommendAgentSessionMode(options.modeInputs);
  const handoffPayload =
    copyState.status === "blocked"
      ? null
      : buildRepoAgentHandoffPayload(summary, profile.id, packId);
  const taskContext =
    options.taskQuery?.trim() || options.budgetTokens
      ? buildRepoTaskContextPack(
          dedupeRepoFilesByPath(summary.packs.flatMap((pack) => pack.files)),
          summary.graph,
          taskType,
          options.taskQuery?.trim() || taskType,
          options.budgetTokens ?? 8_000,
        )
      : summary.taskPacks?.find((pack) => pack.task === taskType) ??
        summary.taskPacks?.[0] ??
        null;

  return {
    target: profile,
    taskType,
    packId,
    freshness,
    copySafety,
    copyStatus: copyState.status,
    copyDetail: copyState.detail,
    recommendedMode: modeRecommendation.mode,
    recommendedModeReason: modeRecommendation.reason,
    copyArtifacts: buildAgentSessionCopyArtifacts(copyState),
    handoffMarkdown:
      copyState.status === "blocked"
        ? null
        : formatRepoAgentHandoffMarkdown(summary, profile.id, packId),
    handoffPayload,
    taskContext,
    configReadiness: handoffPayload?.configReadiness ?? null,
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

export function formatAgentSessionSummaryMarkdown(
  preparation: Pick<
    AgentSessionPreparation,
    | "target"
    | "taskType"
    | "packId"
    | "freshness"
    | "copyStatus"
    | "copyDetail"
    | "recommendedMode"
    | "recommendedModeReason"
    | "handoffPayload"
    | "taskContext"
    | "configReadiness"
    | "manifest"
  >,
): string | null {
  if (preparation.copyStatus === "blocked" || !preparation.handoffPayload) {
    return null;
  }

  const configReadiness = preparation.configReadiness
    ? [
        "",
        "## Connector Config Readiness",
        `- Planned connector: ${preparation.configReadiness.plannedConnectorName} (${preparation.configReadiness.plannedConnectorId})`,
        `- Next gate: ${preparation.configReadiness.nextGate.label}`,
        `- Automation enabled: ${preparation.configReadiness.automationEnabled ? "yes" : "no"}`,
      ]
    : [];
  const taskContext = preparation.taskContext
    ? [
        "",
        "## Task-Aware Context",
        `- Budget: ${preparation.taskContext.budgetTokens.toLocaleString()} tokens`,
        `- Ranked files: ${preparation.taskContext.files.length.toLocaleString()}`,
        `- Likely tests: ${preparation.taskContext.tests.length.toLocaleString()}`,
        "- Top files:",
        ...preparation.taskContext.files.slice(0, 5).map(
          (file) =>
            `  - ${file.path} (score ${file.score}, ~${file.estimatedTokens.toLocaleString()} tokens): ${file.reasons.join("; ")}`,
        ),
        "- Suggested commands:",
        ...preparation.taskContext.commands.map((command) => `  - ${command}`),
      ]
    : [];

  return [
    `# Start Agent Session Summary: ${preparation.target.label}`,
    "",
    `Repository: ${preparation.handoffPayload.repoRoot}`,
    `Task: ${preparation.taskType}`,
    `Selected pack: ${repoAgentPackLabel(preparation.packId)}`,
    `Copy status: ${preparation.copyStatus}`,
    `Freshness: ${preparation.freshness.label}`,
    `Mode: ${agentSessionModeLabel(preparation.recommendedMode)}`,
    `Mode reason: ${preparation.recommendedModeReason}`,
    `Estimated pack tokens: ${preparation.handoffPayload.pack.estimatedTokens.toLocaleString()}`,
    `Estimated tokens avoided: ${preparation.handoffPayload.pack.estimatedTokensAvoided.toLocaleString()}`,
    `Skipped files: ${(preparation.manifest.totals.indexMetadata?.skippedFileCount ?? 0).toLocaleString()}`,
    "Secret-like paths excluded: yes",
    `Detail: ${preparation.copyDetail}`,
    ...taskContext,
    ...configReadiness,
  ].join("\n");
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

export function repoAgentPackLabel(packId: string) {
  switch (packId) {
    case "implementation":
      return "Implementation pack";
    case "verification":
      return "Verification pack";
    case "handoff":
      return "Handoff pack";
    case "risk_review":
      return "Risk review pack";
    case "release_handoff":
      return "Release handoff pack";
    default:
      return `${packId} pack`;
  }
}

function agentSessionModeLabel(mode: SwitchboardMode) {
  switch (mode) {
    case "full":
      return "Full optimization";
    case "headroom":
      return "Headroom only";
    case "rtk":
      return "RTK only";
    case "off":
      return "Off";
  }
}

function agentSessionFreshnessDetailLabel(freshness: RepoIndexFreshness) {
  const api = freshness.apiAvailable ? "API ready" : "API unavailable";
  const graph = freshness.graphAvailable ? "graph ready" : "graph unavailable";
  const parser = freshness.parserVersion
    ? `parser ${freshness.parserVersion} (${freshness.parserHealth})`
    : "parser unavailable";
  const indexHealth = `index ${freshness.indexHealth}`;
  const indexed =
    freshness.indexedFileCount === null || freshness.indexedFileCount === undefined
      ? "indexed files unknown"
      : `${freshness.indexedFileCount.toLocaleString()} indexed`;
  const skipped =
    freshness.skippedFileCount === null || freshness.skippedFileCount === undefined
      ? "skipped files unknown"
      : `${freshness.skippedFileCount.toLocaleString()} skipped`;
  const detail =
    freshness.status === "fresh" || freshness.status === "none"
      ? null
      : freshness.detail;

  return [api, graph, indexHealth, parser, indexed, skipped, detail]
    .filter(Boolean)
    .join(" · ");
}

export function buildAgentSessionDisplayState(
  preparation: AgentSessionPreparation,
  hasRealIndex: boolean,
): AgentSessionDisplayState {
  const copyArtifactAvailable = (id: AgentSessionCopyArtifact["id"]) =>
    preparation.copyArtifacts.find((artifact) => artifact.id === id)
      ?.available === true;
  const canCopyHandoff =
    hasRealIndex &&
    preparation.handoffMarkdown !== null &&
    copyArtifactAvailable("full_handoff");
  const canCopySummary =
    hasRealIndex &&
    preparation.handoffPayload !== null &&
    copyArtifactAvailable("session_summary");
  const canCopyPayload =
    hasRealIndex &&
    preparation.handoffPayload !== null &&
    copyArtifactAvailable("json_payload");
  const canCopySelectedPack =
    hasRealIndex &&
    preparation.handoffPayload !== null &&
    copyArtifactAvailable("selected_pack");
  const selectedPackTokens =
    preparation.handoffPayload?.pack.estimatedTokens ?? 0;
  const tokensAvoided =
    preparation.handoffPayload?.pack.estimatedTokensAvoided ?? 0;
  const skippedFileCount =
    preparation.manifest.totals.indexMetadata?.skippedFileCount ?? 0;
  const secretExcluded =
    preparation.manifest.safety.excludesSecretLikePaths === true;
  const configReadiness = preparation.configReadiness;

  return {
    targetLabel: preparation.target.label,
    packLabel: repoAgentPackLabel(preparation.packId),
    modeLabel: agentSessionModeLabel(preparation.recommendedMode),
    freshnessLabel: preparation.freshness.label,
    freshnessDetailLabel: agentSessionFreshnessDetailLabel(
      preparation.freshness,
    ),
    contextLabel: hasRealIndex ? "Local repo index" : "Sample preview",
    selectedPackTokensLabel: selectedPackTokens.toLocaleString(),
    tokensAvoidedLabel: tokensAvoided.toLocaleString(),
    skippedFilesLabel: `${skippedFileCount.toLocaleString()} skipped`,
    secretExclusionLabel: secretExcluded
      ? "Secret-like paths excluded"
      : "Secret exclusion unavailable",
    connectorReadinessLabel: configReadiness
      ? `${configReadiness.plannedConnectorName} config gated`
      : null,
    connectorReadinessDetailLabel: configReadiness
      ? `Next gate: ${configReadiness.nextGate.label}; automation enabled: ${
          configReadiness.automationEnabled ? "yes" : "no"
        }`
      : null,
    sampleContextWarning: hasRealIndex
      ? null
      : "Sample preview packs are blocked from copy actions. Index a real local repo first.",
    copyStatus: preparation.copyStatus,
    copyDetail: preparation.copyDetail,
    canCopySummary,
    canCopyHandoff,
    canCopySelectedPack,
    canCopyJson: canCopyPayload,
  };
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
  const testRelationships = (graph.testRelationships ?? [])
    .map(
      (edge) =>
        `- ${edge.testPath} -> ${edge.sourcePath} (${edge.reason})`,
    )
    .slice(0, 8);
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
  if (testRelationships.length) {
    lines.push("", "Test relationships", ...testRelationships);
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

export function buildRepoTaskContextPack(
  files: RepoFileSignal[],
  graph: RepoGraphSummary | null | undefined,
  task: string,
  query: string,
  budgetTokens = 8_000,
): RepoTaskContextPack {
  const included = dedupeRepoFilesByPath(files).filter(
    (file) => file.includeByDefault,
  );
  const queryTerms = normalizeTaskQueryTerms(query || task);
  const graphHints = buildTaskGraphHints(graph);
  const ranked = included
    .filter((file) => file.role !== "test")
    .map((file) => rankRepoFileForTask(file, queryTerms, graphHints))
    .filter((rank) => rank.score > 0)
    .sort(
      (left, right) =>
        right.score - left.score ||
        left.estimatedTokens - right.estimatedTokens ||
        left.path.localeCompare(right.path),
    );

  const selected: RepoFileRank[] = [];
  let tokenTotal = 0;
  for (const rank of ranked) {
    if (
      selected.length > 0 &&
      tokenTotal + rank.estimatedTokens > budgetTokens
    ) {
      continue;
    }
    selected.push(rank);
    tokenTotal += rank.estimatedTokens;
    if (selected.length >= 24) break;
  }

  const selectedPaths = new Set(selected.map((rank) => rank.path));
  const tests = included
    .filter((file) => file.role === "test" && !selectedPaths.has(file.path))
    .map((file) => rankRepoFileForTask(file, queryTerms, graphHints))
    .filter((rank) => rank.score > 0)
    .sort(
      (left, right) =>
        right.score - left.score || left.path.localeCompare(right.path),
    )
    .slice(0, 8);
  const omitted = ranked
    .filter((rank) => !selectedPaths.has(rank.path))
    .slice(0, 12);

  return {
    id: `task_${slugifyTaskId(task)}`,
    task,
    budgetTokens,
    files: selected,
    tests,
    commands: taskCommandsForQuery(task, queryTerms),
    omitted,
  };
}

function normalizeTaskQueryTerms(query: string): string[] {
  return [
    ...new Set(
      query
        .toLowerCase()
        .split(/[^a-z0-9_/-]+/)
        .map((term) => term.trim())
        .filter((term) => term.length >= 3),
    ),
  ].slice(0, 16);
}

function buildTaskGraphHints(graph: RepoGraphSummary | null | undefined) {
  return {
    entrypoints: new Set((graph?.entrypoints ?? []).map((file) => file.path)),
    tests: new Set((graph?.likelyTests ?? []).map((file) => file.path)),
    configHubs: new Set((graph?.configHubs ?? []).map((file) => file.path)),
    dependencyHubs: new Set(
      (graph?.dependencyHubs ?? []).map((file) => file.path),
    ),
    reverseHubs: new Set(
      (graph?.reverseDependencyHubs ?? []).map((node) => node.label),
    ),
  };
}

function rankRepoFileForTask(
  file: RepoFileSignal,
  queryTerms: string[],
  graphHints: ReturnType<typeof buildTaskGraphHints>,
): RepoFileRank {
  let score = 0;
  const reasons: string[] = [];
  const risks: string[] = [];

  const roleScore: Record<RepoFileRole, number> = {
    source: 18,
    test: 14,
    config: 10,
    docs: 6,
    asset: 0,
    lockfile: 2,
    generated: 0,
    unknown: 1,
  };
  score += roleScore[file.role] ?? 0;
  if ((roleScore[file.role] ?? 0) > 0) {
    reasons.push(`${file.role} file`);
  }

  if (graphHints.entrypoints.has(file.path)) {
    score += 18;
    reasons.push("likely entrypoint");
  }
  if (graphHints.tests.has(file.path)) {
    score += 10;
    reasons.push("likely test");
  }
  if (graphHints.configHubs.has(file.path)) {
    score += 8;
    reasons.push("config hub");
  }
  if (graphHints.dependencyHubs.has(file.path)) {
    score += 6;
    reasons.push("dependency hub");
  }
  if (graphHints.reverseHubs.has(file.path)) {
    score += 14;
    reasons.push("reverse dependency hub");
  }

  const normalizedPath = file.path.toLowerCase();
  for (const term of queryTerms) {
    if (normalizedPath.includes(term)) {
      score += 16;
      reasons.push(`path matches "${term}"`);
    }
  }
  if (file.estimatedTokens > 4_000) {
    score -= 8;
    risks.push("large file may crowd out narrower context");
  }
  if (!file.includeByDefault) {
    score = 0;
    risks.push("not included by default");
  }

  return {
    path: file.path,
    score: Math.max(0, score),
    estimatedTokens: file.estimatedTokens,
    reasons: reasons.length ? reasons : ["low-confidence contextual match"],
    risks,
  };
}

function taskCommandsForQuery(task: string, queryTerms: string[]): string[] {
  const joined = `${task} ${queryTerms.join(" ")}`;
  const commands = new Set<string>();
  if (/test|verify|smoke|release|build/.test(joined)) {
    commands.add("npm test");
    commands.add("npm run build");
  }
  if (/release|smoke/.test(joined)) {
    commands.add("npm run smoke:preflight");
    commands.add("npm run release:report:check");
  }
  if (/rust|tauri|desktop/.test(joined)) {
    commands.add("npm run test:desktop");
  }
  if (commands.size === 0) {
    commands.add("npm test");
  }
  return [...commands];
}

function slugifyTaskId(task: string): string {
  return (
    task
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "") || "context"
  );
}

function dedupeRepoFilesByPath(files: RepoFileSignal[]): RepoFileSignal[] {
  return [...new Map(files.map((file) => [file.path, file])).values()];
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
    ...buildPackageDependencyEdges(included, contentByPath),
    ...buildPackageScriptEdges(included, contentByPath),
  ];
  const symbols = buildRepoSymbols(included, contentByPath);
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
    testRelationships: buildTestRelationships(importEdges).slice(0, 12),
    configHubs: included.filter((file) => file.role === "config").slice(0, 12),
    dependencyHubs: files.filter(isDependencyHub).slice(0, 12),
    importEdges,
    reverseDependencyHubs: buildReverseDependencyHubs(included, importEdges),
    symbols,
    symbolEdges,
  };
}

function buildTestRelationships(
  edges: RepoGraphEdge[],
): RepoTestRelationship[] {
  return edges
    .filter((edge) => edge.kind === "test_to_source")
    .map((edge) => ({
      testPath: edge.from,
      sourcePath: edge.to,
      reason: edge.reason,
    }));
}

function buildRepoSymbols(
  files: RepoFileSignal[],
  contentByPath: Map<string, string>,
): RepoSymbol[] {
  const symbols: RepoSymbol[] = [];
  for (const file of files) {
    if (symbols.length >= 200) break;
    const symbolLanguage = [
      "TypeScript",
      "JavaScript",
      "React",
      "Rust",
      "Python",
      "Swift",
      "CSS",
      "HTML",
    ].includes(file.language);
    const markdownDocs = file.role === "docs" && file.language === "Markdown";
    if (!(file.role === "source" || file.role === "test" || markdownDocs)) {
      continue;
    }
    if (!symbolLanguage && !markdownDocs) continue;
    const content = contentByPath.get(file.path);
    if (content) {
      symbols.push(
        ...extractFileSymbols(file, content, 200 - symbols.length),
      );
      continue;
    }
    const name =
      file.path
        .split("/")
        .pop()
        ?.replace(/\.[^.]+$/, "") ?? file.path;
    symbols.push({
      name,
      kind:
        file.language === "Rust" || file.language === "Swift"
          ? "struct"
          : "function",
      file: file.path,
      line: 1,
      parent: null,
    });
  }
  return symbols;
}

function extractFileSymbols(
  file: RepoFileSignal,
  content: string,
  remaining: number,
): RepoSymbol[] {
  if (file.language === "Markdown") {
    return extractMarkdownHeadingSymbols(file, content, remaining);
  }
  const symbols: RepoSymbol[] = [];
  const parents: Array<{ indent: number; name: string }> = [];
  for (const [index, rawLine] of content.split(/\r?\n/).entries()) {
    if (symbols.length >= remaining) break;
    const indent = rawLine.match(/^\s*/)?.[0].length ?? 0;
    while (parents.length && indent <= parents[parents.length - 1].indent) {
      parents.pop();
    }
    const parsed = extractSymbolFromLine(file.language, rawLine.trimStart());
    if (!parsed) continue;
    const parent = parents[parents.length - 1]?.name ?? null;
    if (["class", "struct", "enum", "trait"].includes(parsed.kind)) {
      parents.push({ indent, name: parsed.name });
    }
    symbols.push({ ...parsed, file: file.path, line: index + 1, parent });
  }
  return symbols;
}

function extractMarkdownHeadingSymbols(
  file: RepoFileSignal,
  content: string,
  remaining: number,
): RepoSymbol[] {
  const symbols: RepoSymbol[] = [];
  const parents: Array<{ level: number; name: string }> = [];
  for (const [index, rawLine] of content.split(/\r?\n/).entries()) {
    if (symbols.length >= remaining) break;
    const match = rawLine.match(/^(#{1,6})\s+(.+?)\s*#*\s*$/);
    if (!match) continue;
    const level = match[1].length;
    const name = match[2].trim();
    if (!name) continue;
    while (parents.length && parents[parents.length - 1].level >= level) {
      parents.pop();
    }
    const parent = parents[parents.length - 1]?.name ?? null;
    symbols.push({
      name,
      kind: "heading",
      file: file.path,
      line: index + 1,
      parent,
    });
    parents.push({ level, name });
  }
  return symbols;
}

function extractSymbolFromLine(
  language: string,
  rawLine: string,
): Pick<RepoSymbol, "name" | "kind"> | null {
  const line = rawLine
    .replace(/^(?:public|private|internal|open|fileprivate)\s+/, "")
    .replace(/^(?:final|static|mutating)\s+/, "")
    .replace(/^(?:export\s+)?default\s+/, "")
    .replace(/^(?:export\s+)?(?:async\s+)?/, "")
    .replace(/^pub(?:\([^)]*\))?\s+/, "")
    .replace(/^async\s+/, "");
  const matchName = (pattern: RegExp, kind: RepoSymbolKind) => {
    const match = line.match(pattern);
    return match?.[1] ? { name: match[1], kind } : null;
  };
  if (["TypeScript", "JavaScript", "React"].includes(language)) {
    return (
      matchName(/^function\s+([A-Za-z_$][A-Za-z0-9_$]*)/, "function") ??
      matchName(/^class\s+([A-Za-z_$][A-Za-z0-9_$]*)/, "class") ??
      matchName(/^interface\s+([A-Za-z_$][A-Za-z0-9_$]*)/, "trait") ??
      matchName(/^type\s+([A-Za-z_$][A-Za-z0-9_$]*)/, "trait") ??
      matchName(
        /^(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?:async\s*)?(?:\([^)]*\)|[A-Za-z_$][A-Za-z0-9_$]*)\s*=>/,
        "function",
      ) ??
      matchName(/^(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)/, "const")
    );
  }
  if (language === "Rust") {
    return (
      matchName(/^fn\s+([A-Za-z_][A-Za-z0-9_]*)/, "function") ??
      matchName(/^struct\s+([A-Za-z_][A-Za-z0-9_]*)/, "struct") ??
      matchName(/^enum\s+([A-Za-z_][A-Za-z0-9_]*)/, "enum") ??
      matchName(/^trait\s+([A-Za-z_][A-Za-z0-9_]*)/, "trait") ??
      matchName(/^const\s+([A-Za-z_][A-Za-z0-9_]*)/, "const")
    );
  }
  if (language === "Python") {
    return (
      matchName(/^def\s+([A-Za-z_][A-Za-z0-9_]*)/, "function") ??
      matchName(/^class\s+([A-Za-z_][A-Za-z0-9_]*)/, "class")
    );
  }
  if (language === "Swift") {
    return (
      matchName(/^func\s+([A-Za-z_][A-Za-z0-9_]*)/, "function") ??
      matchName(/^class\s+([A-Za-z_][A-Za-z0-9_]*)/, "class") ??
      matchName(/^struct\s+([A-Za-z_][A-Za-z0-9_]*)/, "struct") ??
      matchName(/^enum\s+([A-Za-z_][A-Za-z0-9_]*)/, "enum") ??
      matchName(/^protocol\s+([A-Za-z_][A-Za-z0-9_]*)/, "trait") ??
      matchName(/^(?:let|var)\s+([A-Za-z_][A-Za-z0-9_]*)/, "const")
    );
  }
  if (language === "CSS") {
    return matchName(/^([.#][A-Za-z_][A-Za-z0-9_-]*)\s*[,>{:.[#\s{]/, "const");
  }
  if (language === "HTML") {
    return (
      matchName(/^(?:<[^>]+\s+id=["'])([A-Za-z_][A-Za-z0-9_-]*)/, "const") ??
      matchName(/^<([A-Za-z][A-Za-z0-9-]*)\b/, "const")
    );
  }
  return null;
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

    for (const specifier of extractImportSpecifiers(content, file.language)) {
      if (
        !specifier.startsWith(".") &&
        !specifier.startsWith("crate:") &&
        !specifier.startsWith("py:") &&
        !specifier.startsWith("repo:")
      )
        continue;
      const target = resolveImportSpecifier(file.path, specifier, byPath);
      if (!target) continue;
      const displaySpecifier = specifier.startsWith("repo:")
        ? specifier.slice("repo:".length)
        : specifier;
      pushUniqueGraphEdge(edges, {
        from: file.path,
        to: target.path,
        kind: "import_reference",
        reason:
          file.language === "Shell"
            ? `script invokes ${displaySpecifier}`
            : `source imports ${displaySpecifier}`,
      });
      if (edges.length >= 80) return edges;
    }
  }

  return edges;
}

function buildPackageDependencyEdges(
  files: RepoFileSignal[],
  contentByPath: Map<string, string>,
): RepoGraphEdge[] {
  const packageJson = files.find((file) => file.path === "package.json");
  const packageContent = contentByPath.get("package.json");
  if (!packageJson || !packageContent) return [];
  const packages = packageDependencyNames(packageContent);
  if (packages.size === 0) return [];

  const edges: RepoGraphEdge[] = [];
  for (const file of files.filter(
    (candidate) => candidate.role === "source" || candidate.role === "test",
  )) {
    const content = contentByPath.get(file.path);
    if (!content) continue;
    for (const specifier of extractImportSpecifiers(content, file.language)) {
      if (specifier.startsWith(".") || specifier.startsWith("crate:")) continue;
      const packageName = packageNameFromSpecifier(specifier);
      if (!packageName || !packages.has(packageName)) continue;
      pushUniqueGraphEdge(edges, {
        from: file.path,
        to: packageJson.path,
        kind: "package_dependency",
        reason: `source imports package ${packageName}`,
      });
      if (edges.length >= 80) return edges;
    }
  }
  return edges;
}

function buildPackageScriptEdges(
  files: RepoFileSignal[],
  contentByPath: Map<string, string>,
): RepoGraphEdge[] {
  const packageJson = files.find((file) => file.path === "package.json");
  const packageContent = contentByPath.get("package.json");
  if (!packageJson || !packageContent) return [];
  const scripts = packageScripts(packageContent);
  if (scripts.size === 0) return [];
  const byPath = new Map(files.map((file) => [file.path, file]));
  const edges: RepoGraphEdge[] = [];
  for (const [scriptName, command] of scripts) {
    for (const specifier of extractShellScriptSpecifiers(command)) {
      const target = resolveImportSpecifier(
        packageJson.path,
        specifier,
        byPath,
      );
      if (!target) continue;
      const displaySpecifier = specifier.startsWith("repo:")
        ? specifier.slice("repo:".length)
        : specifier;
      pushUniqueGraphEdge(edges, {
        from: packageJson.path,
        to: target.path,
        kind: "import_reference",
        reason: `package script ${scriptName} invokes ${displaySpecifier}`,
      });
      if (edges.length >= 80) return edges;
    }
    for (const invokedScript of extractPackageRunSpecifiers(command)) {
      if (!scripts.has(invokedScript)) continue;
      pushUniqueGraphEdge(edges, {
        from: packageJson.path,
        to: `${packageJson.path}#script:${invokedScript}`,
        kind: "import_reference",
        reason: `package script ${scriptName} runs script ${invokedScript}`,
      });
      if (edges.length >= 80) return edges;
    }
  }
  return edges;
}

function packageDependencyNames(packageJson: string): Set<string> {
  try {
    const parsed = JSON.parse(packageJson) as Record<string, unknown>;
    return new Set(
      [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
      ].flatMap((key) =>
        Object.keys((parsed[key] as Record<string, unknown>) ?? {}),
      ),
    );
  } catch {
    return new Set();
  }
}

function packageScripts(packageJson: string): Map<string, string> {
  try {
    const parsed = JSON.parse(packageJson) as Record<string, unknown>;
    const scripts = parsed.scripts as Record<string, unknown> | undefined;
    return new Map(
      Object.entries(scripts ?? {}).filter(
        (entry): entry is [string, string] => typeof entry[1] === "string",
      ),
    );
  } catch {
    return new Map();
  }
}

function extractPackageRunSpecifiers(command: string): string[] {
  const scripts = new Set<string>();
  const pattern = /\b(?:npm|pnpm|yarn|bun)\s+(?:run\s+)?([A-Za-z0-9:_-]+)/g;
  for (const match of command.matchAll(pattern)) {
    const scriptName = match[1];
    if (!scriptName || ["run", "exec", "x", "dlx", "install"].includes(scriptName)) {
      continue;
    }
    scripts.add(scriptName);
  }
  return [...scripts];
}

function packageNameFromSpecifier(specifier: string): string | null {
  if (!specifier || specifier.startsWith(".") || specifier.startsWith("/")) {
    return null;
  }
  if (specifier.startsWith("@")) {
    const [scope, name] = specifier.split("/");
    return scope && name ? `${scope}/${name}` : null;
  }
  return specifier.split("/")[0] ?? null;
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

function extractImportSpecifiers(content: string, language: string): string[] {
  const specifiers: string[] = [];
  if (["TypeScript", "JavaScript", "React"].includes(language)) {
    const patterns = [
      /\bimport\s+(?:type\s+)?(?:[^"']+\s+from\s+)?["']([^"']+)["']/g,
      /\bexport\s+(?:type\s+)?[^"']+\s+from\s+["']([^"']+)["']/g,
      /\brequire\(\s*["']([^"']+)["']\s*\)/g,
    ];
    for (const pattern of patterns) {
      for (const match of content.matchAll(pattern)) {
        if (match[1]) specifiers.push(match[1]);
      }
    }
  }
  if (language === "Rust") {
    for (const line of content.split(/\r?\n/)) {
      const trimmed = line.trim();
      const module = trimmed.match(/^mod\s+([A-Za-z0-9_]+)\s*;/)?.[1];
      if (module) specifiers.push(`./${module}`);
      const cratePath = trimmed.match(/^use\s+crate::([A-Za-z0-9_:]+)/)?.[1];
      if (cratePath) specifiers.push(`crate:${cratePath}`);
    }
  }
  if (language === "Python") {
    for (const line of content.split(/\r?\n/)) {
      specifiers.push(...pythonImportSpecifiers(line.trim()));
    }
  }
  if (language === "Shell") {
    specifiers.push(...extractShellScriptSpecifiers(content));
  }
  if (language === "CSS") {
    specifiers.push(...extractCssAssetSpecifiers(content));
  }
  if (language === "HTML") {
    specifiers.push(...extractHtmlAssetSpecifiers(content));
  }

  return specifiers;
}

function extractCssAssetSpecifiers(content: string): string[] {
  const specifiers = new Set<string>();
  const importPattern =
    /@import\s+(?:url\(\s*)?["']?([^"')\s;]+)["']?\s*\)?/g;
  const urlPattern = /\burl\(\s*["']?([^"')]+)["']?\s*\)/g;
  for (const pattern of [importPattern, urlPattern]) {
    for (const match of content.matchAll(pattern)) {
      const specifier = normalizeAssetSpecifier(match[1]);
      if (specifier) specifiers.add(specifier);
    }
  }
  return [...specifiers];
}

function extractHtmlAssetSpecifiers(content: string): string[] {
  const specifiers = new Set<string>();
  const patterns = [
    /<script\b[^>]*\bsrc=["']([^"']+)["'][^>]*>/gi,
    /<link\b[^>]*\bhref=["']([^"']+)["'][^>]*>/gi,
    /<img\b[^>]*\bsrc=["']([^"']+)["'][^>]*>/gi,
  ];
  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) {
      const specifier = normalizeAssetSpecifier(match[1]);
      if (specifier) specifiers.add(specifier);
    }
  }
  return [...specifiers];
}

function normalizeAssetSpecifier(rawSpecifier: string | undefined): string | null {
  const specifier = rawSpecifier?.trim();
  if (!specifier) return null;
  if (/^(?:https?:|data:|mailto:|tel:|#)/i.test(specifier)) return null;
  return specifier.startsWith("/") ? `repo:${specifier.slice(1)}` : specifier;
}

function extractShellScriptSpecifiers(content: string): string[] {
  const specifiers = new Set<string>();
  const scriptPathPattern =
    /(?:^|[\s;&|()])(?:(?:bash|zsh|sh)\s+)?((?:\.{1,2}\/|scripts\/|bin\/|tools\/)[A-Za-z0-9_./-]+\.sh)\b/g;
  for (const line of content.split(/\r?\n/)) {
    const command = line.split("#")[0] ?? "";
    for (const match of command.matchAll(scriptPathPattern)) {
      const scriptPath = match[1]?.trim();
      if (!scriptPath) continue;
      specifiers.add(
        scriptPath.startsWith(".") ? scriptPath : `repo:${scriptPath}`,
      );
    }
  }
  return [...specifiers];
}

function pythonImportSpecifiers(line: string): string[] {
  const trimmed = line.split("#")[0]?.trim() ?? "";
  if (!trimmed) return [];
  if (trimmed.startsWith("import ")) {
    return trimmed
      .slice("import ".length)
      .split(",")
      .map((part) => part.trim().split(/\s+/)[0])
      .filter(Boolean)
      .map((name) => `py:${name.replace(/\./g, "/")}`);
  }
  if (trimmed.startsWith("from ")) {
    const rest = trimmed.slice("from ".length);
    const [module] = rest.split(/\s+import\s+/);
    if (!module || module === rest) return [];
    const relativePrefix = module.match(/^\.+/)?.[0] ?? "";
    const modulePath = module.slice(relativePrefix.length).replace(/\./g, "/");
    return [`py:${relativePrefix}${modulePath}`];
  }
  return [];
}

function resolveImportSpecifier(
  fromPath: string,
  specifier: string,
  byPath: Map<string, RepoFileSignal>,
): RepoFileSignal | null {
  const fromDir = fromPath.split("/").slice(0, -1).join("/");
  const normalized = specifier.startsWith("crate:")
    ? normalizeRepoPath(
        `${crateSourceRoot(fromPath)}/${specifier.slice("crate:".length).replace(/::/g, "/")}`,
      )
    : specifier.startsWith("py:")
      ? normalizePythonImportPath(fromPath, specifier.slice("py:".length))
    : specifier.startsWith("repo:")
      ? normalizeRepoPath(specifier.slice("repo:".length))
    : normalizeRepoPath(`${fromDir}/${specifier}`);
  const crateParent = specifier.startsWith("crate:")
    ? normalized.split("/").slice(0, -1).join("/")
    : "";
  const candidates = [
    normalized,
    ...(crateParent ? [crateParent] : []),
    `${normalized}.ts`,
    `${normalized}.tsx`,
    `${normalized}.js`,
    `${normalized}.jsx`,
    `${normalized}.mjs`,
    `${normalized}.sh`,
    `${normalized}.py`,
    `${normalized}.rs`,
    `${normalized}.css`,
    `${normalized}.html`,
    ...(crateParent ? [`${crateParent}.rs`, `${crateParent}/mod.rs`] : []),
    `${normalized}.swift`,
    `${normalized}/index.ts`,
    `${normalized}/index.tsx`,
    `${normalized}/index.js`,
    `${normalized}/__init__.py`,
    `${normalized}/mod.rs`,
  ];
  for (const candidate of candidates) {
    const target = byPath.get(candidate);
    if (target) return target;
  }
  return null;
}

function normalizePythonImportPath(fromPath: string, pythonPath: string): string {
  const fromDir = fromPath.split("/").slice(0, -1).join("/");
  if (pythonPath.startsWith(".")) {
    const dotCount = pythonPath.match(/^\.+/)?.[0].length ?? 0;
    const modulePath = pythonPath.slice(dotCount);
    const relativeBase = `${fromDir}${"/..".repeat(
      Math.max(0, dotCount - 1),
    )}`;
    return normalizeRepoPath(`${relativeBase}/${modulePath}`);
  }
  const packageRoot = pythonPackageRoot(fromPath);
  return normalizeRepoPath(
    packageRoot ? `${packageRoot}/${pythonPath}` : pythonPath,
  );
}

function pythonPackageRoot(fromPath: string): string {
  const parts = fromPath.split("/");
  for (let index = parts.length - 1; index >= 0; index -= 1) {
    if (
      ["src", "app", "apps", "lib", "server", "backend"].includes(
        parts[index],
      )
    ) {
      return parts.slice(0, index + 1).join("/");
    }
  }
  return "";
}

function crateSourceRoot(fromPath: string): string {
  const parts = fromPath.split("/");
  const srcIndex = parts.lastIndexOf("src");
  return srcIndex >= 0 ? parts.slice(0, srcIndex + 1).join("/") : "src";
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
