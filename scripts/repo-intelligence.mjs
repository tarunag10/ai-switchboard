#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const INDEXER_VERSION = "path-graph-v3";

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
];

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

const plannedConnectorIdByAgentId = {
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
  {
    id: "detect",
    label: "Detect config surface",
    requiredEvidence: [
      "Read-only binary or app detection result.",
      "Detected config, settings, profile, or environment surface documented without writes.",
    ],
  },
  {
    id: "dryRunDiff",
    label: "Show dry-run diff",
    requiredEvidence: [
      "User-visible dry-run diff artifact showing target, before/after local proxy/provider change, managed marker boundary, rollback preview, and confirmation phrase.",
      "No files, profiles, credentials, or account state changed by the preview.",
    ],
  },
  {
    id: "backup",
    label: "Create backup",
    requiredEvidence: [
      "Timestamped backup path or environment-wrapper restore point.",
      "Fixture-home restore test proving unknown fields and unrelated provider entries are preserved.",
    ],
  },
  {
    id: "apply",
    label: "Apply with consent",
    requiredEvidence: [
      "Explicit user consent captured for the connector and config surface.",
      "Managed marker or wrapper boundary proving only Switchboard-owned routing was applied.",
    ],
  },
  {
    id: "verify",
    label: "Verify in Doctor",
    requiredEvidence: [
      "Doctor check confirming account/model guardrails without storing secrets.",
      "Compatibility or caveat message visible before routing is considered supported.",
    ],
  },
  {
    id: "rollback",
    label: "Rollback safely",
    requiredEvidence: [
      "Fixture-home rollback test restoring the exact backup or removing only managed wrapper state.",
      "Post-rollback diff proving unrelated user settings are unchanged.",
    ],
  },
  {
    id: "offCleanup",
    label: "Clean up in Off mode",
    requiredEvidence: [
      "Fixture-home Off-mode cleanup showing managed routing removed.",
      "Doctor verification that the connector returns to manual or RTK-only mode.",
    ],
  },
];

const plannedConnectorDossiers = {
  gemini_cli: {
    name: "Gemini CLI",
    configPathStrategy:
      "Detect PATH: gemini first, then probe documented provider settings or shell flags read-only.",
    accountCaveat:
      "Model and account compatibility must be reported before routing; no account tokens are stored.",
    rollbackStrategy:
      "Restore the previous provider settings or remove only Switchboard-managed shell routing.",
  },
  opencode: {
    name: "OpenCode",
    configPathStrategy:
      "Detect PATH: opencode, then identify the active provider config path before any write.",
    accountCaveat:
      "Secrets stay in the user's existing provider store and must not be copied into Switchboard state.",
    rollbackStrategy:
      "Restore the timestamped provider-config backup and clear managed environment overrides.",
  },
  cursor: {
    name: "Cursor",
    configPathStrategy:
      "Find the active Cursor app/profile settings surface before reading user settings.",
    accountCaveat:
      "Account-specific model choices remain user-controlled until Doctor can explain compatibility.",
    rollbackStrategy:
      "Restore the exact profile settings backup without touching extension-managed secrets.",
  },
  grok_cli: {
    name: "Grok / xAI CLI",
    configPathStrategy:
      "Detect PATH: grok or PATH: xai and avoid guessing hidden provider files.",
    accountCaveat:
      "Unsupported model/account combinations require Doctor guardrails before setup is offered.",
    rollbackStrategy:
      "Remove managed shell routing and leave API key/account state outside app storage.",
  },
  aider: {
    name: "Aider",
    configPathStrategy:
      "Detect PATH: aider and prefer a one-launch environment wrapper over saved config edits.",
    accountCaveat:
      "Existing provider secrets remain in the user's shell or provider config and are never copied.",
    rollbackStrategy:
      "Drop the wrapper environment and leave the user's Aider/provider files unchanged.",
  },
  continue: {
    name: "Continue",
    configPathStrategy:
      "Open or parse the Continue config folder only after preserving unknown provider fields.",
    accountCaveat:
      "Provider credentials and account selections stay visible and user-owned during guided setup.",
    rollbackStrategy:
      "Restore the exact config backup or remove only the marked Switchboard provider entry.",
  },
  goose: {
    name: "Goose",
    configPathStrategy:
      "Detect PATH: goose and inspect Goose provider/MCP surfaces read-only before handoff.",
    accountCaveat:
      "Provider account state remains outside Switchboard until compatibility checks are explicit.",
    rollbackStrategy:
      "Remove managed provider routing while preserving unrelated Goose MCP configuration.",
  },
  qwen_code: {
    name: "Qwen Code",
    configPathStrategy:
      "Detect PATH: qwen-code or PATH: qwen, then probe provider/model settings read-only.",
    accountCaveat:
      "Qwen account and model compatibility must be verified without editing config.",
    rollbackStrategy:
      "Remove managed shell routing and restore provider settings from the exact backup.",
  },
  amazon_q: {
    name: "Amazon Q Developer CLI",
    configPathStrategy:
      "Detect PATH: q and avoid reading AWS credentials, SSO caches, or profile secrets.",
    accountCaveat:
      "AWS profile, SSO, and credential state must remain outside Switchboard storage.",
    rollbackStrategy:
      "Remove managed routing without modifying AWS config, credentials, SSO cache, or profiles.",
  },
  windsurf: {
    name: "Windsurf",
    configPathStrategy:
      "Detect the Windsurf app and active settings location before showing any write plan.",
    accountCaveat:
      "Account and model settings stay manual until the adapter preserves unknown fields.",
    rollbackStrategy:
      "Restore the active settings backup and remove only Switchboard-managed provider entries.",
  },
  zed_ai: {
    name: "Zed AI",
    configPathStrategy:
      "Detect the Zed app and assistant settings before parsing provider entries.",
    accountCaveat:
      "Provider/account selection stays manual until lossless settings parsing is proven.",
    rollbackStrategy:
      "Restore assistant/provider settings from backup and remove managed local proxy entries.",
  },
};

function buildConfigReadiness(agentId) {
  const plannedConnectorId = plannedConnectorIdByAgentId[agentId];
  if (!plannedConnectorId) return null;
  const dossier = plannedConnectorDossiers[plannedConnectorId] ?? {
    name: plannedConnectorId,
    configPathStrategy:
      "Detect the connector config, settings, profile, or environment surface read-only before any write.",
    accountCaveat:
      "Account, credential, profile, and model choices stay user-owned until Doctor guardrails are explicit.",
    rollbackStrategy:
      "Restore the exact backup or remove only Switchboard-managed routing.",
  };
  const nextGate = plannedConnectorConfigGateSteps[0];
  return {
    plannedConnectorId,
    plannedConnectorName: dossier.name,
    automationEnabled: false,
    safetyNote:
      "Planned connector config creation stays disabled until detection, dry-run diff, backup, apply, verify, rollback, and Off cleanup are implemented and tested.",
    nextGate: {
      id: nextGate.id,
      label: nextGate.label,
    },
    safetyDossier: {
      configPathStrategy: dossier.configPathStrategy,
      accountCaveat: dossier.accountCaveat,
      rollbackStrategy: dossier.rollbackStrategy,
    },
    gatedSteps: plannedConnectorConfigGateSteps.map((step) => ({
      ...step,
      requiredEvidence: [...step.requiredEvidence],
    })),
  };
}

