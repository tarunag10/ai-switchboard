import { describe, expect, it } from "vitest";

import {
  createRepoMapHistoryItem,
  normalizeRepoMapError,
  readRepoMapHistory,
  REPO_MAP_HISTORY_KEY,
  type RepoMapGenerationResponse,
  type RepoMapHistoryItem,
  upsertRepoMapHistory,
  writeRepoMapHistory,
} from "./repoMapJob";

function generation(repoPath: string, generatedAt = "2026-07-05T10:00:00.000Z") {
  return {
    repoPath,
    outDir: `${repoPath}/docs/repo-map`,
    map: {
      generatedAt,
      tools: {
        graphify: {
          nodeCount: 42,
        },
      },
      tokenSavings: {
        estimatedTokensAvoided: 1234,
      },
    },
  } as RepoMapGenerationResponse;
}

describe("repoMapJob", () => {
  it("creates compact history items from generation results", () => {
    expect(createRepoMapHistoryItem(generation("/repo"))).toEqual({
      repoPath: "/repo",
      generatedAt: "2026-07-05T10:00:00.000Z",
      outDir: "/repo/docs/repo-map",
      graphNodes: 42,
      estimatedTokensAvoided: 1234,
    });
  });

  it("dedupes generated repo history and caps it at eight entries", () => {
    const history: RepoMapHistoryItem[] = Array.from({ length: 8 }, (_, index) => ({
      repoPath: `/repo-${index}`,
      generatedAt: `2026-07-05T10:0${index}:00.000Z`,
      outDir: `/repo-${index}/docs/repo-map`,
      graphNodes: index,
      estimatedTokensAvoided: index,
    }));

    const next = upsertRepoMapHistory(history, generation("/repo-3", "2026-07-05T11:00:00.000Z"));

    expect(next).toHaveLength(8);
    expect(next[0]).toMatchObject({
      repoPath: "/repo-3",
      generatedAt: "2026-07-05T11:00:00.000Z",
    });
    expect(next.filter((item) => item.repoPath === "/repo-3")).toHaveLength(1);
  });

  it("persists bounded history and tolerates corrupt storage", () => {
    const storage = window.localStorage;
    storage.clear();

    const entries = Array.from({ length: 10 }, (_, index) => ({
      repoPath: `/repo-${index}`,
      generatedAt: `2026-07-05T10:${String(index).padStart(2, "0")}:00.000Z`,
      outDir: `/repo-${index}/docs/repo-map`,
      graphNodes: index,
      estimatedTokensAvoided: index,
    }));

    writeRepoMapHistory(storage, entries);

    expect(readRepoMapHistory(storage)).toHaveLength(8);

    storage.setItem(REPO_MAP_HISTORY_KEY, "{not json");
    expect(readRepoMapHistory(storage)).toEqual([]);
  });

  it("normalizes thrown values for UI notices", () => {
    expect(normalizeRepoMapError(new Error("boom"))).toBe("boom");
    expect(normalizeRepoMapError("plain failure")).toBe("plain failure");
  });
});

