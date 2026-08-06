import { invoke } from "@tauri-apps/api/core";
import { ArrowClockwise, ChartBar } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import {
  buildCompressionDashboardOverview,
  formatCompressionConfidence,
  type CompressionDashboardOverview,
  type CompressionSourceRow,
} from "../lib/compressionDashboard";
import {
  buildAddonSavingsEstimate,
  CAVEMAN_TEMPLATE_BASELINE_TOKENS,
  CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS,
  MARKITDOWN_TEMPLATE_BASELINE_TOKENS,
  MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS,
  PONYTAIL_TEMPLATE_BASELINE_TOKENS,
  PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS,
} from "../lib/savingsCalculator";
import {
  estimateRepoIntelligenceSavings,
  type RepoIntelligenceSummary,
} from "../lib/repoIntelligence";
import type { DashboardState, RuntimeStatus, ContentClassCompressionStats } from "../lib/types";

type SemanticCacheStatus = {
  enabled: boolean;
  hits: number;
  misses: number;
  evidence?: string;
};

export function OptimizationCompressionOverview() {
  const [overview, setOverview] = useState<CompressionDashboardOverview | null>(
    null,
  );
  const [loading, setLoading] = useState(true);

  async function refresh() {
    setLoading(true);
    try {
      const [dashboard, runtimeStatus, semanticCache, repoSummary, contentClass] =
        await Promise.all([
          invoke<DashboardState>("get_dashboard_state").catch(() => null),
          invoke<RuntimeStatus>("get_runtime_status").catch(() => null),
          invoke<SemanticCacheStatus>("get_semantic_cache_status").catch(
            () => null,
          ),
          invoke<RepoIntelligenceSummary | null>(
            "get_latest_repo_intelligence_summary",
          ).catch(() => null),
          invoke<ContentClassCompressionStats>("get_headroom_content_class_stats").catch(
            () => null,
          ),
        ]);
      const repoSavings = repoSummary
        ? estimateRepoIntelligenceSavings(repoSummary)
        : null;
      const addonRows = [
        {
          id: "caveman",
          label: "Caveman",
          ...buildAddonSavingsEstimate(
            CAVEMAN_TEMPLATE_BASELINE_TOKENS,
            CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS,
          ),
          confidence: "inferred" as const,
          detail: "Terse handoff template vs a verbose baseline.",
        },
        {
          id: "ponytail",
          label: "Ponytail",
          ...buildAddonSavingsEstimate(
            PONYTAIL_TEMPLATE_BASELINE_TOKENS,
            PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS,
          ),
          confidence: "inferred" as const,
          detail: "Bounded change slices vs an unbounded rewrite baseline.",
        },
        {
          id: "markitdown",
          label: "MarkItDown",
          ...buildAddonSavingsEstimate(
            MARKITDOWN_TEMPLATE_BASELINE_TOKENS,
            MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS,
          ),
          confidence: "estimated" as const,
          detail: "Markdown extract vs re-attaching the full source document.",
        },
      ]
        .filter((row) => row.tokensAvoided > 0)
        .map((row) => ({
          id: row.id,
          label: row.label,
          tokensSaved: row.tokensAvoided,
          confidence: row.confidence,
          detail: row.detail,
        }));

      setOverview(
        buildCompressionDashboardOverview({
          scope: "today",
          dashboard,
          runtimeStatus,
          semanticCache,
          repoSavings,
          addonRows,
          contentClass,
        }),
      );
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  function renderRow(row: CompressionSourceRow) {
    return (
      <div className="compression-overview__row" key={row.id}>
        <div className="compression-overview__row-main">
          <span className="compression-overview__row-label">{row.label}</span>
          <span className="compression-overview__row-confidence">
            {formatCompressionConfidence(row.confidence)}
          </span>
          <p className="optimize-minimal__meta">{row.detail}</p>
          {row.caveat ? (
            <p className="compression-overview__caveat">{row.caveat}</p>
          ) : null}
        </div>
        <span className="compression-overview__row-value">
          {row.tokensSaved === null
            ? "—"
            : row.tokensSaved.toLocaleString()}
        </span>
      </div>
    );
  }

  return (
    <section
      className="optimize-minimal compression-overview"
      aria-labelledby="compression-overview-title"
    >
      <div className="optimize-card__head">
        <div className="optimize-card__title-row">
          <span className="optimize-card__title-icon" aria-hidden="true">
            <ChartBar weight="duotone" />
          </span>
          <div>
            <h2 id="compression-overview-title">Compression overview</h2>
            <p className="optimize-minimal__meta">
              Unified attribution across Headroom, RTK, cache, Repo Intelligence,
              and governed add-ons.
            </p>
          </div>
        </div>
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void refresh()}
          disabled={loading}
        >
          <ArrowClockwise weight="bold" size={12} aria-hidden="true" />
          {loading ? "Refreshing" : "Refresh"}
        </button>
      </div>

      {overview ? (
        <>
          <div className="compression-overview__totals" aria-live="polite">
            <strong>
              {overview.compressionTokensSaved === null
                ? "No measured compression total yet"
                : `${overview.compressionTokensSaved.toLocaleString()} tokens saved`}
            </strong>
            <span>
              {formatCompressionConfidence(overview.compressionConfidence)} ·{" "}
              {overview.scope === "today" ? "Today" : "Session"}
            </span>
          </div>
          {overview.sources.length > 0 ? (
            <div className="compression-overview__rows">
              {overview.sources.map(renderRow)}
            </div>
          ) : (
            <p className="optimize-minimal__meta">
              No compression families have recorded savings yet. Sources without
              data are omitted instead of showing zero measured savings.
            </p>
          )}
          <ul className="compression-overview__caveats">
            {overview.caveats.map((caveat) => (
              <li key={caveat}>{caveat}</li>
            ))}
          </ul>
        </>
      ) : (
        <p className="optimize-minimal__meta">
          {loading ? "Loading compression attribution..." : "No overview yet."}
        </p>
      )}
    </section>
  );
}