const primaryRepoAgentIds = new Set(["claude", "codex"]);
const agentSessionTaskTypes = new Set([
  "implementation",
  "verification",
  "handoff",
  "risk_review",
  "release_handoff",
]);

function parseArgs(argv) {
  const options = {
    repoRoot: process.cwd(),
    packId: null,
    agent: null,
    session: false,
    taskType: null,
    taskQuery: null,
    budgetTokens: null,
    format: "json",
    formatProvided: false,
    listPacks: false,
    listAgents: false,
    listApi: false,
    manifest: false,
    mcpServe: false,
    headroomHealthy: false,
    rtkHealthy: false,
    providerRoutingSafe: null,
    headroomCompressionRisk: false,
    cleanPassThrough: false,
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
    } else if (arg === "--session") {
      options.session = true;
    } else if (arg === "--task") {
      options.taskType = argv[index + 1] ?? null;
      index += 1;
    } else if (arg.startsWith("--task=")) {
      options.taskType = arg.slice("--task=".length);
    } else if (arg === "--query") {
      options.taskQuery = argv[index + 1] ?? null;
      index += 1;
    } else if (arg.startsWith("--query=")) {
      options.taskQuery = arg.slice("--query=".length);
    } else if (arg === "--budget") {
      options.budgetTokens = Number(argv[index + 1] ?? NaN);
      index += 1;
    } else if (arg.startsWith("--budget=")) {
      options.budgetTokens = Number(arg.slice("--budget=".length));
    } else if (arg === "--headroom-healthy") {
      options.headroomHealthy = true;
    } else if (arg === "--rtk-healthy") {
      options.rtkHealthy = true;
    } else if (arg === "--provider-routing-safe") {
      options.providerRoutingSafe = true;
    } else if (arg === "--provider-routing-unsafe") {
      options.providerRoutingSafe = false;
    } else if (arg === "--headroom-risk") {
      options.headroomCompressionRisk = true;
    } else if (arg === "--clean-pass-through") {
      options.cleanPassThrough = true;
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
    } else if (arg === "--list-api") {
      options.listApi = true;
    } else if (arg === "--manifest") {
      options.manifest = true;
    } else if (arg === "--mcp-serve") {
      options.mcpServe = true;
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
  --pack <id>          Print one context pack: implementation, verification, handoff, risk_review, release_handoff
  --agent <id>         Print agent handoff: claude, codex, gemini, opencode, aider, goose, cursor, continue, grok, qwen, amazonq, windsurf, zed
  --session            Print Start Agent Session preparation for --agent
  --task <type>        Session task: implementation, verification, handoff, risk_review, release_handoff
  --query <text>       Optional free-form task query for task-aware context ranking
  --budget <tokens>    Optional token budget for task-aware context ranking
  --headroom-healthy   Mark Headroom engine healthy for mode recommendation
  --rtk-healthy        Mark RTK healthy for mode recommendation
  --provider-routing-safe|--provider-routing-unsafe
                       Override provider-routing safety for mode recommendation
  --headroom-risk      Prefer RTK when Headroom compression is risky
  --clean-pass-through Prefer Off for clean debugging
  --format <format>   json or markdown
  --list-packs        Print available pack ids
  --list-agents       Print available agent handoff ids
  --list-api          Print read-only local API query command names
  --manifest          Print agent-readable pack manifest JSON
  --mcp-serve         Serve read-only repo-memory MCP tools over stdio
  --help              Show this help

Examples:
  npm run repo:intelligence -- .
  npm run repo:intelligence -- . --manifest
  npm run repo:intelligence -- . --list-api
  npm run repo:intelligence -- . --list-agents
  npm run repo:intelligence -- . --pack implementation --format markdown
  npm run repo:intelligence -- . --agent codex --format markdown
  npm run repo:intelligence -- . --session --agent codex --task verification --headroom-healthy --rtk-healthy --format markdown
  npm run repo:intelligence -- . --session --agent codex --task verification --query "release readiness schema smoke evidence" --budget 6000 --format markdown
  npm run repo:intelligence -- . --agent gemini --format json
  npm run repo:intelligence -- . --mcp-serve`);
}

function walk(repoRoot, dir = repoRoot, files = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (ignoredSegments.has(entry.name)) continue;
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(repoRoot, absolute, files);
    } else if (entry.isFile()) {
      const relative = path
        .relative(repoRoot, absolute)
        .split(path.sep)
        .join("/");
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
  const name =
    normalized.split("/").pop()?.toLowerCase() ?? normalized.toLowerCase();
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
  const reasons = [];
  let role = "unknown";

  if (lockfileNames.has(name)) {
    role = "lockfile";
    reasons.push("package lockfile");
  } else if (
    lower.includes(".test.") ||
    lower.includes(".spec.") ||
    lower.includes("/tests/")
  ) {
    role = "test";
    reasons.push("test path");
  } else if (lower.startsWith("docs/") || extension === ".md") {
    role = "docs";
    reasons.push("documentation");
  } else if ([".json", ".toml", ".yaml", ".yml", ".sh"].includes(extension)) {
    role = "config";
    reasons.push("configuration or script");
  } else if (
    [".ts", ".tsx", ".js", ".jsx", ".rs", ".css", ".html"].includes(extension)
  ) {
    role = "source";
    reasons.push("source extension");
  } else if (
    [".svg", ".png", ".jpg", ".jpeg", ".gif", ".ico", ".webp"].includes(
      extension,
    )
  ) {
    role = "asset";
    reasons.push("asset file");
  }

  const secretLike = isSecretLikePath(filePath);
  if (secretLike) {
    reasons.push("secret-like path excluded");
  }
  const includeByDefault =
    role !== "asset" && role !== "lockfile" && !secretLike;

  return {
    path: filePath,
    role,
    language: languageByExtension[extension] ?? "Unknown",
    estimatedTokens: estimateTokens(bytes),
    includeByDefault,
    reasons,
  };
}

function pack(id, title, purpose, files, estimatedFullScanTokens) {
  const sorted = [...files]
    .sort(
      (a, b) =>
        a.estimatedTokens - b.estimatedTokens || a.path.localeCompare(b.path),
    )
    .slice(0, 40);
  const estimatedTokens = sorted.reduce(
    (sum, file) => sum + file.estimatedTokens,
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
    estimatedTokens,
    savingsVsFullScanPct,
    files: sorted,
  };
}

function buildTaskContextPack(files, graph, task, query, budgetTokens = 8000) {
  const included = dedupeFilesByPath(files).filter((file) => file.includeByDefault);
  const queryTerms = normalizeTaskQueryTerms(query || task);
  const graphHints = buildTaskGraphHints(graph);
  const ranked = included
    .filter((file) => file.role !== "test")
    .map((file) => rankFileForTask(file, queryTerms, graphHints))
    .filter((rank) => rank.score > 0)
    .sort(
      (left, right) =>
        right.score - left.score ||
        left.estimatedTokens - right.estimatedTokens ||
        left.path.localeCompare(right.path),
    );
  const selected = [];
  let tokenTotal = 0;
  for (const rank of ranked) {
    if (selected.length > 0 && tokenTotal + rank.estimatedTokens > budgetTokens) {
      continue;
    }
    selected.push(rank);
    tokenTotal += rank.estimatedTokens;
    if (selected.length >= 24) break;
  }
  const selectedPaths = new Set(selected.map((rank) => rank.path));
  const tests = included
    .filter((file) => file.role === "test" && !selectedPaths.has(file.path))
    .map((file) => rankFileForTask(file, queryTerms, graphHints))
    .filter((rank) => rank.score > 0)
    .sort((left, right) => right.score - left.score || left.path.localeCompare(right.path))
    .slice(0, 8);
  return {
    id: `task_${slugifyTaskId(task)}`,
    task,
    budgetTokens,
    files: selected,
    tests,
    commands: taskCommandsForQuery(task, queryTerms),
    omitted: ranked.filter((rank) => !selectedPaths.has(rank.path)).slice(0, 12),
  };
}

function normalizeTaskQueryTerms(query) {
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

function buildTaskGraphHints(graph) {
  return {
    entrypoints: new Set((graph?.entrypoints ?? []).map((file) => file.path)),
    tests: new Set((graph?.likelyTests ?? []).map((file) => file.path)),
    configHubs: new Set((graph?.configHubs ?? []).map((file) => file.path)),
    dependencyHubs: new Set((graph?.dependencyHubs ?? []).map((file) => file.path)),
    reverseHubs: new Set((graph?.reverseDependencyHubs ?? []).map((node) => node.label)),
  };
}

function rankFileForTask(file, queryTerms, graphHints) {
  const roleScore = {
    source: 18,
    test: 14,
    config: 10,
    docs: 6,
    asset: 0,
    lockfile: 2,
    generated: 0,
    unknown: 1,
  };
  let score = roleScore[file.role] ?? 0;
  const reasons = score > 0 ? [`${file.role} file`] : [];
  const risks = [];
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
  if (file.estimatedTokens > 4000) {
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

function taskCommandsForQuery(task, queryTerms) {
  const joined = `${task} ${queryTerms.join(" ")}`;
  const commands = new Set();
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

function slugifyTaskId(task) {
  return task.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "") || "context";
}

function normalizedTaskBudget(value, fallback = 8000) {
  return Number.isFinite(value) && value > 0 ? Math.round(value) : fallback;
}

function dedupeFilesByPath(files) {
  return [...new Map(files.map((file) => [file.path, file])).values()];
}

function buildGraphSummary(repoRoot, files) {
  const included = files.filter((file) => file.includeByDefault);
  const sourceAndConfig = included.filter(
    (file) => file.role === "source" || file.role === "config",
  );
  const importEdges = [
    ...buildGraphEdges(included),
    ...buildImportReferenceEdges(repoRoot, included),
    ...buildPackageDependencyEdges(repoRoot, included),
  ];
  const symbols = buildRepoSymbols(repoRoot, included);
  const symbolEdges = [
    ...buildSymbolEdges(included, symbols),
    ...buildCallReferenceEdges(repoRoot, included, symbols),
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

function buildRepoSymbols(repoRoot, files) {
  const symbols = [];
  for (const file of files) {
    if (symbols.length >= 200) break;
    if (file.role !== "source" && file.role !== "test") continue;
    if (
      !["TypeScript", "JavaScript", "React", "Rust", "Python"].includes(
        file.language,
      )
    )
      continue;
    let content = "";
    try {
      content = fs.readFileSync(path.join(repoRoot, file.path), "utf8");
    } catch {
      continue;
    }
    symbols.push(...extractFileSymbols(file, content, 200 - symbols.length));
  }
  return symbols;
}

function extractFileSymbols(file, content, remaining) {
  const symbols = [];
  const parents = [];
  for (const [index, rawLine] of content.split(/\r?\n/).entries()) {
    if (symbols.length >= remaining) break;
    const indent = rawLine.match(/^\s*/)?.[0].length ?? 0;
    while (parents.length && indent <= parents.at(-1).indent) parents.pop();
    const parsed = extractSymbolFromLine(file.language, rawLine.trimStart());
    if (!parsed) continue;
    const parent = parents.at(-1)?.name ?? null;
    if (["class", "struct", "enum", "trait"].includes(parsed.kind)) {
      parents.push({ indent, name: parsed.name });
    }
    symbols.push({ ...parsed, file: file.path, line: index + 1, parent });
  }
  return symbols;
}

function extractSymbolFromLine(language, rawLine) {
  const line = rawLine
    .replace(/^(pub|async|export|default)\s+/, "")
    .replace(/^(pub|async|export|default)\s+/, "");
  const pick = (prefix, kind) => {
    if (!line.startsWith(prefix)) return null;
    const match = line.slice(prefix.length).match(/^[A-Za-z_$][A-Za-z0-9_$]*/);
    return match ? { name: match[0], kind } : null;
  };
  if (["TypeScript", "JavaScript", "React"].includes(language)) {
    return (
      pick("function ", "function") ??
      pick("class ", "class") ??
      pick("interface ", "trait") ??
      pick("type ", "trait") ??
      pick("const ", "const")
    );
  }
  if (language === "Rust") {
    return (
      pick("fn ", "function") ??
      pick("struct ", "struct") ??
      pick("enum ", "enum") ??
      pick("trait ", "trait") ??
      pick("const ", "const")
    );
  }
  if (language === "Python") {
    return pick("def ", "function") ?? pick("class ", "class");
  }
  return null;
}

function buildSymbolEdges(files, symbols) {
  const edges = [];
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
      )
        continue;
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

function buildImportReferenceEdges(repoRoot, files) {
  const byPath = new Map(files.map((file) => [file.path, file]));
  const edges = [];
  for (const file of files.filter(
    (candidate) => candidate.role === "source" || candidate.role === "test",
  )) {
    let content = "";
    try {
      content = fs.readFileSync(path.join(repoRoot, file.path), "utf8");
    } catch {
      continue;
    }
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

function buildPackageDependencyEdges(repoRoot, files) {
  const packageJson = files.find((file) => file.path === "package.json");
  if (!packageJson) return [];
  let packageContent = "";
  try {
    packageContent = fs.readFileSync(path.join(repoRoot, packageJson.path), "utf8");
  } catch {
    return [];
  }
  const packages = packageDependencyNames(packageContent);
  if (!packages.size) return [];

  const edges = [];
  for (const file of files.filter(
    (candidate) => candidate.role === "source" || candidate.role === "test",
  )) {
    let content = "";
    try {
      content = fs.readFileSync(path.join(repoRoot, file.path), "utf8");
    } catch {
      continue;
    }
    for (const specifier of extractImportSpecifiers(content)) {
      if (specifier.startsWith(".")) continue;
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

function packageDependencyNames(packageJson) {
  try {
    const parsed = JSON.parse(packageJson);
    return new Set(
      [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
      ].flatMap((key) => Object.keys(parsed[key] ?? {})),
    );
  } catch {
    return new Set();
  }
}

function packageNameFromSpecifier(specifier) {
  if (!specifier || specifier.startsWith(".") || specifier.startsWith("/")) {
    return null;
  }
  if (specifier.startsWith("@")) {
    const [scope, name] = specifier.split("/");
    return scope && name ? `${scope}/${name}` : null;
  }
  return specifier.split("/")[0] ?? null;
}

function buildCallReferenceEdges(repoRoot, files, symbols) {
  const callableSymbols = symbols
    .filter((symbol) => symbol.kind === "function" || symbol.kind === "const")
    .slice(0, 120);
  const edges = [];
  for (const file of files.filter(
    (candidate) => candidate.role === "source" || candidate.role === "test",
  )) {
    let content = "";
    try {
      content = fs.readFileSync(path.join(repoRoot, file.path), "utf8");
    } catch {
      continue;
    }
    for (const symbol of callableSymbols) {
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

function extractImportSpecifiers(content) {
  const specifiers = [];
  const patterns = [
    /\bimport\s+(?:type\s+)?(?:[^"']+\s+from\s+)?["']([^"']+)["']/g,
    /\bexport\s+(?:type\s+)?[^"']+\s+from\s+["']([^"']+)["']/g,
    /\brequire\(\s*["']([^"']+)["']\s*\)/g,
    /\bmod\s+([A-Za-z0-9_]+)\s*;/g,
  ];
  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) {
      if (match[1]) specifiers.push(match[1]);
    }
  }
  return specifiers;
}

function resolveImportSpecifier(fromPath, specifier, byPath) {
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

function normalizeRepoPath(filePath) {
  const parts = [];
  for (const part of filePath.split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") {
      parts.pop();
      continue;
    }
    parts.push(part);
  }
  return parts.join("/");
}

function pushUniqueGraphEdge(edges, edge) {
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

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function buildGraphEdges(files) {
  const byPath = new Map(files.map((file) => [file.path, file]));
  const dependencyHubs = files.filter(isDependencyHub);
  const configHubs = files.filter((file) => file.role === "config");
  const edges = [];
  const pushEdge = (edge) => {
    if (edge.from === edge.to || edges.length >= 24) return;
    if (
      edges.some(
        (existing) =>
          existing.from === edge.from &&
          existing.to === edge.to &&
          existing.kind === edge.kind,
      )
    )
      return;
    edges.push(edge);
  };
  for (const file of files) {
    if (file.role === "test") {
      const target = findTestTarget(file, byPath);
      if (target)
        pushEdge({
          from: file.path,
          to: target.path,
          kind: "test_to_source",
          reason: "test filename matches source module",
        });
    }
    if (isLikelyEntrypoint(file)) {
      const config = findNearestConfigHub(file, configHubs);
      if (config)
        pushEdge({
          from: file.path,
          to: config.path,
          kind: "entrypoint_to_config",
          reason: "entrypoint shares closest config surface",
        });
    }
    if (file.role === "source") {
      const dependencyHub = findNearestDependencyHub(file, dependencyHubs);
      if (dependencyHub)
        pushEdge({
          from: file.path,
          to: dependencyHub.path,
          kind: "source_to_dependency_hub",
          reason: "source file belongs to dependency hub scope",
        });
    }
  }
  return edges;
}

function buildReverseDependencyHubs(files, edges) {
  const byPath = new Map(files.map((file) => [file.path, file]));
  const inbound = new Map();
  for (const edge of edges) {
    const target = byPath.get(edge.to);
    const node = inbound.get(edge.to) ?? {
      label: edge.to,
      count: 0,
      estimatedTokens: target?.estimatedTokens ?? 0,
      examples: [],
    };
    node.count += 1;
    if (node.examples.length < 4) node.examples.push(edge.from);
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

function findTestTarget(testFile, byPath) {
  return testTargetCandidates(testFile.path)
    .map((candidate) => byPath.get(candidate))
    .find(Boolean);
}

function testTargetCandidates(filePath) {
  const extension = extensionForPath(filePath);
  const withoutExtension = extension
    ? filePath.slice(0, -extension.length)
    : filePath;
  const base = withoutExtension.replace(/\.(test|spec)$/i, "");
  if (base === withoutExtension) return [];
  const extensions = [extension, ".tsx", ".ts", ".jsx", ".js", ".rs"].filter(
    Boolean,
  );
  return [
    ...new Set(
      extensions.map((candidateExtension) => `${base}${candidateExtension}`),
    ),
  ];
}

function findNearestConfigHub(file, configHubs) {
  return (
    nearestScopedFile(file, configHubs) ??
    configHubs.find((candidate) => !candidate.path.includes("/"))
  );
}

function findNearestDependencyHub(file, dependencyHubs) {
  return (
    nearestScopedFile(file, dependencyHubs) ??
    dependencyHubs.find((candidate) => !candidate.path.includes("/"))
  );
}

function nearestScopedFile(file, candidates) {
  return candidates
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
    )[0]?.candidate;
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
    (node) =>
      "- " +
      node.label +
      ": " +
      node.count +
      " files, ~" +
      node.estimatedTokens.toLocaleString() +
      " tokens",
  );
  const languages = graph.topLanguages.map(
    (node) => "- " + node.label + ": " + node.count + " files",
  );
  const entrypoints = graph.entrypoints.map(
    (file) => "- " + file.path + " (" + file.language + ")",
  );
  const tests = graph.likelyTests.map((file) => "- " + file.path);
  const config = graph.configHubs.map((file) => "- " + file.path);
  const dependencies = (graph.dependencyHubs ?? []).map(
    (file) => "- " + file.path,
  );
  const importEdges = (graph.importEdges ?? []).map(
    (edge) =>
      "- " +
      edge.from +
      " -> " +
      edge.to +
      " (" +
      edge.kind +
      ": " +
      edge.reason +
      ")",
  );
  const symbols = (graph.symbols ?? []).map(
    (symbol) =>
      "- " +
      symbol.name +
      " (" +
      symbol.kind +
      ") in " +
      symbol.file +
      ":" +
      symbol.line,
  );
  const symbolEdges = (graph.symbolEdges ?? []).map(
    (edge) =>
      "- " +
      edge.from +
      " -> " +
      edge.to +
      " (" +
      edge.kind +
      ": " +
      edge.reason +
      ")",
  );
  const reverseDependencyHubs = (graph.reverseDependencyHubs ?? []).map(
    (node) => "- " + node.label + ": " + node.count + " inbound links",
  );
  if (directories.length) lines.push("", "Top directories", ...directories);
  if (languages.length) lines.push("", "Top languages", ...languages);
  if (entrypoints.length) lines.push("", "Likely entrypoints", ...entrypoints);
  if (tests.length) lines.push("", "Likely tests", ...tests);
  if (config.length) lines.push("", "Config hubs", ...config);
  if (dependencies.length) lines.push("", "Dependency hubs", ...dependencies);
  if (symbols.length) lines.push("", "Symbols", ...symbols.slice(0, 12));
  if (symbolEdges.length)
    lines.push("", "Symbol edges", ...symbolEdges.slice(0, 8));
  if (importEdges.length)
    lines.push("", "Import and dependency edges", ...importEdges.slice(0, 8));
  if (reverseDependencyHubs.length)
    lines.push(
      "",
      "Reverse dependency hubs",
      ...reverseDependencyHubs.slice(0, 8),
    );
  return lines.join("\n");
}

function buildIndexMetadata(repoRoot, files, signals) {
  const includeByPath = new Map(
    signals.map((signal) => [signal.path, signal.includeByDefault]),
  );
  const signalByPath = new Map(signals.map((signal) => [signal.path, signal]));
  const fileFingerprints = files
    .filter((file) => includeByPath.get(file.path) === true)
    .map((file) => ({
      path: file.path,
      bytes: file.bytes,
      modifiedUnixMs: 0,
      fingerprint: hashString(file.path + ":" + file.bytes),
    }))
    .sort((a, b) => a.path.localeCompare(b.path));
  const fingerprintByPath = new Map(
    fileFingerprints.map((entry) => [entry.path, entry]),
  );
  const skippedFiles = signals
    .filter((signal) => !signal.includeByDefault)
    .map((signal) => ({
      path: signal.reasons?.includes("secret-like path excluded")
        ? "<secret-like path>"
        : signal.path,
      role: signal.role,
      reasons: signal.reasons?.length
        ? signal.reasons
        : ["not included in default repo index"],
    }))
    .sort((a, b) => a.path.localeCompare(b.path));
  const graphInputs = files
    .filter((file) => includeByPath.get(file.path) === true)
    .map((file) => {
      const signal = signalByPath.get(file.path);
      const fingerprint = fingerprintByPath.get(file.path);
      return signal && fingerprint
        ? {
            path: file.path,
            role: signal.role,
            language: signal.language,
            bytes: fingerprint.bytes,
            fingerprint: fingerprint.fingerprint,
          }
        : null;
    })
    .filter(
      (entry) =>
        entry &&
        (entry.role === "source" ||
          entry.role === "test" ||
          entry.role === "config"),
    )
    .sort((a, b) => a.path.localeCompare(b.path));
  const cacheKey = hashString(
    [
      "1",
      INDEXER_VERSION,
      "metadata-fingerprint-v1",
      repoRoot,
      ...fileFingerprints.map(
        (entry) => entry.path + ":" + entry.bytes + ":" + entry.fingerprint,
      ),
      ...graphInputs.map(
        (entry) =>
          "graph:" + entry.path + ":" + entry.role + ":" + entry.fingerprint,
      ),
    ].join("|"),
  );
  return {
    schemaVersion: 1,
    indexerVersion: INDEXER_VERSION,
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

function hashString(value) {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

function buildSummary(repoRoot) {
  const files = walk(repoRoot);
  const signals = files.map((file) => classify(file.path, file.bytes));
  const indexable = signals.filter((file) => file.includeByDefault);
  const estimatedFullScanTokens = signals.reduce(
    (sum, file) => sum + file.estimatedTokens,
    0,
  );
  const roleCounts = signals.reduce((counts, file) => {
    counts[file.role] = (counts[file.role] ?? 0) + 1;
    return counts;
  }, {});
  const indexMetadata = buildIndexMetadata(repoRoot, files, signals);
  const graph = buildGraphSummary(repoRoot, indexable);
  const taskPacks = [
    buildTaskContextPack(
      indexable,
      graph,
      "implementation",
      "implementation app feature UI state",
      8000,
    ),
    buildTaskContextPack(
      indexable,
      graph,
      "verification",
      "test build smoke release validation",
      6000,
    ),
  ];

  return {
    repoRoot,
    indexedAt: indexMetadata.generatedAt,
    indexerVersion: INDEXER_VERSION,
    totalFiles: signals.length,
    indexedFiles: indexable.length,
    skippedFiles: signals.length - indexable.length,
    estimatedFullScanTokens,
    roleCounts,
    indexMetadata,
    graph,
    taskPacks,
    packs: [
      pack(
        "implementation",
        "Implementation Pack",
        "Source files likely needed feature work.",
        indexable.filter(
          (file) => file.role === "source" || file.role === "config",
        ),
        estimatedFullScanTokens,
      ),
      pack(
        "verification",
        "Verification Pack",
        "Tests, scripts, config likely needed before committing.",
        indexable.filter(
          (file) => file.role === "test" || file.role === "config",
        ),
        estimatedFullScanTokens,
      ),
      pack(
        "handoff",
        "Handoff Pack",
        "Docs project metadata useful for another agent or maintainer.",
        indexable.filter(
          (file) => file.role === "docs" || file.role === "config",
        ),
        estimatedFullScanTokens,
      ),
      pack(
        "risk_review",
        "Risk Review Pack",
        "Source, tests, and config likely needed for regression or security review.",
        indexable.filter(
          (file) =>
            file.role === "source" ||
            file.role === "test" ||
            file.role === "config",
        ),
        estimatedFullScanTokens,
      ),
      pack(
        "release_handoff",
        "Release Handoff Pack",
        "Verification, docs, and config useful for release readiness handoff.",
        indexable.filter(
          (file) =>
            file.role === "test" ||
            file.role === "docs" ||
            file.role === "config",
        ),
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
    "Safety: read-only context pack; secret-like paths excluded; repository not modified.",
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
  const profile = agentHandoffProfiles.find(
    (candidate) => candidate.id === agentId,
  );
  if (!profile) {
    throw new Error(
      `Unknown agent: ${agentId}. Available agents: ${agentHandoffProfiles
        .map((candidate) => candidate.id)
        .join(", ")}`,
    );
  }

  const selectedPack =
    summary.packs.find(
      (contextPack) =>
        contextPack.id === (requestedPackId ?? profile.defaultPackId),
    ) ??
    summary.packs.find(
      (contextPack) => contextPack.id === profile.defaultPackId,
    ) ??
    summary.packs[0];

  if (!selectedPack) {
    throw new Error("No repo intelligence packs are available.");
  }
  const configReadiness = buildConfigReadiness(profile.id);
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
    configReadiness
      ? "Connector readiness payload in agent handoffs: do not create or modify this connector's config unless every gated config-creation step is implemented and verified."
      : "",
    "",
    configReadinessMarkdown,
    formatSinglePackMarkdown(summary, selectedPack),
  ]
    .filter((line) => line !== "")
    .join("\n");
}

function buildAgentHandoffPayload(summary, agentId, requestedPackId) {
  const profile = agentHandoffProfiles.find(
    (candidate) => candidate.id === agentId,
  );
  if (!profile) {
    throw new Error(
      `Unknown agent: ${agentId}. Available agents: ${agentHandoffProfiles
        .map((candidate) => candidate.id)
        .join(", ")}`,
    );
  }

  const selectedPack =
    summary.packs.find(
      (contextPack) =>
        contextPack.id === (requestedPackId ?? profile.defaultPackId),
    ) ??
    summary.packs.find(
      (contextPack) => contextPack.id === profile.defaultPackId,
    ) ??
    summary.packs[0];

  if (!selectedPack) {
    throw new Error("No repo intelligence packs available.");
  }
  const configReadiness = buildConfigReadiness(profile.id);

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

function packIdForTask(profile, taskType) {
  if (taskType === "implementation") return "implementation";
  if (taskType === "verification") return "verification";
  if (taskType === "handoff") return "handoff";
  if (taskType === "risk_review") return "risk_review";
  if (taskType === "release_handoff") return "release_handoff";
  return profile.defaultPackId;
}

function getIndexFreshness(summary) {
  const metadata = summary.indexMetadata;
  const base = {
    apiAvailable: true,
    graphAvailable: Boolean(summary.graph),
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

function recommendSessionMode({
  headroomHealthy,
  rtkHealthy,
  providerRoutingSafe,
  headroomCompressionRisk,
  cleanPassThrough,
}) {
  if (cleanPassThrough) {
    return {
      mode: "off",
      reason: "Clean pass-through/debugging was requested.",
    };
  }
  if (headroomHealthy && rtkHealthy && providerRoutingSafe) {
    return {
      mode: "full",
      reason: "Headroom engine and RTK are healthy.",
    };
  }
  if (headroomHealthy && !rtkHealthy && providerRoutingSafe) {
    return {
      mode: "headroom",
      reason: "Headroom engine is healthy; RTK is unavailable.",
    };
  }
  if (rtkHealthy && (!providerRoutingSafe || headroomCompressionRisk)) {
    return {
      mode: "rtk",
      reason:
        "RTK is healthy while provider routing is unsafe or Headroom compression is risky.",
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

function getSessionCopyState(summary, freshness) {
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

function buildAgentSessionPreparation(summary, options) {
  const agentId = options.agent ?? "codex";
  const profile = agentHandoffProfiles.find(
    (candidate) => candidate.id === agentId,
  );
  if (!profile) {
    throw new Error(
      `Unknown agent: ${agentId}. Available agents: ${agentHandoffProfiles
        .map((candidate) => candidate.id)
        .join(", ")}`,
    );
  }
  const taskType = options.taskType ?? profile.defaultPackId;
  if (!agentSessionTaskTypes.has(taskType)) {
    throw new Error(
      `Unknown task: ${taskType}. Available tasks: ${[
        ...agentSessionTaskTypes,
      ].join(", ")}`,
    );
  }
  const packId = packIdForTask(profile, taskType);
  const providerRoutingSafe =
    options.providerRoutingSafe ?? primaryRepoAgentIds.has(profile.id);
  const freshness = getIndexFreshness(summary);
  const copyState = getSessionCopyState(summary, freshness);
  const copyArtifactAvailable = copyState.status !== "blocked";
  const copyArtifacts = [
    {
      id: "full_handoff",
      label: "Full handoff",
      format: "markdown",
      available: copyArtifactAvailable,
      blockedReason: copyArtifactAvailable ? null : copyState.detail,
    },
    {
      id: "selected_pack",
      label: "Selected pack",
      format: "markdown",
      available: copyArtifactAvailable,
      blockedReason: copyArtifactAvailable ? null : copyState.detail,
    },
    {
      id: "json_payload",
      label: "JSON payload",
      format: "json",
      available: copyArtifactAvailable,
      blockedReason: copyArtifactAvailable ? null : copyState.detail,
    },
  ];
  const modeRecommendation = recommendSessionMode({
    headroomHealthy: options.headroomHealthy,
    rtkHealthy: options.rtkHealthy,
    providerRoutingSafe,
    headroomCompressionRisk: options.headroomCompressionRisk,
    cleanPassThrough: options.cleanPassThrough,
  });
  const handoff =
    copyState.status === "blocked"
      ? null
      : buildAgentHandoffPayload(summary, profile.id, packId);
  const taskContext =
    options.taskQuery?.trim() || options.budgetTokens
      ? buildTaskContextPack(
          dedupeFilesByPath(summary.packs.flatMap((contextPack) => contextPack.files)),
          summary.graph,
          taskType,
          options.taskQuery?.trim() || taskType,
          normalizedTaskBudget(options.budgetTokens),
        )
      : summary.taskPacks?.find((pack) => pack.task === taskType) ??
        summary.taskPacks?.[0] ??
        null;
  return {
    schemaVersion: 1,
    kind: "mac_ai_switchboard.agent_session_preparation",
    repoRoot: summary.repoRoot,
    target: {
      id: profile.id,
      label: profile.label,
      toolKind: profile.toolKind,
      guidance: profile.guidance,
    },
    taskType,
    packId,
    freshness,
    copyStatus: copyState.status,
    copyDetail: copyState.detail,
    copyArtifacts,
    recommendedMode: modeRecommendation.mode,
    recommendedModeReason: modeRecommendation.reason,
    safety: {
      readOnly: true,
      excludesSecretLikePaths: true,
      modifiesRepository: false,
      manualProviderRouting: !providerRoutingSafe,
    },
    handoff,
    taskContext,
    configReadiness: handoff?.configReadiness ?? null,
    handoffMarkdown:
      copyState.status === "blocked"
        ? null
        : formatAgentHandoffMarkdown(summary, profile.id, packId),
    manifest: buildAgentManifest(summary),
  };
}

function formatAgentSessionMarkdown(preparation) {
  const lines = [
    `# Start Agent Session: ${preparation.target.label}`,
    "",
    `Repository: ${preparation.repoRoot}`,
    `Task: ${preparation.taskType}`,
    `Selected pack: ${preparation.packId}`,
    `Copy status: ${preparation.copyStatus}`,
    `Freshness: ${preparation.freshness.label}`,
    `Recommended mode: ${preparation.recommendedMode}`,
    `Mode reason: ${preparation.recommendedModeReason}`,
    `Copy artifacts: ${preparation.copyArtifacts
      .map((artifact) => `${artifact.label}=${artifact.available ? "ready" : "blocked"}`)
      .join(", ")}`,
  ];
  if (preparation.configReadiness) {
    lines.push(
      `Connector readiness: ${preparation.configReadiness.plannedConnectorName} (${preparation.configReadiness.plannedConnectorId})`,
      `Connector next gate: ${preparation.configReadiness.nextGate.label}`,
      `Connector automation enabled: ${
        preparation.configReadiness.automationEnabled ? "yes" : "no"
      }`,
      `Connector gated evidence: ${preparation.configReadiness.gatedSteps.length} steps`,
    );
  }
  if (preparation.taskContext) {
    lines.push(
      "",
      "## Task-Aware Context",
      `Budget: ${preparation.taskContext.budgetTokens.toLocaleString()} tokens`,
      `Ranked files: ${preparation.taskContext.files.length}`,
      `Likely tests: ${preparation.taskContext.tests.length}`,
      "Top files:",
      ...preparation.taskContext.files
        .slice(0, 8)
        .map(
          (file) =>
            `- ${file.path} (score ${file.score}, ~${file.estimatedTokens.toLocaleString()} tokens): ${file.reasons.join("; ")}`,
        ),
      "Suggested commands:",
      ...preparation.taskContext.commands.map((command) => `- ${command}`),
    );
  }
  lines.push(
    "",
    "## Safety",
    `- Read-only: ${preparation.safety.readOnly ? "yes" : "no"}`,
    `- Secret-like paths excluded: ${
      preparation.safety.excludesSecretLikePaths ? "yes" : "no"
    }`,
    `- Repository modified: ${
      preparation.safety.modifiesRepository ? "yes" : "no"
    }`,
    `- Provider routing manual: ${
      preparation.safety.manualProviderRouting ? "yes" : "no"
    }`,
    "",
    "## Detail",
    preparation.copyDetail,
  );

  if (preparation.handoffMarkdown) {
    lines.push("", "## Handoff", preparation.handoffMarkdown);
  }

  return lines.join("\n");
}

