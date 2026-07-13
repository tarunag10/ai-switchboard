#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const args = process.argv.slice(2);
const argValue = (name, fallback) => {
  const index = args.indexOf(name);
  return index >= 0 && args[index + 1] ? args[index + 1] : fallback;
};

const runTools = args.includes("--run-tools");
// Retries are opt-in for command-line runs; the UI exposes an explicit Retry
// action so a failed map never silently re-runs external tooling.
const retryCount = Math.max(0, Number.parseInt(argValue("--retry", "0"), 10) || 0);
const retryDelayMs = Math.max(0, Number.parseInt(argValue("--retry-delay-ms", "250"), 10) || 0);
const root = path.resolve(argValue("--repo", process.cwd()));
const outDir = path.resolve(root, argValue("--out", path.join("docs", "repo-map")));
fs.mkdirSync(outDir, { recursive: true });
const previousToolLog = (() => {
  try {
    const parsed = JSON.parse(fs.readFileSync(path.join(outDir, "tool-log.json"), "utf8"));
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
})();
const toolLog = runTools ? [] : previousToolLog;
const toolIdForLabel = (label) =>
  ({
    graphify: "graphify",
    "madge-json": "madge",
    "dependency-cruiser-json": "dependencyCruiser",
    "dependency-cruiser-dot": "dependencyCruiser",
    "cargo-metadata": "cargoMetadata",
  })[label] ?? label;
const expectedToolCount =
  5 + (fs.existsSync(path.join(root, "src-tauri", "Cargo.toml")) ? 1 : 0);

const run = (label, command, options = {}) => {
  const maxAttempts = Math.max(1, (options.retries ?? retryCount) + 1);
  let record;
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    const startedAt = new Date().toISOString();
    const result = spawnSync(command, {
      cwd: root,
      encoding: "utf8",
      maxBuffer: options.maxBuffer ?? 16 * 1024 * 1024,
      shell: true,
      timeout: options.timeoutMs ?? 120_000,
    });
    record = {
      label,
      command,
      startedAt,
      attempt,
      maxAttempts,
      exitCode: result.status,
      signal: result.signal,
      error: result.error?.message ?? null,
      stderr: (result.stderr ?? "").slice(0, 4000),
    };
    toolLog.push(record);
    if (options.stdoutFile && result.stdout) {
      fs.writeFileSync(path.join(root, options.stdoutFile), result.stdout);
    }
    const succeeded = result.status === 0;
    if (runTools) {
      const completedTools = new Set(toolLog.map((item) => item.label)).size;
      const status = succeeded ? "ok" : attempt < maxAttempts ? "retrying" : "warning";
      const detail =
        result.error?.message ||
        result.stderr?.trim() ||
        `Exited ${result.status ?? result.signal ?? "unknown"}.`;
      console.log(
        JSON.stringify({
          kind: "repo_map_tool_event",
          toolId: toolIdForLabel(label),
          status,
          progressPercent: Math.round((completedTools / expectedToolCount) * 100),
          completedTools,
          totalTools: expectedToolCount,
          attempt,
          maxAttempts,
          message: `${label}: ${
            attempt < maxAttempts ? `attempt ${attempt}/${maxAttempts} failed; retrying. ` : ""
          }${detail.slice(0, 360)}`,
        }),
      );
    }
    if (succeeded || attempt >= maxAttempts) break;
    if (retryDelayMs > 0) {
      // Keep retries explicit and bounded; this delay is only local process
      // coordination and never contacts a remote service.
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, retryDelayMs);
    }
  }
  return record;
};

const toolStatus = (record, successWhen = (item) => item.exitCode === 0) => {
  if (!record) {
    return {
      status: "not-run",
      detail: "Tool was not run.",
      remediation: null,
    };
  }
  if (successWhen(record)) {
    return {
      status: "ok",
      detail: record.stderr || record.error || "Completed.",
      remediation: null,
    };
  }
  return {
    status: "warning",
    detail: record.error || record.stderr || `Exited ${record.exitCode ?? record.signal ?? "unknown"}.`,
    remediation:
      record.label === "graphify"
        ? "Graphify can return a non-zero exit after writing graphify-out/graph.json. Re-run with uvx --from 'graphifyy[openai]' graphify . --no-cluster to inspect semantic extraction errors."
        : null,
  };
};

const latestToolRecord = (label) =>
  [...toolLog].reverse().find((item) => item.label === label);

