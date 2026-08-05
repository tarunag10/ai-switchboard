import type { RepoSavingsEstimate } from "./repoIntelligence";
import { describeCompressionAttributionPolicy } from "./compressionAttributionRules";
import type {
  DashboardState,
  RtkTodayStats,
  RuntimeStatus,
  ContentClassCompressionStats,
} from "./types";

export type CompressionConfidence =
  | "measured"
  | "estimated"
  | "inferred"
  | "external";

export type CompressionSourceFamily =
  | "headroom"
  | "rtk"
  | "cache"
  | "repo-intelligence"
  | "addon";

export type CompressionDrillDown =
  | "token-xray"
  | "addons"
  | "repo-intelligence"
  | "usage-savings";

export interface CompressionSourceRow {
  id: string;
  family: CompressionSourceFamily;
  label: string;
  tokensSaved: number | null;
  confidence: CompressionConfidence;
  detail: string;
  caveat: string | null;
  drillDown: CompressionDrillDown | null;
}

export interface CompressionDashboardOverview {
  scope: "session" | "today";
  compressionTokensSaved: number | null;
  compressionConfidence: CompressionConfidence;
  sources: CompressionSourceRow[];
  caveats: string[];
  generatedAt: string;
}

export interface CompressionDashboardSemanticCache {
  enabled: boolean;
  hits: number;
  misses: number;
  evidence?: string;
}

export interface CompressionDashboardAddonRow {
  id: string;
  label: string;
  tokensSaved: number;
  confidence: Extract<CompressionConfidence, "estimated" | "inferred">;
  detail: string;
}

export interface CompressionDashboardInput {
  scope?: "session" | "today";
  dashboard?: DashboardState | null;
  runtimeStatus?: RuntimeStatus | null;
  semanticCache?: CompressionDashboardSemanticCache | null;
  repoSavings?: RepoSavingsEstimate | null;
  addonRows?: CompressionDashboardAddonRow[];
  rtkToday?: RtkTodayStats | null;
  contentClass?: ContentClassCompressionStats | null;
  generatedAt?: string;
}

function summarizeRtkToday(
  runtimeStatus: RuntimeStatus | null | undefined,
  rtkToday: RtkTodayStats | null | undefined,
  scope: "session" | "today",
): number | null {
  if (scope === "today") {
    const todaySaved = Math.max(0, rtkToday?.savedTokens ?? 0);
    if (todaySaved > 0) return todaySaved;
    const daily = runtimeStatus?.rtk.daily ?? [];
    const todayKey = new Date().toISOString().slice(0, 10);
    const match = daily.find((row) => row.date === todayKey);
    if (match && match.savedTokens > 0) return match.savedTokens;
    return null;
  }
  const lifetimeSaved = Math.max(0, runtimeStatus?.rtk.totalSaved ?? 0);
  return lifetimeSaved > 0 ? lifetimeSaved : null;
}

function headroomTokensSaved(
  dashboard: DashboardState | null | undefined,
  scope: "session" | "today",
): number | null {
  if (!dashboard) return null;
  if (scope === "session") {
    const saved = Math.max(0, dashboard.sessionEstimatedTokensSaved ?? 0);
    return saved > 0 ? saved : null;
  }
  const saved = Math.max(0, dashboard.lifetimeEstimatedTokensSaved ?? 0);
  return saved > 0 ? saved : null;
}

function dominantConfidence(
  rows: CompressionSourceRow[],
): CompressionConfidence {
  if (rows.some((row) => row.confidence === "measured")) return "measured";
  if (rows.some((row) => row.confidence === "estimated")) return "estimated";
  if (rows.some((row) => row.confidence === "inferred")) return "inferred";
  return "estimated";
}