function buildAgentSessionRecipes(repoRoot) {
  return agentHandoffProfiles.map((profile) => {
    const configReadiness = buildConfigReadiness(profile.id);
    return {
      id: profile.id,
      label: profile.label,
      toolKind: profile.toolKind,
      taskType: profile.defaultPackId,
      command: `npm run repo:intelligence -- ${JSON.stringify(repoRoot || ".")} --session --agent ${profile.id} --task ${profile.defaultPackId} --format markdown`,
      readOnly: true,
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
      importEdges: summary.graph?.importEdges ?? [],
      reverseDependencyHubs: summary.graph?.reverseDependencyHubs ?? [],
    },
    packs: summary.packs.map((contextPack) => ({
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
    taskPacks: (summary.taskPacks ?? []).map((taskPack) => ({
      id: taskPack.id,
      task: taskPack.task,
      budgetTokens: taskPack.budgetTokens,
      fileCount: taskPack.files.length,
      testCount: taskPack.tests.length,
      commandCount: taskPack.commands.length,
      topFiles: taskPack.files.slice(0, 8),
      tests: taskPack.tests.slice(0, 8),
      commands: taskPack.commands,
      omittedCount: taskPack.omitted.length,
    })),
    agentRecipes: repoAgentRecipeTemplates.map((recipe) => ({
      ...recipe,
      command: `npm run repo:intelligence -- ${JSON.stringify(summary.repoRoot)} --pack ${recipe.packIds[0]} --format markdown`,
    })),
    agentSessionRecipes: buildAgentSessionRecipes(summary.repoRoot),
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

function formatApiQueryList(summary) {
  const manifest = buildAgentManifest(summary);
  return [
    "# Repo Intelligence Read-Only API",
    "Safety: read-only yes; secret-like paths excluded yes; modifies repository no.",
    ...manifest.apiQueries.map(
      (query) =>
        `- ${query.command}: ${query.description} Read-only: ${
          query.readOnly ? "yes" : "no"
        }.`,
    ),
  ].join("\n");
}

function repoMemorySafety() {
  return {
    readOnly: true,
    excludesSecretLikePaths: true,
    modifiesRepository: false,
  };
}

function mcpTextResult(value) {
  return {
    content: [
      {
        type: "text",
        text:
          typeof value === "string" ? value : JSON.stringify(value, null, 2),
      },
    ],
  };
}

function handleRepoMemoryTool(summary, name, args = {}) {
  if (name === "switchboard.list_context_packs") {
    return mcpTextResult({
      repoRoot: summary.repoRoot,
      packs: summary.packs.map((pack) => ({
        id: pack.id,
        title: pack.title,
        purpose: pack.purpose,
        estimatedTokens: pack.estimatedTokens,
        fileCount: pack.files.length,
      })),
      safety: repoMemorySafety(),
    });
  }
  if (name === "repo_context_pack" || name === "switchboard.build_context_pack") {
    const pack =
      summary.packs.find(
        (candidate) =>
          candidate.id ===
          (args.packId ?? args.pack_id ?? args.id ?? "implementation"),
      ) ?? summary.packs[0];
    return mcpTextResult(formatSinglePackMarkdown(summary, pack));
  }
  if (name === "switchboard.get_repo_graph_summary") {
    return mcpTextResult({
      repoRoot: summary.repoRoot,
      graph: summary.graph ?? null,
      safety: repoMemorySafety(),
    });
  }
  if (name === "repo_symbol_lookup") {
    const query = String(args.query ?? "").toLowerCase();
    const symbols = (summary.graph?.symbols ?? [])
      .filter((symbol) => !query || symbol.name.toLowerCase().includes(query))
      .slice(0, 25);
    return mcpTextResult({
      repoRoot: summary.repoRoot,
      symbols,
      safety: repoMemorySafety(),
    });
  }
  if (name === "repo_dependents_of") {
    const target = String(args.target ?? "");
    const edges = [
      ...(summary.graph?.importEdges ?? []),
      ...(summary.graph?.symbolEdges ?? []),
    ]
      .filter(
        (edge) =>
          !target || edge.to.includes(target) || edge.from.includes(target),
      )
      .slice(0, 50);
    return mcpTextResult({
      repoRoot: summary.repoRoot,
      target,
      edges,
      safety: repoMemorySafety(),
    });
  }
  throw new Error(`Unknown repo-memory tool: ${name}`);
}

function runRepoMemoryMcpServer(options) {
  const summary = buildSummary(options.repoRoot);
  const tools = [
    {
      name: "switchboard.list_context_packs",
      description:
        "List available read-only Switchboard Repo Intelligence context packs; secret-like paths are excluded and repositories are not modified.",
      inputSchema: {
        type: "object",
        properties: {},
      },
      annotations: { readOnlyHint: true },
    },
    {
      name: "switchboard.build_context_pack",
      description:
        "Build a read-only Switchboard context pack as Markdown; secret-like paths are excluded and repositories are not modified.",
      inputSchema: {
        type: "object",
        properties: {
          packId: { type: "string" },
          pack_id: { type: "string" },
          task: { type: "string" },
          budget_tokens: { type: "number" },
        },
      },
      annotations: { readOnlyHint: true },
    },
    {
      name: "switchboard.get_repo_graph_summary",
      description:
        "Return the read-only Switchboard Repo Intelligence graph summary; secret-like paths are excluded and repositories are not modified.",
      inputSchema: {
        type: "object",
        properties: {},
      },
      annotations: { readOnlyHint: true },
    },
    {
      name: "repo_context_pack",
      description:
        "Return a read-only Repo Intelligence context pack as Markdown; secret-like paths are excluded and repositories are not modified.",
      inputSchema: {
        type: "object",
        properties: { packId: { type: "string" } },
      },
      annotations: { readOnlyHint: true },
    },
    {
      name: "repo_symbol_lookup",
      description:
        "Search the latest Repo Intelligence symbol graph read-only; secret-like paths are excluded and repositories are not modified.",
      inputSchema: {
        type: "object",
        properties: { query: { type: "string" } },
      },
      annotations: { readOnlyHint: true },
    },
    {
      name: "repo_dependents_of",
      description:
        "Return read-only import/symbol edges that point at a file or symbol; secret-like paths are excluded and repositories are not modified.",
      inputSchema: {
        type: "object",
        properties: { target: { type: "string" } },
      },
      annotations: { readOnlyHint: true },
    },
  ];
  process.stdin.setEncoding("utf8");
  let buffer = "";
  process.stdin.on("data", (chunk) => {
    buffer += chunk;
    for (;;) {
      const index = buffer.indexOf("\n");
      if (index === -1) break;
      const line = buffer.slice(0, index).trim();
      buffer = buffer.slice(index + 1);
      if (!line) continue;
      let request;
      try {
        request = JSON.parse(line);
        let result = {};
        if (request.method === "initialize")
          result = {
            protocolVersion: "2024-11-05",
            capabilities: { tools: {} },
            serverInfo: { name: "repo-memory", version: "1" },
          };
        else if (request.method === "tools/list") result = { tools };
        else if (request.method === "tools/call")
          result = handleRepoMemoryTool(
            summary,
            request.params?.name,
            request.params?.arguments ?? {},
          );
        else if (request.method === "ping") result = {};
        else if (request.id == null) continue;
        else throw new Error(`Unsupported method: ${request.method}`);
        if (request.id != null)
          process.stdout.write(
            `${JSON.stringify({ jsonrpc: "2.0", id: request.id, result })}\n`,
          );
      } catch (error) {
        process.stdout.write(
          `${JSON.stringify({ jsonrpc: "2.0", id: request?.id ?? null, error: { code: -32000, message: error.message } })}\n`,
        );
      }
    }
  });
}

const options = parseArgs(process.argv.slice(2));
options.mcpServe = options.mcpServe || process.argv.includes("--mcp-serve");

if (options.mcpServe) {
  runRepoMemoryMcpServer(options);
} else {
  if (
    !fs.existsSync(options.repoRoot) ||
    !fs.statSync(options.repoRoot).isDirectory()
  ) {
    console.error(
      `Repository path does not exist or not directory: ${options.repoRoot}`,
    );
    process.exit(1);
  }
  if (!["json", "markdown"].includes(options.format)) {
    console.error(
      `Unsupported format: ${options.format}. Use json or markdown.`,
    );
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
  if (options.listApi) {
    console.log(formatApiQueryList(summary));
    process.exit(0);
  }
  if (options.manifest) {
    console.log(JSON.stringify(buildAgentManifest(summary), null, 2));
    process.exit(0);
  }
  if (options.session) {
    try {
      const preparation = buildAgentSessionPreparation(summary, options);
      if (options.format === "markdown")
        console.log(formatAgentSessionMarkdown(preparation));
      else console.log(JSON.stringify(preparation, null, 2));
    } catch (error) {
      console.error(error.message);
      process.exit(1);
    }
    process.exit(0);
  }
  if (options.agent) {
    try {
      if (options.formatProvided && options.format === "json")
        console.log(
          JSON.stringify(
            buildAgentHandoffPayload(summary, options.agent, options.packId),
            null,
            2,
          ),
        );
      else
        console.log(
          formatAgentHandoffMarkdown(summary, options.agent, options.packId),
        );
    } catch (error) {
      console.error(error.message);
      process.exit(1);
    }
    process.exit(0);
  }
  if (options.packId) {
    const selectedPack = summary.packs.find(
      (contextPack) => contextPack.id === options.packId,
    );
    if (!selectedPack) {
      console.error(
        `Unknown pack: ${options.packId}. Available packs: ${summary.packs.map((contextPack) => contextPack.id).join(", ")}`,
      );
      process.exit(1);
    }
    if (options.format === "markdown")
      console.log(formatSinglePackMarkdown(summary, selectedPack));
    else
      console.log(
        JSON.stringify(
          { repoRoot: summary.repoRoot, pack: selectedPack },
          null,
          2,
        ),
      );
  } else if (options.format === "markdown") {
    for (const contextPack of summary.packs) {
      console.log(formatSinglePackMarkdown(summary, contextPack));
      console.log("\n---\n");
    }
  } else {
    console.log(JSON.stringify(summary, null, 2));
  }
}
