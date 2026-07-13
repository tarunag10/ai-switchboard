import { invoke } from "@tauri-apps/api/core";

export interface RepoMapToolRunStatus {
  status: "ok" | "warning" | "not-run";
  detail: string;
  remediation: string | null;
}

export interface RepoMapTokenSavings {
  compactContextEstimatedTokens: number;
  broadScanEstimatedTokens: number;
  estimatedTokensAvoided: number;
  method: string;
}

export type RepoMapSnapshot = typeof import("../../docs/repo-map/repo-map.json") & {
  toolRuns?: Record<string, RepoMapToolRunStatus>;
  tokenSavings?: RepoMapTokenSavings;
};

export interface RepoMapGenerationResponse {
  repoPath: string;
  outDir: string;
  readmePath: string;
  compactContextPath: string;
  map: RepoMapSnapshot;
  compactContext: string;
  toolLog: unknown;
  stdoutTail: string;
  stderrTail: string;
}

export interface RepoMapPreflightTool {
  id: string;
  label: string;
  available: boolean;
  detail: string;
  installHint: string | null;
}

export interface RepoMapPreflightResponse {
  repoPath: string;
  exists: boolean;
  isDirectory: boolean;
  hasPackageJson: boolean;
  hasCargoManifest: boolean;
  tools: RepoMapPreflightTool[];
}

export interface RepoMapHistoryItem {
  repoPath: string;
  generatedAt: string;
  outDir: string;
  graphNodes: number;
  estimatedTokensAvoided: number;
}

export interface RepoMapArtifactRequest {
  repoPath: string;
  artifact: string;
}

export interface RepoMapJobAdapter {
  preflight(repoPath: string | null): Promise<RepoMapPreflightResponse>;
  generate(repoPath: string | null): Promise<RepoMapGenerationResponse>;
  cancel(): Promise<boolean>;
  openArtifact(request: RepoMapArtifactRequest): Promise<boolean>;
}

export const REPO_MAP_HISTORY_KEY = "mac-ai-switchboard:repoMapHistory";
export const DEFAULT_REPO_MAP_REPO_PATH =
  "/Users/tarunagarwal/Developer/Codex-Repos/mac-ai-switchboard";

export const repoMapTauriAdapter: RepoMapJobAdapter = {
  preflight(repoPath) {
    return invoke<RepoMapPreflightResponse>("preflight_repo_map", { repoPath });
  },
  generate(repoPath) {
    return invoke<RepoMapGenerationResponse>("generate_repo_map", { repoPath });
  },
  cancel() {
    return invoke<boolean>("cancel_repo_map_generation");
  },
  openArtifact(request) {
    return invoke<boolean>("open_repo_map_artifact", { request });
  },
};

export function normalizeRepoMapError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function readRepoMapHistory(storage: Storage): RepoMapHistoryItem[] {
  try {
    const raw = storage.getItem(REPO_MAP_HISTORY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.slice(0, 8) : [];
  } catch {
    return [];
  }
}

export function writeRepoMapHistory(storage: Storage, items: RepoMapHistoryItem[]) {
  try {
    storage.setItem(REPO_MAP_HISTORY_KEY, JSON.stringify(items.slice(0, 8)));
  } catch {
    // History is a convenience only.
  }
}

export function createRepoMapHistoryItem(
  result: RepoMapGenerationResponse,
): RepoMapHistoryItem {
  return {
    repoPath: result.repoPath,
    generatedAt: result.map.generatedAt,
    outDir: result.outDir,
    graphNodes: result.map.tools.graphify.nodeCount,
    estimatedTokensAvoided: result.map.tokenSavings?.estimatedTokensAvoided ?? 0,
  };
}

export function upsertRepoMapHistory(
  history: RepoMapHistoryItem[],
  result: RepoMapGenerationResponse,
): RepoMapHistoryItem[] {
  const item = createRepoMapHistoryItem(result);
  return [item, ...history.filter((entry) => entry.repoPath !== item.repoPath)].slice(0, 8);
}