/** Normalizes attribution inputs into one compression overview read model. */
export function buildCompressionDashboardOverview(
  input: CompressionDashboardInput = {},
): CompressionDashboardOverview {
  const scope = input.scope ?? "today";
  const sources: CompressionSourceRow[] = [];
  const caveats: string[] = [
    describeCompressionAttributionPolicy(),
    "Cache replay savings are shown separately from live-request compression.",
    "Sources without data are omitted rather than shown as zero measured savings.",
  ];

  const headroomSaved = headroomTokensSaved(input.dashboard, scope);
  if (headroomSaved !== null) {
    sources.push({
      id: "headroom",
      family: "headroom",
      label: "Headroom",
      tokensSaved: headroomSaved,
      confidence: scope === "session" ? "measured" : "estimated",
      detail:
        scope === "session"
          ? "Runtime compression measured in the current app session."
          : "Runtime compression recorded across saved Switchboard history.",
      caveat:
        scope === "session"
          ? null
          : "Historical Headroom savings remain estimated until provider-billed counterfactual pairs are complete.",
      drillDown: "token-xray",
    });
  }

  const contentClassRows: Array<{
    id: string;
    label: string;
    tokens: number | null;
  }> = [
    {
      id: "headroom-tool-results",
      label: "Tool results",
      tokens: input.contentClass?.toolResultTokens ?? null,
    },
    {
      id: "headroom-history",
      label: "History",
      tokens: input.contentClass?.historyTokens ?? null,
    },
    {
      id: "headroom-user-messages",
      label: "User messages",
      tokens: input.contentClass?.userMessageTokens ?? null,
    },
  ];
  for (const row of contentClassRows) {
    if (row.tokens === null) continue;
    sources.push({
      id: row.id,
      family: "headroom",
      label: `Headroom · ${row.label}`,
      tokensSaved: row.tokens,
      confidence: "measured",
      detail: `Content-class compression savings from Headroom /stats (${row.label.toLowerCase()}).`,
      caveat: null,
      drillDown: "token-xray",
    });
  }

  const rtkSaved = summarizeRtkToday(
    input.runtimeStatus,
    input.rtkToday,
    scope,
  );
  if (rtkSaved !== null) {
    sources.push({
      id: "rtk",
      family: "rtk",
      label: "RTK",
      tokensSaved: rtkSaved,
      confidence: "measured",
      detail:
        scope === "today"
          ? "Command-output compression from RTK daily stats."
          : "Command-output compression from recorded RTK history.",
      caveat: null,
      drillDown: "addons",
    });
  }

  const cacheHits = Math.max(0, input.semanticCache?.hits ?? 0);
  const cacheMisses = Math.max(0, input.semanticCache?.misses ?? 0);
  if (input.semanticCache && (cacheHits > 0 || cacheMisses > 0 || input.semanticCache.enabled)) {
    sources.push({
      id: "cache",
      family: "cache",
      label: "Exact Replay Cache",
      tokensSaved: null,
      confidence: cacheHits > 0 ? "estimated" : "estimated",
      detail:
        cacheHits > 0 || cacheMisses > 0
          ? `${cacheHits.toLocaleString()} cache hit${cacheHits === 1 ? "" : "s"}, ${cacheMisses.toLocaleString()} miss${cacheMisses === 1 ? "" : "es"}.`
          : input.semanticCache.enabled
            ? "Exact cache is enabled; no replay hits recorded yet."
            : "Exact cache counters are available but disabled.",
      caveat:
        input.semanticCache.evidence?.trim() ||
        "Cache hits are estimated until a counterfactual provider pair exists.",
      drillDown: "addons",
    });
  }

  const repoSaved = Math.max(0, input.repoSavings?.bestPackTokensAvoided ?? 0);
  if (repoSaved > 0) {
    sources.push({
      id: "repo-intelligence",
      family: "repo-intelligence",
      label: "Repo Intelligence",
      tokensSaved: repoSaved,
      confidence: "estimated",
      detail: `${input.repoSavings?.bestPack?.title ?? "Best context pack"} avoids a broad full-repo scan.`,
      caveat: "Context-avoidance estimate; not live Headroom compression.",
      drillDown: "repo-intelligence",
    });
  }

  for (const addon of input.addonRows ?? []) {
    if (addon.tokensSaved <= 0) continue;
    sources.push({
      id: addon.id,
      family: "addon",
      label: addon.label,
      tokensSaved: addon.tokensSaved,
      confidence: addon.confidence,
      detail: addon.detail,
      caveat: "Add-on savings stay separate from Headroom compression attribution.",
      drillDown: "addons",
    });
  }

  const compressionRows = sources.filter((row) => row.family !== "cache");
  const compressionTokensSaved = compressionRows.every(
    (row) => row.tokensSaved === null,
  )
    ? null
    : compressionRows.reduce((sum, row) => sum + (row.tokensSaved ?? 0), 0);

  if (compressionRows.length === 0) {
    caveats.push("No compression sources have recorded savings yet.");
  }

  return {
    scope,
    compressionTokensSaved:
      compressionTokensSaved && compressionTokensSaved > 0
        ? compressionTokensSaved
        : null,
    compressionConfidence: dominantConfidence(compressionRows),
    sources,
    caveats,
    generatedAt: input.generatedAt ?? new Date().toISOString(),
  };
}

export function formatCompressionConfidence(
  confidence: CompressionConfidence,
): string {
  switch (confidence) {
    case "measured":
      return "Measured";
    case "estimated":
      return "Estimated";
    case "inferred":
      return "Inferred";
    case "external":
      return "External";
  }
}