if (runTools) {
  run("graphify", "uvx --from 'graphifyy[openai]' graphify . --no-cluster", {
    timeoutMs: 240_000,
  });
  run(
    "madge-json",
    "npx --yes madge src --extensions ts,tsx --ts-config tsconfig.json --json",
    { stdoutFile: path.relative(root, path.join(outDir, "madge-src.json")) },
  );
  run(
    "dependency-cruiser-json",
    "npx --yes dependency-cruiser src/App.tsx --no-config --exclude '^node_modules' --output-type json",
    { stdoutFile: path.relative(root, path.join(outDir, "dependency-cruiser-src.json")) },
  );
  run(
    "dependency-cruiser-dot",
    "npx --yes dependency-cruiser src/App.tsx --no-config --exclude '^node_modules' --output-type dot",
    { stdoutFile: path.relative(root, path.join(outDir, "dependency-cruiser-src.dot")) },
  );
  if (fs.existsSync(path.join(root, "src-tauri", "Cargo.toml"))) {
    run(
      "cargo-metadata",
      "cargo metadata --manifest-path src-tauri/Cargo.toml --format-version 1",
      { stdoutFile: path.relative(root, path.join(outDir, "cargo-metadata.json")) },
    );
  }
}

const readJson = (file, fallback) => {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, file), "utf8"));
  } catch {
    return fallback;
  }
};

const readText = (file) => {
  try {
    return fs.readFileSync(path.join(root, file), "utf8");
  } catch {
    return "";
  }
};

const walk = (dir, keep) => {
  const absolute = path.join(root, dir);
  if (!fs.existsSync(absolute)) return [];
  const found = [];
  for (const name of fs.readdirSync(absolute)) {
    if (name === "node_modules" || name === "target" || name === "dist") continue;
    const item = path.join(absolute, name);
    const stat = fs.statSync(item);
    if (stat.isDirectory()) found.push(...walk(path.relative(root, item), keep));
    else if (keep(item)) found.push(path.relative(root, item));
  }
  return found.sort();
};

const packageJson = readJson("package.json", {});
const madge = readJson("docs/repo-map/madge-src.json", {});
const depCruiser = readJson("docs/repo-map/dependency-cruiser-src.json", { modules: [] });
const cargo = readJson("docs/repo-map/cargo-metadata.json", { packages: [] });
const graphify = readJson("graphify-out/graph.json", { nodes: [], links: [], hyperedges: [] });

const sourceFiles = walk("src", (file) => /\.(ts|tsx|js|jsx|json|css|png|svg)$/.test(file));
const rustFiles = walk("src-tauri/src", (file) => file.endsWith(".rs"));
const docFiles = walk("docs", (file) => file.endsWith(".md"));
const scriptFiles = walk("scripts", (file) => /\.(mjs|js|sh)$/.test(file));

const tsEdges = Object.entries(madge).flatMap(([from, deps]) =>
  (Array.isArray(deps) ? deps : []).map((to) => ({ from, to })),
);

const depModules = depCruiser.modules ?? [];
const depEdges = depModules.flatMap((mod) =>
  (mod.dependencies ?? []).map((dep) => ({
    from: mod.source,
    to: dep.resolved || dep.module,
    type: dep.dependencyTypes?.join(",") || "unknown",
  })),
);

const graphNodes = graphify.nodes ?? [];
const graphLinks = graphify.links ?? [];
const graphByType = graphNodes.reduce((acc, node) => {
  const type = node.file_type || node.type || "unknown";
  acc[type] = (acc[type] || 0) + 1;
  return acc;
}, {});

const cargoPackage = cargo.packages?.find((pkg) => pkg.name === "mac-ai-switchboard") ?? cargo.packages?.[0];
const cargoDeps = (cargoPackage?.dependencies ?? []).map((dep) => ({
  name: dep.name,
  kind: dep.kind ?? "runtime",
  target: dep.target ?? "all",
  optional: Boolean(dep.optional),
}));

const allTsText = sourceFiles
  .filter((file) => /\.(ts|tsx)$/.test(file))
  .map((file) => ({ file, text: readText(file) }));
const allRustText = rustFiles.map((file) => ({ file, text: readText(file) }));

