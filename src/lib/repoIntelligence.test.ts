import { describe, expect, it } from "vitest";

import {
  buildRepoIntelligenceSummary,
  classifyRepoFile,
  estimateRepoTokens,
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
});
