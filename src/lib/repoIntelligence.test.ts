import { describe, expect, it } from "vitest";

import {
  buildAgentSessionPreparation,
  buildRepoIntelligenceSummary,
  buildRepoAgentManifest,
  buildRepoAgentHandoffPayload,
  classifyRepoFile,
  estimateRepoIntelligenceSavings,
  estimateRepoTokens,
  formatAgentSessionPreparationJson,
  formatAgentSessionSelectedPackMarkdown,
  formatRepoAgentManifestJson,
  formatRepoAgentHandoffMarkdown,
  formatRepoContextPackMarkdown,
  formatSingleRepoContextPackMarkdown,
  getRepoIndexFreshness,
  isSecretLikeRepoPath,
  recommendAgentSessionMode,
} from "./repoIntelligence";

describe("repoIntelligence", () => {
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
    expect(summary.indexerVersion).toBe("path-graph-v2");
    expect(summary.roleCounts.generated).toBe(1);
    expect(summary.indexMetadata).toMatchObject({
      schemaVersion: 1,
      indexerVersion: "path-graph-v2",
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
  });

  it("derives index freshness copy from persistent metadata", () => {
    expect(getRepoIndexFreshness({}).status).toBe("none");
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
        indexMetadata: baseMetadata,
      }),
    ).toMatchObject({ status: "fresh", label: "Fresh local index" });
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
        content: 'import { helper } from "./lib/helper";\nhelper();',
      },
      { path: "src/main.tsx", bytes: 1400, content: 'import "./App";' },
      { path: "src/App.test.tsx", bytes: 2000 },
      {
        path: "src/lib/helper.ts",
        bytes: 900,
        content: "export function helper() { return true; }",
      },
      { path: "src-tauri/src/lib.rs", bytes: 5000 },
      { path: "scripts/release.mjs", bytes: 1200 },
      { path: "package.json", bytes: 800 },
      { path: "package-lock.json", bytes: 1600 },
      { path: ".env.local", bytes: 200 },
    ]);

    expect(summary.graph?.topDirectories[0].label).toBe("src");
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
      ]),
    );
    expect(
      summary.graph?.reverseDependencyHubs?.map((node) => node.label),
    ).toContain("package.json");
    expect(summary.graph?.symbols?.map((symbol) => symbol.name)).toContain(
      "App",
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
    expect(manifest.totals.indexerVersion).toBe("path-graph-v2");
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
    expect(payload.configReadiness).toEqual(
      expect.objectContaining({
        plannedConnectorId: "gemini_cli",
        automationEnabled: false,
      }),
    );
    expect(payload.configReadiness?.gatedSteps.map((step) => step.id)).toEqual([
      "detect",
      "dryRunDiff",
      "backup",
      "apply",
      "verify",
      "rollback",
      "offCleanup",
    ]);
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
    expect(gemini).toContain("## Connector Config Readiness");
    expect(gemini).toContain("Planned connector: gemini_cli");
    expect(gemini).toContain("Clean up in Off mode");
    expect(gemini).toContain("src/App.tsx");
    expect(gemini).not.toContain(".env.local");

    const cursor = formatRepoAgentHandoffMarkdown(summary, "cursor");
    expect(cursor).toContain("# Cursor Handoff");
    expect(cursor).toContain("Selected pack: Handoff Pack");
    expect(cursor).toContain("Planned connector: cursor");
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
    expect(preparation.recommendedMode).toBe("full");
    expect(preparation.handoffMarkdown).toContain("# Codex Handoff");
    expect(preparation.handoffPayload?.pack.id).toBe("verification");
    expect(preparation.manifest.generatedAt).toBe("2026-06-25T10:05:00Z");

    const json = formatAgentSessionPreparationJson(preparation);
    expect(json).toContain('"kind": "mac_ai_switchboard.repo_agent_handoff"');
    expect(json).toContain('"id": "codex"');

    const packMarkdown = formatAgentSessionSelectedPackMarkdown(
      summary,
      preparation,
    );
    expect(packMarkdown).toContain("# Verification Pack: /Users/me/app");
    expect(packMarkdown).toContain("Estimated tokens avoided:");
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
    expect(preparation.copyDetail).toContain("Changed local index");
    expect(preparation.recommendedMode).toBe("rtk");
    expect(preparation.handoffMarkdown).toContain("# Gemini CLI Handoff");
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
    expect(preparation.copyDetail).toContain("Index a real local repo");
    expect(preparation.recommendedMode).toBe("off");
    expect(preparation.handoffMarkdown).toBeNull();
    expect(preparation.handoffPayload).toBeNull();
    expect(formatAgentSessionPreparationJson(preparation)).toBeNull();
    expect(formatAgentSessionSelectedPackMarkdown({
      totalFiles: 0,
      indexedFiles: 0,
      estimatedFullScanTokens: 0,
      roleCounts: preparation.manifest.totals.roleCounts,
      packs: [],
    }, preparation)).toBeNull();
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
