import { describe, expect, it } from "vitest";

import {
  buildRepoIntelligenceSummary,
  buildRepoAgentManifest,
  classifyRepoFile,
  estimateRepoIntelligenceSavings,
  estimateRepoTokens,
  formatRepoAgentManifestJson,
  formatRepoAgentHandoffMarkdown,
  formatRepoContextPackMarkdown,
  formatSingleRepoContextPackMarkdown,
  isSecretLikeRepoPath,
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
    expect(isSecretLikeRepoPath(".private_keys/AuthKey_ABC123XYZ.p8")).toBe(true);
    expect(isSecretLikeRepoPath("config/service-account.pem")).toBe(true);

    const envFile = classifyRepoFile(".env.local", 100);
    expect(envFile.role).toBe("config");
    expect(envFile.includeByDefault).toBe(false);
    expect(envFile.reasons).toContain("secret-like path excluded");

    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: ".env.local", bytes: 400 },
      { path: ".private_keys/AuthKey_ABC123XYZ.p8", bytes: 800 },
    ]);
    const packedPaths = summary.packs.flatMap((pack) =>
      pack.files.map((file) => file.path),
    );

    expect(packedPaths).toContain("src/App.tsx");
    expect(packedPaths).not.toContain(".env.local");
    expect(packedPaths).not.toContain(".private_keys/AuthKey_ABC123XYZ.p8");
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
    expect(summary.roleCounts.generated).toBe(1);
    expect(summary.packs).toHaveLength(3);
    expect(summary.packs[0].id).toBe("implementation");
    expect(summary.packs[0].files.map((file) => file.path)).toContain("src/App.tsx");
    expect(summary.packs[0].savingsVsFullScanPct).toBeGreaterThan(50);
  });

  it("builds a bounded repo graph summary for agent context", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/main.tsx", bytes: 1400 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "src-tauri/src/lib.rs", bytes: 5000 },
{ path: "scripts/release.mjs", bytes: 1200 },
{ path: "package.json", bytes: 800 },
{ path: "package-lock.json", bytes: 1600 },
{ path: ".env.local", bytes: 200 },
    ]);

    expect(summary.graph?.topDirectories[0].label).toBe("src");
    expect(summary.graph?.topLanguages.map((node) => node.label)).toContain("React");
    expect(summary.graph?.entrypoints.map((file) => file.path)).toContain("src/main.tsx");
    expect(summary.graph?.likelyTests.map((file) => file.path)).toContain("src/App.test.tsx");
expect(summary.graph?.configHubs.map((file) => file.path)).toContain("package.json");
expect(summary.graph?.configHubs.map((file) => file.path)).not.toContain(".env.local");
expect(summary.graph?.dependencyHubs?.map((file) => file.path)).toEqual([
"package.json",
]);
expect(summary.graph?.importEdges).toEqual(expect.arrayContaining([
expect.objectContaining({ from: "src/App.test.tsx", to: "src/App.tsx", kind: "test_to_source" }),
expect.objectContaining({ from: "src/main.tsx", to: "package.json", kind: "entrypoint_to_config" }),
]));
expect(summary.graph?.reverseDependencyHubs?.map((node) => node.label)).toContain("package.json");
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

    expect(markdown).toContain("# Repo Intelligence Context Pack: /Users/me/app");
    expect(markdown).toContain("Indexed at: 2026-06-25T10:00:00Z");
    expect(markdown).toContain("## Repo Graph Summary");
expect(markdown).toContain("Top directories");
expect(markdown).toContain("Likely tests");
expect(markdown).toContain("Dependency hubs");
expect(markdown).toContain("Import and dependency edges");
expect(markdown).toContain("Reverse dependency hubs");
expect(markdown).toContain("- package.json");
expect(markdown).toContain("## Implementation Pack");
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

    const markdown = formatSingleRepoContextPackMarkdown(summary, summary.packs[0]);

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
  expect(manifest.safety).toEqual({
    readOnly: true,
    excludesSecretLikePaths: true,
    modifiesRepository: false,
  });
  expect(manifest.packs.map((pack) => pack.id)).toEqual([
    "implementation",
    "verification",
    "handoff",
  ]);
  expect(manifest.packs[0].command).toContain("--pack implementation --format markdown");
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
  expect(manifest.agentRecipes[2].instruction).toContain("provider routing remains manual");
  expect(manifest.agentRecipes[0].command).toContain("--pack implementation --format markdown");
  expect(manifest.graph.available).toBe(true);
expect(manifest.graph.dependencyHubCount).toBe(1);
expect(manifest.graph.importEdgeCount).toBeGreaterThan(0);
expect(manifest.graph.reverseDependencyHubCount).toBeGreaterThan(0);
expect(manifest.graph.importEdges[0]).toEqual(expect.objectContaining({ from: expect.any(String), to: expect.any(String) }));

  const parsed = JSON.parse(formatRepoAgentManifestJson(summary, "2026-06-25T10:00:00Z"));
  expect(parsed.packs[0].estimatedTokensAvoided).toBeGreaterThan(0);
    expect(JSON.stringify(parsed)).not.toContain(".env.local");
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

    const codex = formatRepoAgentHandoffMarkdown(summary, "codex");
    expect(codex).toContain("# Codex Handoff");
    expect(codex).toContain("Selected pack: Verification Pack");
    expect(codex).toContain("repeated broad repo discovery");

    const gemini = formatRepoAgentHandoffMarkdown(summary, "gemini");
    expect(gemini).toContain("# Gemini CLI Handoff");
    expect(gemini).toContain("Selected pack: Implementation Pack");
    expect(gemini).toContain("Secret-like paths");
    expect(gemini).toContain("src/App.tsx");
    expect(gemini).not.toContain(".env.local");

    const cursor = formatRepoAgentHandoffMarkdown(summary, "cursor");
    expect(cursor).toContain("# Cursor Handoff");
    expect(cursor).toContain("Selected pack: Handoff Pack");
    expect(cursor).toContain("docs/install.md");
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
    expect(estimate.bestPackSavingsPct).toBeGreaterThan(estimate.allPacksSavingsPct);
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
