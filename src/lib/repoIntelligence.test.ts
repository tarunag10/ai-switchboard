import { describe, expect, it } from "vitest";

import {
  buildAgentSessionPreparation,
  buildAgentSessionDisplayState,
  buildRepoIntelligenceSummary,
  buildRepoTaskContextPack,
  buildRepoAgentManifest,
  buildRepoAgentHandoffPayload,
  classifyRepoFile,
  estimateRepoIntelligenceSavings,
  estimateRepoTokens,
  formatAgentSessionPreparationJson,
  formatAgentSessionSelectedPackMarkdown,
  formatAgentSessionSummaryMarkdown,
  formatRepoAgentManifestJson,
  formatRepoAgentHandoffMarkdown,
  formatRepoContextPackMarkdown,
  formatSingleRepoContextPackMarkdown,
  getRepoIndexFreshness,
  isSecretLikeRepoPath,
  normalizeRepoIndexRequest,
  recommendAgentSessionMode,
  repoAgentPackLabel,
} from "./repoIntelligence";

describe("repoIntelligence", () => {
  it("validates repo index requests before invoking the app indexer", () => {
    expect(normalizeRepoIndexRequest("")).toEqual({
      repoPath: "",
      error: "Enter a local repository folder path first.",
    });
    expect(normalizeRepoIndexRequest("   ")).toEqual({
      repoPath: "",
      error: "Enter a local repository folder path first.",
    });
    expect(
      normalizeRepoIndexRequest("  /Users/me/Developer/app  "),
    ).toEqual({
      repoPath: "/Users/me/Developer/app",
      error: null,
    });
  });

  it("estimates tokens from bytes conservatively", () => {
    expect(estimateRepoTokens(0)).toBe(1);
    expect(estimateRepoTokens(400)).toBe(100);
    expect(estimateRepoTokens(401)).toBe(101);
  });

  it("classifies common repo files into local context roles", () => {
    expect(classifyRepoFile("src/App.tsx", 100).role).toBe("source");
    expect(classifyRepoFile("src/App.test.tsx", 100).role).toBe("test");
    expect(classifyRepoFile("docs/install.md", 100).role).toBe("docs");
    expect(classifyRepoFile("package-lock.json", 100).role).toBe("lockfile");
    expect(classifyRepoFile("dist/assets/app.js", 100).role).toBe("generated");
    expect(classifyRepoFile("src/assets/logo.svg", 100).role).toBe("asset");
  });

  it("excludes secret-like paths from default context packs", () => {
    expect(isSecretLikeRepoPath(".env.local")).toBe(true);
    expect(isSecretLikeRepoPath(".envrc")).toBe(true);
    expect(isSecretLikeRepoPath(".git-credentials")).toBe(true);
    expect(isSecretLikeRepoPath(".netrc")).toBe(true);
    expect(isSecretLikeRepoPath(".cargo/credentials.toml")).toBe(true);
    expect(isSecretLikeRepoPath(".config/gh/hosts.yml")).toBe(true);
    expect(isSecretLikeRepoPath(".ssh/config")).toBe(true);
    expect(isSecretLikeRepoPath(".aws/credentials")).toBe(true);
    expect(isSecretLikeRepoPath(".private_keys/AuthKey_ABC123XYZ.p8")).toBe(
      true,
    );
    expect(isSecretLikeRepoPath("config/service-account.pem")).toBe(true);
    expect(isSecretLikeRepoPath(".claude/settings.local.json")).toBe(true);
    expect(isSecretLikeRepoPath(".playwright-mcp/console.log")).toBe(true);
    expect(isSecretLikeRepoPath("headroom_memory.db")).toBe(true);

    const envFile = classifyRepoFile(".env.local", 100);
    expect(envFile.role).toBe("config");
    expect(envFile.includeByDefault).toBe(false);
    expect(envFile.reasons).toContain("secret-like path excluded");

    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: ".env.local", bytes: 400 },
      { path: ".envrc", bytes: 80 },
      { path: ".git-credentials", bytes: 90 },
      { path: ".netrc", bytes: 90 },
      { path: ".cargo/credentials.toml", bytes: 90 },
      { path: ".config/gh/hosts.yml", bytes: 90 },
      { path: ".ssh/config", bytes: 90 },
      { path: ".aws/credentials", bytes: 90 },
      { path: ".private_keys/AuthKey_ABC123XYZ.p8", bytes: 800 },
      { path: ".claude/settings.local.json", bytes: 120 },
      { path: ".playwright-mcp/console.log", bytes: 900 },
      { path: "headroom_memory.db", bytes: 9000 },
    ]);
    const packedPaths = summary.packs.flatMap((pack) =>
      pack.files.map((file) => file.path),
    );

    expect(packedPaths).toContain("src/App.tsx");
    expect(packedPaths).not.toContain(".env.local");
    expect(packedPaths).not.toContain(".envrc");
    expect(packedPaths).not.toContain(".git-credentials");
    expect(packedPaths).not.toContain(".netrc");
    expect(packedPaths).not.toContain(".cargo/credentials.toml");
    expect(packedPaths).not.toContain(".config/gh/hosts.yml");
    expect(packedPaths).not.toContain(".ssh/config");
    expect(packedPaths).not.toContain(".aws/credentials");
    expect(packedPaths).not.toContain(".private_keys/AuthKey_ABC123XYZ.p8");
    expect(packedPaths).not.toContain(".claude/settings.local.json");
    expect(packedPaths).not.toContain(".playwright-mcp/console.log");
    expect(packedPaths).not.toContain("headroom_memory.db");
    expect(
      summary.indexMetadata?.fileFingerprints.map((entry) => entry.path),
    ).toEqual(["src/App.tsx"]);
  });

  it("builds bounded context packs with savings estimates", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "src/lib/types.ts", bytes: 1000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "dist/bundle.js", bytes: 20000 },
      { path: "package.json", bytes: 800 },
    ]);

    expect(summary.totalFiles).toBe(6);
    expect(summary.indexedFiles).toBe(5);
    expect(summary.indexerVersion).toBe("path-graph-v5");
    expect(summary.roleCounts.generated).toBe(1);
    expect(summary.indexMetadata).toMatchObject({
      schemaVersion: 1,
      indexerVersion: "path-graph-v5",
      parserVersion: "metadata-fingerprint-v1",
      cacheState: "new",
      fileCount: 6,
      indexedFileCount: 5,
      skippedFileCount: 1,
    });
    expect(summary.indexMetadata?.cacheKey).toEqual(expect.any(String));
    expect(summary.indexMetadata?.fileFingerprints).toHaveLength(5);
    expect(summary.indexMetadata?.fileFingerprints[0]).toEqual(
      expect.objectContaining({
        path: expect.any(String),
        fingerprint: expect.any(String),
      }),
    );
    expect(summary.indexMetadata?.skippedFiles).toEqual([
      expect.objectContaining({
        path: "dist/bundle.js",
        role: "generated",
        reasons: expect.arrayContaining(["generated or dependency output"]),
      }),
    ]);
    expect(summary.indexMetadata?.graphInputs).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          path: "src/App.tsx",
          role: "source",
          fingerprint: expect.any(String),
        }),
        expect.objectContaining({
          path: "package.json",
          role: "config",
          fingerprint: expect.any(String),
        }),
      ]),
    );
    expect(summary.packs.map((pack) => pack.id)).toEqual([
      "implementation",
      "verification",
      "handoff",
      "risk_review",
      "release_handoff",
    ]);
    expect(summary.packs[0].id).toBe("implementation");
    expect(summary.packs[0].files.map((file) => file.path)).toContain(
      "src/App.tsx",
    );
    expect(
      summary.packs
        .find((pack) => pack.id === "risk_review")
        ?.files.map((file) => file.path),
    ).toEqual(
      expect.arrayContaining([
        "src/App.tsx",
        "src/App.test.tsx",
        "package.json",
      ]),
    );
    expect(
      summary.packs
        .find((pack) => pack.id === "release_handoff")
        ?.files.map((file) => file.path),
    ).toEqual(
      expect.arrayContaining([
        "src/App.test.tsx",
        "docs/install.md",
        "package.json",
      ]),
    );
    expect(summary.packs[0].savingsVsFullScanPct).toBeGreaterThan(50);
    expect(summary.taskPacks?.map((pack) => pack.id)).toEqual([
      "task_implementation",
      "task_verification",
    ]);
  });

  it("builds task-aware context packs with reasons and omitted files", () => {
    const summary = buildRepoIntelligenceSummary([
      {
        path: "src/components/ReleaseReadinessPanel.tsx",
        bytes: 1200,
      },
      {
        path: "src/components/ReleaseReadinessPanel.test.tsx",
        bytes: 900,
      },
      { path: "src/lib/releaseReadiness.ts", bytes: 2000 },
      { path: "src/lib/savingsCalculator.ts", bytes: 2600 },
      { path: "docs/macos-release.md", bytes: 800 },
      { path: ".env.local", bytes: 100 },
      { path: "dist/bundle.js", bytes: 20000 },
    ]);

    const files = summary.packs.flatMap((contextPack) => contextPack.files);
    const pack = buildRepoTaskContextPack(
      files,
      summary.graph,
      "release readiness",
      "release readiness panel smoke evidence",
      900,
    );

    expect(pack.id).toBe("task_release_readiness");
    expect(pack.files[0].path).toContain("ReleaseReadinessPanel");
    expect(pack.files[0].reasons.join(" ")).toContain("release");
    expect(pack.tests.map((file) => file.path)).toContain(
      "src/components/ReleaseReadinessPanel.test.tsx",
    );
    expect(pack.omitted.every((file) => !file.path.includes(".env"))).toBe(
      true,
    );
    expect(pack.commands).toEqual(
      expect.arrayContaining(["npm run smoke:preflight"]),
    );
  });

  it("derives index freshness copy from persistent metadata", () => {
    expect(getRepoIndexFreshness({})).toMatchObject({
      status: "none",
      apiAvailable: true,
      graphAvailable: false,
      indexHealth: "metadata_missing",
      parserHealth: "unavailable",
      indexerVersion: null,
      parserVersion: null,
      indexedFileCount: null,
      skippedFileCount: null,
    });
    expect(
      getRepoIndexFreshness({
        indexedAt: "2026-06-27T10:00:00Z",
      }).status,
    ).toBe("unknown");

    const baseMetadata = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "package.json", bytes: 800 },
    ]).indexMetadata;

    expect(
      getRepoIndexFreshness({
        indexedAt: "2026-06-27T10:00:00Z",
        indexerVersion: "path-graph-v5",
        indexMetadata: baseMetadata,
        graph: buildRepoIntelligenceSummary([
          { path: "src/App.tsx", bytes: 4000 },
          { path: "package.json", bytes: 800 },
        ]).graph,
      }),
    ).toMatchObject({
      status: "fresh",
      label: "Fresh local index",
      apiAvailable: true,
      graphAvailable: true,
      indexHealth: "new",
      parserHealth: "current",
      indexerVersion: "path-graph-v5",
      parserVersion: "metadata-fingerprint-v1",
      indexedFileCount: 2,
      skippedFileCount: 0,
    });
    expect(
      getRepoIndexFreshness({
        indexedAt: "2026-06-27T10:00:00Z",
        indexMetadata: {
          ...baseMetadata!,
          cacheState: "unchanged",
          previousIndexedAt: "2026-06-27T09:00:00Z",
        },
      }),
    ).toMatchObject({
      status: "unchanged_cache",
      label: "Unchanged local index",
    });
    expect(
      getRepoIndexFreshness({
        indexedAt: "2026-06-27T10:00:00Z",
        indexMetadata: { ...baseMetadata!, cacheState: "changed" },
      }),
    ).toMatchObject({ status: "changed_cache", label: "Changed local index" });
  });

  it("builds a bounded repo graph summary for agent context", () => {
    const summary = buildRepoIntelligenceSummary([
      {
        path: "src/App.tsx",
        bytes: 4000,
        content:
          'import React from "react";\nimport { helper } from "./lib/helper";\nexport default function App() { helper(); return null; }\nexport const loadPanel = async () => true;',
      },
      { path: "src/main.tsx", bytes: 1400, content: 'import "./App";' },
      { path: "src/App.test.tsx", bytes: 2000 },
      {
        path: "src/lib/helper.ts",
        bytes: 900,
        content:
          "export function helper() { return true; }\nexport const makeHelper = () => helper();",
      },
      {
        path: "src-tauri/src/lib.rs",
        bytes: 5000,
        content: "mod state;\nuse crate::state::AppState;\npub(crate) fn run_app() {}\n",
      },
      {
        path: "src-tauri/src/state.rs",
        bytes: 3200,
        content: "pub struct AppState;\npub fn load_state() {}\n",
      },
      {
        path: "src-tauri/src/consumer.rs",
        bytes: 1400,
        content: "use crate::state::AppState;\npub fn consume_state() {}\n",
      },
      {
        path: "scripts/tools.py",
        bytes: 700,
        content: "async def collect_context():\n    return []\nclass ToolRunner:\n    pass",
      },
      {
        path: "Sources/Switchboard/AppView.swift",
        bytes: 900,
        content:
          "import SwiftUI\npublic struct AppView: View {\n  var body: some View { Text(\"Hi\") }\n}\nfinal class AppViewModel {}\nfunc makeAppView() -> AppView { AppView() }\n",
      },
      { path: "scripts/release.mjs", bytes: 1200 },
      {
        path: "scripts/release.sh",
        bytes: 600,
        content: "set -e\n./build.sh\nbash scripts/smoke.sh\n",
      },
      { path: "scripts/build.sh", bytes: 200, content: "echo build\n" },
      { path: "scripts/smoke.sh", bytes: 200, content: "echo smoke\n" },
      {
        path: "package.json",
        bytes: 800,
        content: '{"dependencies":{"react":"18.3.1"}}',
      },
      { path: "package-lock.json", bytes: 1600 },
      { path: ".env.local", bytes: 200 },
    ]);

    expect(summary.graph?.topDirectories.map((node) => node.label)).toContain(
      "src",
    );
    expect(summary.graph?.topLanguages.map((node) => node.label)).toContain(
      "React",
    );
    expect(summary.graph?.entrypoints.map((file) => file.path)).toContain(
      "src/main.tsx",
    );
    expect(summary.graph?.likelyTests.map((file) => file.path)).toContain(
      "src/App.test.tsx",
    );
    expect(summary.graph?.configHubs.map((file) => file.path)).toContain(
      "package.json",
    );
    expect(summary.graph?.configHubs.map((file) => file.path)).not.toContain(
      ".env.local",
    );
    expect(summary.graph?.dependencyHubs?.map((file) => file.path)).toEqual([
      "package.json",
    ]);
    expect(summary.graph?.importEdges).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          from: "src/App.test.tsx",
          to: "src/App.tsx",
          kind: "test_to_source",
        }),
        expect.objectContaining({
          from: "src/main.tsx",
          to: "package.json",
          kind: "entrypoint_to_config",
        }),
        expect.objectContaining({
          from: "src/App.tsx",
          to: "src/lib/helper.ts",
          kind: "import_reference",
        }),
        expect.objectContaining({
          from: "src/App.tsx",
          to: "package.json",
          kind: "package_dependency",
          reason: "source imports package react",
        }),
        expect.objectContaining({
          from: "scripts/release.sh",
          to: "scripts/build.sh",
          kind: "import_reference",
          reason: "script invokes ./build.sh",
        }),
        expect.objectContaining({
          from: "scripts/release.sh",
          to: "scripts/smoke.sh",
          kind: "import_reference",
          reason: "script invokes scripts/smoke.sh",
        }),
      ]),
    );
    expect(
      summary.graph?.reverseDependencyHubs?.map((node) => node.label),
    ).toContain("package.json");
    expect(summary.graph?.symbols?.map((symbol) => symbol.name)).toEqual(
      expect.arrayContaining([
        "App",
        "loadPanel",
        "makeHelper",
        "run_app",
        "AppState",
        "collect_context",
        "ToolRunner",
        "AppView",
        "AppViewModel",
        "makeAppView",
      ]),
    );
    expect(summary.graph?.topLanguages.map((node) => node.label)).toContain(
      "Swift",
    );
    expect(summary.graph?.importEdges).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          from: "src-tauri/src/lib.rs",
          to: "src-tauri/src/state.rs",
          kind: "import_reference",
          reason: "source imports ./state",
        }),
        expect.objectContaining({
          from: "src-tauri/src/consumer.rs",
          to: "src-tauri/src/state.rs",
          kind: "import_reference",
          reason: "source imports crate:state::AppState",
        }),
      ]),
    );
    expect(summary.graph?.symbolEdges).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          kind: "symbol_reference",
          to: "src/App.tsx#App",
        }),
        expect.objectContaining({
          from: "src/App.tsx",
          to: "src/lib/helper.ts#helper",
          kind: "call_reference",
        }),
      ]),
    );
  });

  it("formats bounded context packs for agent handoff", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
      { path: "package-lock.json", bytes: 1600 },
    ]);
    summary.repoRoot = "/Users/me/app";
    summary.indexedAt = "2026-06-25T10:00:00Z";

    const markdown = formatRepoContextPackMarkdown(summary);

    expect(markdown).toContain(
      "# Repo Intelligence Context Pack: /Users/me/app",
    );
    expect(markdown).toContain("Indexed at: 2026-06-25T10:00:00Z");
    expect(markdown).toContain("## Repo Graph Summary");
    expect(markdown).toContain("Top directories");
    expect(markdown).toContain("Likely tests");
    expect(markdown).toContain("Dependency hubs");
    expect(markdown).toContain("Import and dependency edges");
    expect(markdown).toContain("Reverse dependency hubs");
    expect(markdown).toContain("Symbols");
    expect(markdown).toContain("- package.json");
    expect(markdown).toContain("## Implementation Pack");
    expect(markdown).toContain("## Risk Review Pack");
    expect(markdown).toContain("## Release Handoff Pack");
    expect(markdown).toContain("src/App.tsx");
    expect(markdown).toContain("Estimated savings vs full scan");
  });

  it("formats a single task-specific context pack", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
    ]);
    summary.repoRoot = "/Users/me/app";
    summary.indexedAt = "2026-06-25T10:00:00Z";

    const markdown = formatSingleRepoContextPackMarkdown(
      summary,
      summary.packs[0],
    );

    expect(markdown).toContain("# Implementation Pack: /Users/me/app");
    expect(markdown).toContain("Indexed at: 2026-06-25T10:00:00Z");
    expect(markdown).toContain("Estimated tokens avoided");
    expect(markdown).toContain("## Repo Graph Summary");
    expect(markdown).toContain("## Files");
    expect(markdown).toContain("src/App.tsx");
    expect(markdown).not.toContain("## Verification Pack");
  });

  it("formats an agent-readable manifest for external coding tools", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
      { path: "package-lock.json", bytes: 1600 },
      { path: ".env.local", bytes: 300 },
    ]);
    summary.repoRoot = "/Users/me/app";
    const manifest = buildRepoAgentManifest(summary, "2026-06-25T10:00:00Z");
    expect(manifest.kind).toBe("mac_ai_switchboard.repo_intelligence_manifest");
    expect(manifest.schemaVersion).toBe(1);
    expect(manifest.generatedAt).toBe("2026-06-25T10:00:00Z");
    expect(manifest.totals.indexerVersion).toBe("path-graph-v5");
    expect(manifest.totals.indexMetadata?.cacheState).toBe("new");
    expect(manifest.totals.indexMetadata?.fileFingerprints.length).toBe(4);
    expect(manifest.totals.indexMetadata?.skippedFiles).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          path: "<secret-like path>",
          reasons: expect.arrayContaining(["secret-like path excluded"]),
        }),
        expect.objectContaining({ path: "package-lock.json" }),
      ]),
    );
    expect(manifest.totals.indexMetadata?.graphInputs).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ path: "src/App.tsx", role: "source" }),
        expect.objectContaining({ path: "package.json", role: "config" }),
      ]),
    );
    expect(manifest.safety).toEqual({
      readOnly: true,
      excludesSecretLikePaths: true,
      modifiesRepository: false,
    });
    expect(manifest.packs.map((pack) => pack.id)).toEqual([
      "implementation",
      "verification",
      "handoff",
      "risk_review",
      "release_handoff",
    ]);
    expect(manifest.packs[0].command).toContain(
      "--pack implementation --format markdown",
    );
    expect(manifest.agentRecipes.map((recipe) => recipe.id)).toEqual([
      "cli_implementation",
      "cli_verification",
      "editor_context",
    ]);
    expect(manifest.agentRecipes[0].tools).toContain("Claude Code");
    expect(manifest.agentRecipes[1].tools).toContain("Codex");
    expect(manifest.agentRecipes[0].tools).toContain("Gemini CLI");
    expect(manifest.agentRecipes[0].tools).toContain("Aider");
    expect(manifest.agentRecipes[2].tools).toContain("Cursor");
    expect(manifest.agentRecipes[2].instruction).toContain(
      "provider routing remains manual",
    );
    expect(manifest.agentRecipes[0].command).toContain(
      "--pack implementation --format markdown",
    );
    expect(manifest.agentSessionRecipes).toHaveLength(13);
    expect(
      manifest.agentSessionRecipes.map((recipe) => recipe.id),
    ).toContain("gemini");
    expect(
      manifest.agentSessionRecipes.map((recipe) => recipe.id),
    ).toContain("zed");
    const codexSessionRecipe = manifest.agentSessionRecipes.find(
      (recipe) => recipe.id === "codex",
    );
    expect(codexSessionRecipe).toEqual(
      expect.objectContaining({
        label: "Codex",
        taskType: "verification",
        readOnly: true,
        manualProviderRouting: false,
        configReadiness: null,
      }),
    );
    expect(codexSessionRecipe?.command).toContain(
      "--session --agent codex --task verification --format markdown",
    );
    const geminiSessionRecipe = manifest.agentSessionRecipes.find(
      (recipe) => recipe.id === "gemini",
    );
    expect(geminiSessionRecipe).toEqual(
      expect.objectContaining({
        label: "Gemini CLI",
        taskType: "implementation",
        readOnly: true,
        manualProviderRouting: true,
        configReadiness: null,
      }),
    );
    expect(geminiSessionRecipe?.command).toContain(
      "--session --agent gemini --task implementation --format markdown",
    );
    expect(manifest.apiQueries.map((query) => query.command)).toEqual([
      "get_repo_manifest",
      "get_repo_pack",
      "get_agent_handoff",
      "get_index_freshness",
      "clear_repo_index",
      "search_repo_intelligence_symbols",
      "get_repo_intelligence_dependents",
    ]);
    expect(manifest.apiQueries.every((query) => query.readOnly)).toBe(true);
    expect(manifest.graph.available).toBe(true);
    expect(manifest.graph.dependencyHubCount).toBe(1);
    expect(manifest.graph.symbolCount).toBeGreaterThan(0);
    expect(manifest.graph.symbols[0]).toEqual(
      expect.objectContaining({
        name: expect.any(String),
        file: expect.any(String),
      }),
    );
    expect(manifest.graph.importEdgeCount).toBeGreaterThan(0);
    expect(manifest.graph.reverseDependencyHubCount).toBeGreaterThan(0);
    expect(manifest.graph.importEdges[0]).toEqual(
      expect.objectContaining({
        from: expect.any(String),
        to: expect.any(String),
      }),
    );

    const parsed = JSON.parse(
      formatRepoAgentManifestJson(summary, "2026-06-25T10:00:00Z"),
    );
    expect(parsed.packs[0].estimatedTokensAvoided).toBeGreaterThan(0);
    expect(JSON.stringify(parsed)).not.toContain(".env.local");
  });

  it("builds machine-readable agent handoff payloads", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
      { path: ".env.local", bytes: 300 },
    ]);
    summary.repoRoot = "/Users/me/app";

    const payload = buildRepoAgentHandoffPayload(summary, "gemini");

    expect(payload.kind).toBe("mac_ai_switchboard.repo_agent_handoff");
    expect(payload.schemaVersion).toBe(1);
    expect(payload.repoRoot).toBe("/Users/me/app");
    expect(payload.agent).toEqual(
      expect.objectContaining({
        id: "gemini",
        label: "Gemini CLI",
        toolKind: "cli",
      }),
    );
    expect(payload.pack.id).toBe("implementation");
    expect(payload.pack.estimatedTokensAvoided).toBeGreaterThan(0);
    expect(payload.pack.files.map((file) => file.path)).toContain(
      "src/App.tsx",
    );
    expect(payload.pack.files.map((file) => file.path)).not.toContain(
      ".env.local",
    );
    expect(payload.graph.available).toBe(true);
    expect(payload.graph.dependencyHubs.map((file) => file.path)).toContain(
      "package.json",
    );
    expect(payload.graph.symbols.map((symbol) => symbol.name)).toContain("App");
    expect(payload.safety).toEqual({
      readOnly: true,
      excludesSecretLikePaths: true,
      modifiesRepository: false,
      manualProviderRouting: true,
    });
    expect(payload.configReadiness).toBeUndefined();

    const cursorPayload = buildRepoAgentHandoffPayload(summary, "cursor");
    expect(cursorPayload.configReadiness).toEqual(
      expect.objectContaining({
        plannedConnectorId: "cursor",
        plannedConnectorName: "Cursor",
      }),
    );
    expect(
      cursorPayload.configReadiness?.safetyDossier.configPathStrategy,
    ).toContain("Cursor app/profile");
  });

  it("formats agent-specific bounded handoffs for popular coding tools", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
      { path: ".env.local", bytes: 300 },
    ]);
    summary.repoRoot = "/Users/me/app";

    const claude = formatRepoAgentHandoffMarkdown(summary, "claude");
    expect(claude).toContain("# Claude Code Handoff");
    expect(claude).toContain("Selected pack: Implementation Pack");
    expect(claude).toContain("bounded repo context");
    expect(claude).not.toContain("Connector Config Readiness");

    const codex = formatRepoAgentHandoffMarkdown(summary, "codex");
    expect(codex).toContain("# Codex Handoff");
    expect(codex).toContain("Selected pack: Verification Pack");
    expect(codex).toContain("repeated broad repo discovery");
    expect(codex).not.toContain("Connector Config Readiness");

    const gemini = formatRepoAgentHandoffMarkdown(summary, "gemini");
    expect(gemini).toContain("# Gemini CLI Handoff");
    expect(gemini).toContain("Selected pack: Implementation Pack");
    expect(gemini).toContain("Secret-like paths");
    expect(gemini).not.toContain("Connector Config Readiness");
    expect(gemini).toContain("src/App.tsx");
    expect(gemini).not.toContain(".env.local");

    const cursor = formatRepoAgentHandoffMarkdown(summary, "cursor");
    expect(cursor).toContain("# Cursor Handoff");
    expect(cursor).toContain("Selected pack: Handoff Pack");
    expect(cursor).toContain("Planned connector: Cursor (cursor)");
    expect(cursor).toContain("docs/install.md");
  });

  it("recommends conservative switchboard modes for agent sessions", () => {
    expect(
      recommendAgentSessionMode({
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: true,
      }),
    ).toMatchObject({
      mode: "full",
      reason: "Headroom engine and RTK are healthy.",
    });

    expect(
      recommendAgentSessionMode({
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: false,
      }).mode,
    ).toBe("rtk");

    expect(
      recommendAgentSessionMode({
        headroomHealthy: true,
        rtkHealthy: false,
        providerRoutingSafe: false,
      }).mode,
    ).toBe("off");

    expect(
      recommendAgentSessionMode({
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: true,
        cleanPassThrough: true,
      }).mode,
    ).toBe("off");
  });

  it("builds agent session preparation from a fresh local index", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
    ]);
    summary.repoRoot = "/Users/me/app";
    summary.indexedAt = "2026-06-25T10:00:00Z";

    const preparation = buildAgentSessionPreparation(summary, {
      target: "codex",
      taskType: "verification",
      generatedAt: "2026-06-25T10:05:00Z",
      modeInputs: {
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: true,
      },
    });

    expect(preparation.target.label).toBe("Codex");
    expect(preparation.taskType).toBe("verification");
    expect(preparation.packId).toBe("verification");
    expect(preparation.copyStatus).toBe("ready");
    expect(preparation.copySafety).toMatchObject({
      hasRealIndex: true,
      allowsCopy: true,
      blocksSampleContext: false,
      excludesSecretLikePaths: true,
      freshnessStatus: "fresh",
      skippedFileCount: 0,
      reason: "Fresh local index",
    });
    expect(preparation.copyArtifacts).toEqual([
      {
        id: "session_summary",
        label: "Session summary",
        format: "markdown",
        available: true,
        blockedReason: null,
      },
      {
        id: "full_handoff",
        label: "Full handoff",
        format: "markdown",
        available: true,
        blockedReason: null,
      },
      {
        id: "selected_pack",
        label: "Selected pack",
        format: "markdown",
        available: true,
        blockedReason: null,
      },
      {
        id: "json_payload",
        label: "JSON payload",
        format: "json",
        available: true,
        blockedReason: null,
      },
    ]);
    expect(preparation.recommendedMode).toBe("full");
    expect(preparation.handoffMarkdown).toContain("# Codex Handoff");
    expect(preparation.handoffMarkdown).toContain(
      "Index freshness: Fresh local index",
    );
    expect(preparation.handoffPayload?.pack.id).toBe("verification");
    expect(preparation.handoffPayload?.indexFreshness.status).toBe("fresh");
    expect(preparation.taskContext).toMatchObject({
      id: "task_verification",
      task: "verification",
      budgetTokens: 6000,
    });
    expect(preparation.taskContext?.commands).toContain("npm test");
    expect(preparation.configReadiness).toBeNull();
    expect(preparation.manifest.generatedAt).toBe("2026-06-25T10:05:00Z");
    expect(preparation.manifest.taskPacks?.[0]).toMatchObject({
      id: "task_implementation",
      fileCount: expect.any(Number),
      commandCount: expect.any(Number),
    });

    const json = formatAgentSessionPreparationJson(preparation);
    expect(json).toContain('"kind": "mac_ai_switchboard.repo_agent_handoff"');
    expect(json).toContain('"id": "codex"');

    const sessionSummary = formatAgentSessionSummaryMarkdown(preparation);
    expect(sessionSummary).toContain("# Start Agent Session Summary: Codex");
    expect(sessionSummary).toContain("Selected pack: Verification pack");
    expect(sessionSummary).toContain("Estimated tokens avoided:");
    expect(sessionSummary).toContain("Secret-like paths excluded: yes");
    expect(sessionSummary).toContain("## Task-Aware Context");
    expect(sessionSummary).toContain("Suggested commands:");

    const packMarkdown = formatAgentSessionSelectedPackMarkdown(
      summary,
      preparation,
    );
    expect(packMarkdown).toContain("# Verification Pack: /Users/me/app");
    expect(packMarkdown).toContain("Estimated tokens avoided:");
  });

  it("builds custom task context from a session query and budget", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/lib/releaseReadiness.ts", bytes: 1200 },
      { path: "src/lib/releaseReadiness.test.ts", bytes: 900 },
      { path: "src/lib/repoIntelligence.ts", bytes: 2000 },
      { path: "docs/macos-release.md", bytes: 800 },
      { path: "package.json", bytes: 700 },
    ]);
    summary.repoRoot = "/Users/me/app";
    summary.indexedAt = "2026-06-25T10:00:00Z";

    const preparation = buildAgentSessionPreparation(summary, {
      target: "codex",
      taskType: "verification",
      taskQuery: "release readiness schema smoke evidence",
      budgetTokens: 1_000,
      modeInputs: {
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: true,
      },
    });

    expect(preparation.taskContext).toMatchObject({
      task: "verification",
      budgetTokens: 1_000,
    });
    expect(preparation.taskContext?.files[0].path).toContain(
      "releaseReadiness",
    );
    expect(preparation.taskContext?.commands).toEqual(
      expect.arrayContaining(["npm run smoke:preflight"]),
    );
    expect(
      new Set(preparation.taskContext?.files.map((file) => file.path)).size,
    ).toBe(preparation.taskContext?.files.length);
    expect(formatAgentSessionSummaryMarkdown(preparation)).toContain(
      "releaseReadiness",
    );
  });

  it("exposes planned connector readiness in agent session preparation", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
    ]);
    summary.repoRoot = "/Users/me/app";
    summary.indexedAt = "2026-06-25T10:00:00Z";

    const preparation = buildAgentSessionPreparation(summary, {
      target: "cursor",
      taskType: "implementation",
      generatedAt: "2026-06-25T10:05:00Z",
      modeInputs: {
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: false,
      },
    });

    expect(preparation.configReadiness).toEqual(
      expect.objectContaining({
        plannedConnectorId: "cursor",
        plannedConnectorName: "Cursor",
        automationEnabled: false,
        nextGate: {
          id: "detect",
          label: "Detect config surface",
        },
      }),
    );
    expect(preparation.handoffPayload?.configReadiness).toBe(
      preparation.configReadiness,
    );
    expect(
      preparation.configReadiness?.gatedSteps.find(
        (step) => step.id === "dryRunDiff",
      )?.requiredEvidence.join(" "),
    ).toContain("dry-run diff artifact");

    const json = formatAgentSessionPreparationJson(preparation);
    expect(json).toContain('"configReadiness"');
    expect(json).toContain('"plannedConnectorId": "cursor"');

    const sessionSummary = formatAgentSessionSummaryMarkdown(preparation);
    expect(sessionSummary).toContain("## Connector Config Readiness");
    expect(sessionSummary).toContain("Planned connector: Cursor (cursor)");
    expect(sessionSummary).toContain("Next gate: Detect config surface");

    const display = buildAgentSessionDisplayState(preparation, true);
    expect(display.connectorReadinessLabel).toBe("Cursor config gated");
    expect(display.connectorReadinessDetailLabel).toBe(
      "Next gate: Detect config surface; automation enabled: no",
    );
  });

  it("builds risk review and release handoff agent sessions", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
    ]);
    summary.repoRoot = "/Users/me/app";
    summary.indexedAt = "2026-06-25T10:00:00Z";

    const riskReview = buildAgentSessionPreparation(summary, {
      target: "codex",
      taskType: "risk_review",
      modeInputs: {
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: true,
      },
    });
    const releaseHandoff = buildAgentSessionPreparation(summary, {
      target: "codex",
      taskType: "release_handoff",
      modeInputs: {
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: true,
      },
    });

    expect(riskReview.packId).toBe("risk_review");
    expect(riskReview.handoffPayload?.pack.title).toBe("Risk Review Pack");
    expect(releaseHandoff.packId).toBe("release_handoff");
    expect(releaseHandoff.handoffPayload?.pack.title).toBe(
      "Release Handoff Pack",
    );
  });

  it("warns when agent session preparation uses a changed cached index", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "package.json", bytes: 800 },
    ]);
    summary.indexedAt = "2026-06-25T10:00:00Z";
    summary.indexMetadata = {
      ...summary.indexMetadata!,
      cacheState: "changed",
      previousIndexedAt: "2026-06-25T09:00:00Z",
    };

    const preparation = buildAgentSessionPreparation(summary, {
      target: "gemini",
      modeInputs: {
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: false,
      },
    });

    expect(preparation.copyStatus).toBe("warn");
    expect(preparation.copySafety).toMatchObject({
      hasRealIndex: true,
      allowsCopy: true,
      blocksSampleContext: false,
      excludesSecretLikePaths: true,
      freshnessStatus: "changed_cache",
    });
    expect(preparation.copyDetail).toContain("Changed local index");
    expect(preparation.recommendedMode).toBe("rtk");
    expect(preparation.handoffMarkdown).toContain("# Gemini CLI Handoff");
    expect(preparation.handoffMarkdown).toContain(
      "Warning: Changed local index",
    );
    expect(preparation.handoffMarkdown).toContain(
      "Refresh before relying on this handoff for current code.",
    );
    expect(preparation.handoffPayload?.indexFreshness.status).toBe(
      "changed_cache",
    );
  });

  it("blocks agent session copying until a real repo index exists", () => {
    const preparation = buildAgentSessionPreparation(
      {
        totalFiles: 0,
        indexedFiles: 0,
        estimatedFullScanTokens: 0,
        roleCounts: {
          source: 0,
          test: 0,
          config: 0,
          docs: 0,
          asset: 0,
          lockfile: 0,
          generated: 0,
          unknown: 0,
        },
        packs: [],
      },
      {
        target: "cursor",
        modeInputs: {
          headroomHealthy: false,
          rtkHealthy: false,
          providerRoutingSafe: true,
        },
      },
    );

    expect(preparation.copyStatus).toBe("blocked");
    expect(preparation.copySafety).toMatchObject({
      hasRealIndex: false,
      allowsCopy: false,
      blocksSampleContext: true,
      excludesSecretLikePaths: true,
      freshnessStatus: "none",
      skippedFileCount: 0,
    });
    expect(preparation.copySafety.reason).toContain("Index a real local repo");
    expect(preparation.copyDetail).toContain("Index a real local repo");
    expect(preparation.recommendedMode).toBe("off");
    expect(preparation.handoffMarkdown).toBeNull();
    expect(preparation.handoffPayload).toBeNull();
    expect(preparation.configReadiness).toBeNull();
    expect(preparation.copyArtifacts.every((artifact) => !artifact.available))
      .toBe(true);
    expect(
      preparation.copyArtifacts.map((artifact) => artifact.blockedReason),
    ).toEqual([
      "Index a real local repo before copying agent context.",
      "Index a real local repo before copying agent context.",
      "Index a real local repo before copying agent context.",
      "Index a real local repo before copying agent context.",
    ]);
    expect(formatAgentSessionPreparationJson(preparation)).toBeNull();
    expect(formatAgentSessionSummaryMarkdown(preparation)).toBeNull();
    expect(formatAgentSessionSelectedPackMarkdown({
      totalFiles: 0,
      indexedFiles: 0,
      estimatedFullScanTokens: 0,
      roleCounts: preparation.manifest.totals.roleCounts,
      packs: [],
    }, preparation)).toBeNull();
  });

  it("builds Start Agent Session display state for copy controls", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
    ]);
    summary.repoRoot = "/Users/me/app";
    summary.indexedAt = "2026-06-25T10:00:00Z";

    const readyPreparation = buildAgentSessionPreparation(summary, {
      target: "codex",
      taskType: "verification",
      modeInputs: {
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: true,
      },
    });
    const readyDisplay = buildAgentSessionDisplayState(
      readyPreparation,
      true,
    );

    expect(readyDisplay).toMatchObject({
      targetLabel: "Codex",
      packLabel: "Verification pack",
      modeLabel: "Full optimization",
      freshnessLabel: "Fresh local index",
      freshnessDetailLabel:
        "API ready · graph ready · index new · parser metadata-fingerprint-v1 (current) · 4 indexed · 0 skipped",
      contextLabel: "Local repo index",
      selectedPackTokensLabel: "700",
      tokensAvoidedLabel: "1,300",
      skippedFilesLabel: "0 skipped",
      secretExclusionLabel: "Secret-like paths excluded",
      connectorReadinessLabel: null,
      connectorReadinessDetailLabel: null,
      sampleContextWarning: null,
      copyStatus: "ready",
      canCopySummary: true,
      canCopyHandoff: true,
      canCopySelectedPack: true,
      canCopyJson: true,
    });

    summary.indexMetadata = {
      ...summary.indexMetadata!,
      cacheState: "changed",
      previousIndexedAt: "2026-06-25T09:00:00Z",
    };
    const stalePreparation = buildAgentSessionPreparation(summary, {
      target: "cursor",
      taskType: "implementation",
      modeInputs: {
        headroomHealthy: true,
        rtkHealthy: true,
        providerRoutingSafe: false,
      },
    });
    const staleDisplay = buildAgentSessionDisplayState(
      stalePreparation,
      true,
    );

    expect(staleDisplay.copyStatus).toBe("warn");
    expect(staleDisplay.freshnessLabel).toBe("Changed local index");
    expect(staleDisplay.freshnessLabel).not.toBe("Fresh local index");
    expect(staleDisplay.freshnessDetailLabel).toBe(
      "API ready · graph ready · index changed · parser metadata-fingerprint-v1 (current) · 4 indexed · 0 skipped · Repo metadata changed since the previous saved index.",
    );
    expect(staleDisplay.modeLabel).toBe("RTK only");
    expect(staleDisplay.copyDetail).toContain("Changed local index");
    expect(staleDisplay.connectorReadinessLabel).toBe("Cursor config gated");
    expect(staleDisplay.connectorReadinessDetailLabel).toBe(
      "Next gate: Detect config surface; automation enabled: no",
    );
    expect(staleDisplay.canCopyHandoff).toBe(true);
    expect(staleDisplay.canCopySummary).toBe(true);
    expect(staleDisplay.canCopySelectedPack).toBe(true);
    expect(staleDisplay.canCopyJson).toBe(true);

    const artifactBlockedDisplay = buildAgentSessionDisplayState(
      {
        ...readyPreparation,
        copyArtifacts: readyPreparation.copyArtifacts.map((artifact) =>
          artifact.id === "selected_pack"
            ? {
                ...artifact,
                available: false,
                blockedReason: "Selected pack unavailable.",
              }
            : artifact,
        ),
      },
      true,
    );

    expect(artifactBlockedDisplay.canCopyHandoff).toBe(true);
    expect(artifactBlockedDisplay.canCopySelectedPack).toBe(false);
    expect(artifactBlockedDisplay.canCopyJson).toBe(true);

    const blockedDisplay = buildAgentSessionDisplayState(
      buildAgentSessionPreparation(
        {
          totalFiles: 0,
          indexedFiles: 0,
          estimatedFullScanTokens: 0,
          roleCounts: readyPreparation.manifest.totals.roleCounts,
          packs: [],
        },
        {
          target: "cursor",
          modeInputs: {
            headroomHealthy: false,
            rtkHealthy: false,
            providerRoutingSafe: true,
          },
        },
      ),
      false,
    );

    expect(blockedDisplay).toMatchObject({
      targetLabel: "Cursor",
      packLabel: "Handoff pack",
      modeLabel: "Off",
      contextLabel: "Sample preview",
      selectedPackTokensLabel: "0",
      tokensAvoidedLabel: "0",
      skippedFilesLabel: "0 skipped",
      secretExclusionLabel: "Secret-like paths excluded",
      copyStatus: "blocked",
      canCopySummary: false,
      canCopyHandoff: false,
      canCopySelectedPack: false,
      canCopyJson: false,
    });
    expect(blockedDisplay.sampleContextWarning).toContain(
      "Sample preview packs are blocked",
    );
  });

  it("labels agent session packs consistently", () => {
    expect(repoAgentPackLabel("implementation")).toBe("Implementation pack");
    expect(repoAgentPackLabel("verification")).toBe("Verification pack");
    expect(repoAgentPackLabel("handoff")).toBe("Handoff pack");
    expect(repoAgentPackLabel("risk_review")).toBe("Risk review pack");
    expect(repoAgentPackLabel("release_handoff")).toBe("Release handoff pack");
  });

  it("calculates best-pack and all-pack token savings", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "src/lib/types.ts", bytes: 1000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
    ]);

    const estimate = estimateRepoIntelligenceSavings(summary);

    expect(estimate.fullScanTokens).toBe(summary.estimatedFullScanTokens);
    expect(estimate.bestPack?.id).toBe("handoff");
    expect(estimate.bestPackTokensAvoided).toBeGreaterThan(0);
    expect(estimate.bestPackSavingsPct).toBeGreaterThan(
      estimate.allPacksSavingsPct,
    );
    expect(estimate.allPacksTokensAvoided).toBeGreaterThanOrEqual(0);
  });

  it("returns zero savings for an empty repo index", () => {
    const estimate = estimateRepoIntelligenceSavings({
      totalFiles: 0,
      indexedFiles: 0,
      estimatedFullScanTokens: 0,
      roleCounts: {
        source: 0,
        test: 0,
        config: 0,
        docs: 0,
        asset: 0,
        lockfile: 0,
        generated: 0,
        unknown: 0,
      },
      packs: [],
    });

    expect(estimate.bestPack).toBeUndefined();
    expect(estimate.bestPackTokensAvoided).toBe(0);
    expect(estimate.bestPackSavingsPct).toBe(0);
    expect(estimate.allPacksTokensAvoided).toBe(0);
    expect(estimate.allPacksSavingsPct).toBe(0);
  });
});