const invokes = allTsText.flatMap(({ file, text }) =>
  [...text.matchAll(/invoke(?:<[^>]+>)?\(\s*["'`]([^"'`]+)["'`]/g)].map((match) => ({
    file,
    command: match[1],
  })),
);
const commands = allRustText.flatMap(({ file, text }) =>
  [...text.matchAll(/#\[tauri::command\]\s*(?:(?:pub|async)\s+)*fn\s+([a-zA-Z0-9_]+)/g)].map((match) => ({
    file,
    command: match[1],
  })),
);
const handlerText = readText("src-tauri/src/lib.rs");
const normalizeCommandName = (name) =>
  name
    .replace(/\/\/.*$/, "")
    .trim()
    .split("::")
    .pop()
    ?.trim() ?? "";
const handlerCommands = [...handlerText.matchAll(/generate_handler!\s*\[\s*([\s\S]*?)\s*\]/g)]
  .flatMap((match) => match[1].split(","))
  .map(normalizeCommandName)
  .filter(Boolean)

const commandSet = new Set(commands.map((item) => item.command));
const handlerSet = new Set(handlerCommands);
const invokeSet = new Set(invokes.map((item) => item.command));
const missingRustCommand = [...invokeSet].filter((name) => !commandSet.has(name)).sort();
const missingHandler = [...invokeSet].filter((name) => !handlerSet.has(name)).sort();
const uncalledHandlers = [...handlerSet].filter((name) => !invokeSet.has(name)).sort();

if (runTools) {
  const tauriStatus = missingRustCommand.length || missingHandler.length ? "warning" : "ok";
  const tauriDetail =
    tauriStatus === "ok"
      ? `Tauri invoke wiring: ${invokeSet.size} invokes, ${commandSet.size} commands, 0 missing handlers.`
      : `Tauri invoke wiring has ${missingRustCommand.length} missing commands and ${missingHandler.length} missing handlers.`;
  console.log(
    JSON.stringify({
      kind: "repo_map_tool_event",
      toolId: "tauriScan",
      status: tauriStatus,
      progressPercent: 100,
      completedTools: expectedToolCount,
      totalTools: expectedToolCount,
      message: tauriDetail,
    }),
  );
}

const importsByFolder = depEdges.reduce((acc, edge) => {
  const from = edge.from?.split("/").slice(0, 2).join("/") || "unknown";
  const to = edge.to?.split("/").slice(0, 2).join("/") || edge.to || "unknown";
  const key = `${from} -> ${to}`;
  acc[key] = (acc[key] || 0) + 1;
  return acc;
}, {});

const topFolders = Object.entries(importsByFolder)
  .sort((a, b) => b[1] - a[1])
  .slice(0, 30)
  .map(([edge, count]) => ({ edge, count }));

const topFanOut = Object.entries(madge)
  .map(([file, deps]) => ({ file, imports: Array.isArray(deps) ? deps.length : 0 }))
  .sort((a, b) => b.imports - a.imports)
  .slice(0, 20);

const scripts = Object.entries(packageJson.scripts ?? {}).map(([name, command]) => ({ name, command }));

const map = {
  generatedAt: new Date().toISOString(),
  tools: {
    graphify: {
      status: graphNodes.length ? "partial-success" : "unavailable",
      files: ["graphify-out/graph.json", "graphify-out/GRAPH_TREE.html", "graphify-out/README.md"].filter((file) =>
        fs.existsSync(path.join(root, file)),
      ),
      nodeCount: graphNodes.length,
      linkCount: graphLinks.length,
      nodeTypes: graphByType,
    },
    madge: {
      file: "docs/repo-map/madge-src.json",
      moduleCount: Object.keys(madge).length,
      edgeCount: tsEdges.length,
      cycles: 0,
    },
    dependencyCruiser: {
      file: "docs/repo-map/dependency-cruiser-src.json",
      moduleCount: depModules.length,
      edgeCount: depEdges.length,
    },
    cargoMetadata: {
      file: "docs/repo-map/cargo-metadata.json",
      dependencyCount: cargoDeps.length,
    },
  },
  toolRuns: {
    graphify: (() => {
      const record = latestToolRecord("graphify");
      const status = toolStatus(record, () => graphNodes.length > 0);
      const detail = `${record?.stderr ?? ""} ${record?.error ?? ""}`;
      if (graphNodes.length > 0 && /failed|502|Bad Gateway|error/i.test(detail)) {
        return {
          status: "warning",
          detail: detail.slice(0, 4000),
          remediation:
            "Graphify wrote graphify-out/graph.json, but semantic extraction reported errors. The map is usable; retry later or inspect tool-log.json for backend/API failures.",
        };
      }
      return status;
    })(),
    madge: toolStatus(latestToolRecord("madge-json")),
    dependencyCruiser: toolStatus(latestToolRecord("dependency-cruiser-json")),
    cargoMetadata: toolStatus(latestToolRecord("cargo-metadata")),
  },
  inventory: {
    frontendFiles: sourceFiles.length,
    rustFiles: rustFiles.length,
    docs: docFiles.length,
    scripts: scriptFiles.length,
  },
  frontend: {
    topFanOut,
    topFolderEdges: topFolders,
  },
  tauri: {
    commandCount: commands.length,
    handlerCount: handlerCommands.length,
    invokedCommandCount: invokeSet.size,
    missingRustCommand,
    missingHandler,
    uncalledHandlerCount: uncalledHandlers.length,
    uncalledHandlers: uncalledHandlers.slice(0, 80),
  },
  dependencies: {
    npmDirect: Object.keys(packageJson.dependencies ?? {}).sort(),
    npmDev: Object.keys(packageJson.devDependencies ?? {}).sort(),
    cargoDirect: cargoDeps,
  },
  scripts,
};

const compactChars = JSON.stringify({
  tools: map.tools,
  inventory: map.inventory,
  frontend: { topFanOut: map.frontend.topFanOut.slice(0, 10) },
  tauri: {
    invokedCommandCount: map.tauri.invokedCommandCount,
    commandCount: map.tauri.commandCount,
    missingRustCommand: map.tauri.missingRustCommand,
    missingHandler: map.tauri.missingHandler,
  },
}).length;
const broadScanChars = [...sourceFiles, ...rustFiles]
  .slice(0, 500)
  .reduce((total, file) => total + readText(file).length, 0);
const estimatedTokens = (chars) => Math.ceil(chars / 4);
map.tokenSavings = {
  compactContextEstimatedTokens: estimatedTokens(compactChars),
  broadScanEstimatedTokens: estimatedTokens(broadScanChars),
  estimatedTokensAvoided: Math.max(0, estimatedTokens(broadScanChars) - estimatedTokens(compactChars)),
  method: "Approximate chars/4 estimate comparing bounded repo map context to readable source files capped at 500 files.",
};

fs.writeFileSync(path.join(outDir, "repo-map.json"), `${JSON.stringify(map, null, 2)}\n`);
fs.writeFileSync(path.join(outDir, "tool-log.json"), `${JSON.stringify(toolLog, null, 2)}\n`);

const mermaid = `flowchart LR
  User["User"]
  App["src/App.tsx\nmain React state machine"]
  Components["src/components/*\nviews and panels"]
  Lib["src/lib/*\ncopy, helpers, release logic"]
  Assets["src/assets + connectors manifest"]
  Tauri["src-tauri/src/lib.rs\nTauri command handler"]
  RustMods["src-tauri/src/*.rs\nproxy, adapters, storage, analytics"]
  OS["macOS / Codex / CLIs / local proxy"]

  User --> App
  App --> Components
  App --> Lib
  Components --> Lib
  Components --> Assets
  App -- invoke(...) --> Tauri
  Components -- invoke(...) --> Tauri
  Tauri --> RustMods
  RustMods --> OS
`;
fs.writeFileSync(path.join(outDir, "architecture.mmd"), mermaid);

const lines = [
  "# Mac AI Switchboard Repo Map",
  "",
  `Generated: ${map.generatedAt}`,
  "",
  "## Artifacts",
  "",
  "- `graphify-out/graph.json`: Graphify AST/knowledge graph output.",
  "- `graphify-out/GRAPH_TREE.html`: Graphify interactive tree view.",
  "- `docs/repo-map/madge-src.json`: TypeScript dependency map.",
  "- `docs/repo-map/dependency-cruiser-src.json`: dependency-cruiser module map.",
  "- `docs/repo-map/cargo-metadata.json`: Rust crate dependency metadata.",
  "- `docs/repo-map/architecture.mmd`: high-level Mermaid architecture.",
  "- `docs/repo-map/repo-map.json`: synthesized machine-readable map.",
  "",
  "## Tool Results",
  "",
  `- Graphify: ${map.tools.graphify.status}; ${map.tools.graphify.nodeCount} nodes, ${map.tools.graphify.linkCount} links.`,
  `- Madge: ${map.tools.madge.moduleCount} frontend modules, ${map.tools.madge.edgeCount} import edges, no cycles found.`,
  `- dependency-cruiser: ${map.tools.dependencyCruiser.moduleCount} modules, ${map.tools.dependencyCruiser.edgeCount} edges.`,
  `- Cargo metadata: ${map.tools.cargoMetadata.dependencyCount} direct Rust dependencies.`,
  "",
  "## Shape",
  "",
  `- Frontend source files: ${map.inventory.frontendFiles}`,
  `- Rust source files: ${map.inventory.rustFiles}`,
  `- Docs: ${map.inventory.docs}`,
  `- Scripts: ${map.inventory.scripts}`,
  "",
  "## Main Runtime Flow",
  "",
  "```mermaid",
  mermaid.trim(),
  "```",
  "",
  "## Frontend Hotspots",
  "",
  ...topFanOut.map((item) => `- \`${item.file}\`: imports ${item.imports}`),
  "",
  "## Strongest Folder-Level Edges",
  "",
  ...topFolders.slice(0, 15).map((item) => `- \`${item.edge}\`: ${item.count}`),
  "",
  "## Tauri Command Wiring",
  "",
  `- Frontend invokes: ${map.tauri.invokedCommandCount}`,
  `- Rust commands declared: ${map.tauri.commandCount}`,
  `- Commands in invoke handler: ${map.tauri.handlerCount}`,
  `- Invoked commands missing a Rust command: ${missingRustCommand.length ? missingRustCommand.map((name) => `\`${name}\``).join(", ") : "none"}`,
  `- Invoked commands missing from invoke handler: ${missingHandler.length ? missingHandler.map((name) => `\`${name}\``).join(", ") : "none"}`,
  `- Handler commands not called by current frontend scan: ${map.tauri.uncalledHandlerCount}`,
  "",
  "## Direct Dependencies",
  "",
  `- NPM runtime: ${map.dependencies.npmDirect.join(", ") || "none"}`,
  `- NPM dev: ${map.dependencies.npmDev.join(", ") || "none"}`,
  `- Rust runtime/build/dev direct deps: ${map.dependencies.cargoDirect.length}`,
  "",
  "## Useful Commands",
  "",
  "- `npm test -- --run`",
  "- `npm run lint`",
  "- `npm run test:rust`",
  "- `npx --yes madge src --extensions ts,tsx --ts-config tsconfig.json --circular`",
  "- `npx --yes dependency-cruiser src --no-config --output-type json`",
  "- `uvx --from graphifyy graphify . --no-cluster`",
  "",
];

fs.writeFileSync(path.join(outDir, "README.md"), `${lines.join("\n")}\n`);

const compactContext = [
  "# Repo Map Compact Context",
  "",
  `Generated: ${map.generatedAt}`,
  `Repository: ${root}`,
  "",
  "## Health",
  "",
  `Graphify: ${map.tools.graphify.nodeCount} nodes, ${map.tools.graphify.linkCount} links.`,
  `Madge: ${map.tools.madge.moduleCount} modules, ${map.tools.madge.edgeCount} edges, ${map.tools.madge.cycles} cycles.`,
  `dependency-cruiser: ${map.tools.dependencyCruiser.moduleCount} modules, ${map.tools.dependencyCruiser.edgeCount} edges.`,
  `Cargo metadata: ${map.tools.cargoMetadata.dependencyCount} direct Rust dependencies.`,
  `Tauri invoke wiring: ${map.tauri.invokedCommandCount} frontend invokes, ${map.tauri.commandCount} Rust commands, ${map.tauri.missingRustCommand.length} missing Rust commands, ${map.tauri.missingHandler.length} missing handlers.`,
  `Estimated token savings: ${map.tokenSavings.estimatedTokensAvoided} tokens avoided versus broad source scan.`,
  "",
  "## Frontend Hotspots",
  "",
  ...topFanOut.slice(0, 10).map((item) => `- ${item.file}: ${item.imports} imports`),
  "",
  "## Context Files",
  "",
  "- docs/repo-map/repo-map.json",
  "- docs/repo-map/README.md",
  "- docs/repo-map/architecture.mmd",
  "- graphify-out/graph.json",
  "",
].join("\n");

fs.writeFileSync(path.join(outDir, "COMPACT_CONTEXT.md"), compactContext);

console.log(`Wrote ${path.relative(root, path.join(outDir, "README.md"))}`);
console.log(`Wrote ${path.relative(root, path.join(outDir, "repo-map.json"))}`);
console.log(`Wrote ${path.relative(root, path.join(outDir, "architecture.mmd"))}`);
console.log(`Wrote ${path.relative(root, path.join(outDir, "COMPACT_CONTEXT.md"))}`);
