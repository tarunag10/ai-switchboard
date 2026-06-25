import { describe, expect, it } from "vitest";

import {
  buildRepoIntelligenceSummary,
  classifyRepoFile,
  estimateRepoIntelligenceSavings,
  estimateRepoTokens,
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
      { path: ".env.local", bytes: 200 },
    ]);

    expect(summary.graph?.topDirectories[0].label).toBe("src");
    expect(summary.graph?.topLanguages.map((node) => node.label)).toContain("React");
    expect(summary.graph?.entrypoints.map((file) => file.path)).toContain("src/main.tsx");
    expect(summary.graph?.likelyTests.map((file) => file.path)).toContain("src/App.test.tsx");
    expect(summary.graph?.configHubs.map((file) => file.path)).toContain("package.json");
    expect(summary.graph?.configHubs.map((file) => file.path)).not.toContain(".env.local");
  });

  it("formats bounded context packs for agent handoff", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
      { path: "src/App.test.tsx", bytes: 2000 },
      { path: "docs/install.md", bytes: 1200 },
      { path: "package.json", bytes: 800 },
    ]);
    summary.repoRoot = "/Users/me/app";
    summary.indexedAt = "2026-06-25T10:00:00Z";

    const markdown = formatRepoContextPackMarkdown(summary);

    expect(markdown).toContain("# Repo Intelligence Context Pack: /Users/me/app");
    expect(markdown).toContain("Indexed at: 2026-06-25T10:00:00Z");
    expect(markdown).toContain("## Repo Graph Summary");
    expect(markdown).toContain("Top directories");
    expect(markdown).toContain("Likely tests");
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
