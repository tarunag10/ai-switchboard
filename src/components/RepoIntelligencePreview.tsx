import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  buildAgentSessionPreparation,
  buildAgentSessionDisplayState,
  buildRepoAgentHandoffPayload,
  buildRepoAgentManifest,
  buildRepoIntelligenceSummary,
  estimateRepoIntelligenceSavings,
  formatRepoAgentHandoffMarkdown,
  formatAgentSessionPreparationJson,
  formatAgentSessionSelectedPackMarkdown,
  formatAgentSessionSummaryMarkdown,
  formatRepoAgentManifestJson,
  formatRepoContextPackMarkdown,
  formatSingleRepoContextPackMarkdown,
  getRepoIndexFreshness,
  normalizeRepoIndexRequest,
  repoAgentPackLabel,
  repoAgentHandoffProfiles,
  type AgentSessionTaskType,
  type RepoContextPack,
  type RepoAgentHandoffTarget,
  type RepoIntelligenceSummary,
  type RepoSavingsEstimate,
} from "../lib/repoIntelligence";

const repoIntelligencePreview = buildRepoIntelligenceSummary([
  { path: "src/App.tsx", bytes: 184_000 },
  { path: "src/lib/dashboardHelpers.ts", bytes: 28_000 },
  { path: "src/lib/repoIntelligence.ts", bytes: 7_400 },
  { path: "src-tauri/src/client_adapters.rs", bytes: 190_000 },
  { path: "src-tauri/src/lib.rs", bytes: 310_000 },
  { path: "src/lib/repoIntelligence.test.ts", bytes: 2_300 },
  { path: "src/lib/dashboardHelpers.test.ts", bytes: 18_000 },
  { path: "docs/repo-intelligence-plan.md", bytes: 4_800 },
  { path: "docs/beta-smoke-test.md", bytes: 9_200 },
  { path: "package.json", bytes: 1_900 },
  { path: "dist/assets/index.js", bytes: 767_000 },
]);

const primaryRepoAgentIds = new Set<RepoAgentHandoffTarget>([
  "claude",
  "codex",
]);

function repoAgentGroupLabel(
  profile: (typeof repoAgentHandoffProfiles)[number],
) {
  if (primaryRepoAgentIds.has(profile.id)) {
    return "Primary agents";
  }
  switch (profile.toolKind) {
    case "cli":
      return "CLI agents";
    case "editor":
      return "Editor agents";
    case "chat":
      return "Chat agents";
  }
}

const repoAgentHandoffGroups = repoAgentHandoffProfiles.reduce<
  Array<{
    label: string;
    profiles: typeof repoAgentHandoffProfiles;
  }>
>((groups, profile) => {
  const label = repoAgentGroupLabel(profile);
  const group = groups.find((candidate) => candidate.label === label);
  if (group) {
    group.profiles.push(profile);
  } else {
    groups.push({ label, profiles: [profile] });
  }
  return groups;
}, []);

export function RepoIntelligencePreview({
  headroomHealthy = false,
  onSummaryChange,
  rtkHealthy = false,
}: {
  headroomHealthy?: boolean;
  onSummaryChange?: (summary: RepoIntelligenceSummary) => void;
  rtkHealthy?: boolean;
}) {
  const [repoPath, setRepoPath] = useState("");
  const [selectedAgent, setSelectedAgent] =
    useState<RepoAgentHandoffTarget>("codex");
  const [selectedTaskType, setSelectedTaskType] =
    useState<AgentSessionTaskType>("verification");
  const [summary, setSummary] = useState<RepoIntelligenceSummary>(
    repoIntelligencePreview,
  );
  const [indexing, setIndexing] = useState(false);
  const [indexError, setIndexError] = useState<string | null>(null);
  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const isPreview = summary === repoIntelligencePreview;
  const hasRealIndex = !isPreview;
  const indexFreshness = getRepoIndexFreshness(summary);
  const indexStatusLabel = indexFreshness.label;
  const cacheStateLabel = summary.indexMetadata
    ? `${summary.indexMetadata.cacheState} cache · ${summary.indexMetadata.fileFingerprints.length.toLocaleString()} fingerprints · ${(summary.indexMetadata.skippedFiles?.length ?? summary.indexMetadata.skippedFileCount).toLocaleString()} skipped reasons · ${(summary.indexMetadata.graphInputs?.length ?? 0).toLocaleString()} graph inputs · ${summary.indexMetadata.parserVersion}`
    : null;
  const savingsEstimate = estimateRepoIntelligenceSavings(summary);
  const agentManifest = buildRepoAgentManifest(summary);
  const selectedAgentProfile =
    repoAgentHandoffProfiles.find((profile) => profile.id === selectedAgent) ??
    repoAgentHandoffProfiles[0];
  const providerRoutingSafe = primaryRepoAgentIds.has(selectedAgent);
  const sessionPreparation = buildAgentSessionPreparation(summary, {
    target: selectedAgentProfile.id,
    taskType: selectedTaskType,
    modeInputs: {
      headroomHealthy,
      rtkHealthy,
      providerRoutingSafe,
    },
  });
  const sessionDisplayState = buildAgentSessionDisplayState(
    sessionPreparation,
    hasRealIndex,
  );

  useEffect(() => {
    let cancelled = false;
    invoke<RepoIntelligenceSummary | null>(
      "get_latest_repo_intelligence_summary",
    )
      .then((latest) => {
        if (!cancelled && latest) {
          setSummary(latest);
          setRepoPath(latest.repoRoot ?? "");
          onSummaryChange?.(latest);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  async function runRepoIndex() {
    const request = normalizeRepoIndexRequest(repoPath);
    if (request.error) {
      setIndexError(request.error);
      return;
    }
    setIndexing(true);
    setIndexError(null);
    try {
      const next = await invoke<RepoIntelligenceSummary>(
        "build_repo_intelligence_summary",
        {
          repoPath: request.repoPath,
        },
      );
      setSummary(next);
      onSummaryChange?.(next);
    } catch (error) {
      setIndexError(
        error instanceof Error
          ? error.message
          : "Repo Intelligence could not index that folder.",
      );
    } finally {
      setIndexing(false);
    }
  }

  async function clearRepoIndex() {
    setIndexing(true);
    setIndexError(null);
    try {
      await invoke<boolean>("clear_repo_intelligence_summary");
      setSummary(repoIntelligencePreview);
      setRepoPath("");
      onSummaryChange?.(repoIntelligencePreview);
    } catch (error) {
      setIndexError(
        error instanceof Error
          ? error.message
          : "Repo Intelligence could not clear the saved index.",
      );
    } finally {
      setIndexing(false);
    }
  }

  async function copyContextPack() {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying real context.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        formatRepoContextPackMarkdown(summary),
      );
      setCopyNotice("Context pack copied.");
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select pack details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyAgentManifest() {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying a real manifest.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(formatRepoAgentManifestJson(summary));
      setCopyNotice("Agent manifest copied.");
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select manifest manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copySingleContextPack(pack: RepoContextPack) {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying this pack.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        formatSingleRepoContextPackMarkdown(summary, pack),
      );
      setCopyNotice(`${pack.title} copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select pack details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyAgentRecipePack(packId: string, label: string) {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying recipe packs.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    const pack = summary.packs.find((contextPack) => contextPack.id === packId);
    if (!pack) {
      setCopyNotice("Recipe pack unavailable. Re-index this repo.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        formatSingleRepoContextPackMarkdown(summary, pack),
      );
      setCopyNotice(`${label} copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select pack details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyAgentHandoff(
    target: RepoAgentHandoffTarget,
    label: string,
  ) {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying agent handoffs.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        formatRepoAgentHandoffMarkdown(summary, target),
      );
      setCopyNotice(`${label} handoff copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select handoff details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyAgentHandoffJson(
    target: RepoAgentHandoffTarget,
    label: string,
  ) {
    if (!hasRealIndex) {
      setCopyNotice("Index a repo before copying agent handoffs.");
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(
        JSON.stringify(buildRepoAgentHandoffPayload(summary, target), null, 2),
      );
      setCopyNotice(`${label} JSON handoff copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select JSON handoff manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyPreparedAgentSession() {
    if (!hasRealIndex || !sessionPreparation.handoffMarkdown) {
      setCopyNotice(sessionPreparation.copyDetail);
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(sessionPreparation.handoffMarkdown);
      setCopyNotice(`${sessionPreparation.target.label} session copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select session details manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyPreparedAgentSessionSummary() {
    const summaryMarkdown = formatAgentSessionSummaryMarkdown(sessionPreparation);
    if (!hasRealIndex || !summaryMarkdown) {
      setCopyNotice(sessionPreparation.copyDetail);
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(summaryMarkdown);
      setCopyNotice(`${sessionPreparation.target.label} summary copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select session summary manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyPreparedAgentSessionJson() {
    const json = formatAgentSessionPreparationJson(sessionPreparation);
    if (!hasRealIndex || !json) {
      setCopyNotice(sessionPreparation.copyDetail);
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(json);
      setCopyNotice(`${sessionPreparation.target.label} JSON copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select session JSON manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyPreparedAgentSessionPack() {
    const packMarkdown = formatAgentSessionSelectedPackMarkdown(
      summary,
      sessionPreparation,
    );
    if (!hasRealIndex || !packMarkdown) {
      setCopyNotice(sessionPreparation.copyDetail);
      window.setTimeout(() => setCopyNotice(null), 3000);
      return;
    }
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(packMarkdown);
      setCopyNotice(
        `${repoAgentPackLabel(sessionPreparation.packId)} copied.`,
      );
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Select session pack manually.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  return (
    <div
      className="repo-intelligence-preview"
      aria-label="Repo Intelligence context pack preview"
    >
      <div className="repo-intelligence-preview__topline">
        <span>{indexStatusLabel}</span>
        <strong>
          {summary.indexedFiles} indexed signals
          {summary.skippedFiles ? `, ${summary.skippedFiles} skipped` : ""}
        </strong>
      </div>
      <div className="repo-intelligence-preview__controls">
        <input
          aria-label="Repository folder path"
          className="repo-intelligence-preview__input"
          onChange={(event) => setRepoPath(event.target.value)}
          placeholder="~/Developer/my-repo"
          type="text"
          value={repoPath}
        />
        <button
          className="addon-card__action addon-card__action--primary"
          disabled={indexing}
          onClick={() => void runRepoIndex()}
          type="button"
        >
          {indexing ? "Indexing..." : "Index"}
        </button>
        {!isPreview ? (
          <>
            <button
              className="addon-card__action"
              disabled={indexing}
              onClick={() => void copyContextPack()}
              type="button"
            >
              Copy pack
            </button>
            <button
              className="addon-card__action"
              disabled={indexing}
              onClick={() => void copyAgentManifest()}
              type="button"
            >
              Copy agent manifest
            </button>
          </>
        ) : null}
        {!isPreview ? (
          <button
            className="addon-card__action"
            disabled={indexing}
            onClick={() => void clearRepoIndex()}
            type="button"
          >
            Clear
          </button>
        ) : null}
      </div>
      {summary.repoRoot ? (
        <p className="repo-intelligence-preview__path">{summary.repoRoot}</p>
      ) : null}
      {summary.indexedAt ? (
        <p className="repo-intelligence-preview__path">
          Indexed {new Date(summary.indexedAt).toLocaleString()}
        </p>
      ) : null}
      {hasRealIndex ? (
        <p className="repo-intelligence-preview__path">
          {indexFreshness.detail}
        </p>
      ) : null}
      {cacheStateLabel ? (
        <p className="repo-intelligence-preview__path">
          Index cache: {cacheStateLabel}
        </p>
      ) : null}
      {copyNotice ? (
        <p className="repo-intelligence-preview__path">{copyNotice}</p>
      ) : null}
      {indexError ? (
        <p className="install-progress__error">{indexError}</p>
      ) : null}
      <div
        className="repo-intelligence-session"
        aria-label="Start agent session"
      >
        <div className="repo-intelligence-session__heading">
          <div>
            <span>Start session</span>
            <strong>{sessionDisplayState.targetLabel}</strong>
          </div>
          <span
            className={`repo-intelligence-session__status repo-intelligence-session__status--${sessionDisplayState.copyStatus}`}
          >
            {sessionDisplayState.copyStatus}
          </span>
        </div>
        <div className="repo-intelligence-session__controls">
          <label>
            <span>Agent</span>
            <select
              value={selectedAgent}
              onChange={(event) =>
                setSelectedAgent(event.target.value as RepoAgentHandoffTarget)
              }
            >
              {repoAgentHandoffProfiles.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Task</span>
            <select
              value={selectedTaskType}
              onChange={(event) =>
                setSelectedTaskType(event.target.value as AgentSessionTaskType)
              }
            >
              <option value="implementation">Implementation</option>
              <option value="verification">Verification</option>
              <option value="handoff">Handoff</option>
              <option value="risk_review">Risk review</option>
              <option value="release_handoff">Release handoff</option>
            </select>
          </label>
          <button
            className="addon-card__action addon-card__action--primary"
            disabled={!sessionDisplayState.canCopyHandoff}
            onClick={() => void copyPreparedAgentSession()}
            type="button"
          >
            Copy full handoff
          </button>
          <button
            className="addon-card__action"
            disabled={!sessionDisplayState.canCopySummary}
            onClick={() => void copyPreparedAgentSessionSummary()}
            type="button"
          >
            Copy summary
          </button>
          <button
            className="addon-card__action"
            disabled={!sessionDisplayState.canCopySelectedPack}
            onClick={() => void copyPreparedAgentSessionPack()}
            type="button"
          >
            Copy selected pack
          </button>
          <button
            className="addon-card__action"
            disabled={!sessionDisplayState.canCopyJson}
            onClick={() => void copyPreparedAgentSessionJson()}
            type="button"
          >
            Copy JSON
          </button>
        </div>
        <div className="repo-intelligence-session__summary">
          <div>
            <span>Pack</span>
            <strong>{sessionDisplayState.packLabel}</strong>
          </div>
          <div>
            <span>Mode</span>
            <strong>
              {sessionDisplayState.modeLabel}
            </strong>
          </div>
          <div>
            <span>Freshness</span>
            <strong>{sessionDisplayState.freshnessLabel}</strong>
            <small>{sessionDisplayState.freshnessDetailLabel}</small>
          </div>
          <div>
            <span>Context</span>
            <strong>{sessionDisplayState.contextLabel}</strong>
          </div>
          <div>
            <span>Selected pack</span>
            <strong>{sessionDisplayState.selectedPackTokensLabel}</strong>
          </div>
          <div>
            <span>Avoided</span>
            <strong>{sessionDisplayState.tokensAvoidedLabel}</strong>
          </div>
          <div>
            <span>Skipped</span>
            <strong>{sessionDisplayState.skippedFilesLabel}</strong>
          </div>
          <div>
            <span>Secrets</span>
            <strong>{sessionDisplayState.secretExclusionLabel}</strong>
          </div>
          {sessionDisplayState.connectorReadinessLabel ? (
            <div>
              <span>Connector</span>
              <strong>{sessionDisplayState.connectorReadinessLabel}</strong>
              <small>{sessionDisplayState.connectorReadinessDetailLabel}</small>
            </div>
          ) : null}
        </div>
        {sessionDisplayState.sampleContextWarning ? (
          <p className="repo-intelligence-session__detail">
            {sessionDisplayState.sampleContextWarning}
          </p>
        ) : null}
        <p className="repo-intelligence-session__detail">
          {sessionDisplayState.copyDetail} Doctor still verifies runtime and
          connector health before any managed setup.
        </p>
        <div
          className="repo-intelligence-session__safety"
          aria-label="Agent session copy safety proof"
        >
          <span>
            {sessionPreparation.copySafety.hasRealIndex
              ? "Real index"
              : "Sample blocked"}
          </span>
          <span>
            {sessionPreparation.copySafety.allowsCopy
              ? "Copy allowed"
              : "Copy disabled"}
          </span>
          <span>
            {sessionPreparation.copySafety.excludesSecretLikePaths
              ? "Secrets excluded"
              : "Secrets unchecked"}
          </span>
          <span>
            {sessionPreparation.copySafety.skippedFileCount.toLocaleString()}{" "}
            skipped
          </span>
        </div>
        <p className="repo-intelligence-session__detail">
          {sessionPreparation.recommendedModeReason}
        </p>
      </div>
      <div
        className="repo-intelligence-savings"
        aria-label="Repo Intelligence savings calculator"
      >
        <div>
          <span>Full scan</span>
          <strong>{savingsEstimate.fullScanTokens.toLocaleString()}</strong>
          <em>tokens estimated</em>
        </div>
        <div>
          <span>Best pack saved</span>
          <strong>
            {savingsEstimate.bestPackTokensAvoided.toLocaleString()}
          </strong>
          <em>
            {savingsEstimate.bestPack?.title ?? "Context pack"} ·{" "}
            {savingsEstimate.bestPackSavingsPct.toFixed(1)}%
          </em>
        </div>
        <div>
          <span>All packs saved</span>
          <strong>
            {savingsEstimate.allPacksTokensAvoided.toLocaleString()}
          </strong>
          <em>{savingsEstimate.allPacksSavingsPct.toFixed(1)}% vs full scan</em>
        </div>
      </div>
      {summary.graph ? (
        <div
          className="repo-intelligence-graph"
          aria-label="Repo Intelligence graph summary"
        >
          <div>
            <span>Top directories</span>
            <strong>
              {summary.graph.topDirectories
                .slice(0, 3)
                .map((node) => `${node.label} (${node.count})`)
                .join(", ") || "None"}
            </strong>
          </div>
          <div>
            <span>Languages</span>
            <strong>
              {summary.graph.topLanguages
                .slice(0, 3)
                .map((node) => node.label)
                .join(", ") || "Unknown"}
            </strong>
          </div>
          <div>
            <span>Entrypoints</span>
            <strong>{summary.graph.entrypoints.length}</strong>
          </div>
          <div>
            <span>Likely tests</span>
            <strong>{summary.graph.likelyTests.length}</strong>
          </div>
          <div className="repo-intelligence-graph__wide">
            <span>Test relationships</span>
            <strong>{summary.graph.testRelationships?.length ?? 0}</strong>
            <em>
              {summary.graph.testRelationships
                ?.slice(0, 2)
                .map((edge) => `${edge.testPath} -> ${edge.sourcePath}`)
                .join(", ") || "No source/test links yet"}
            </em>
          </div>
          <div>
            <span>Dependency hubs</span>
            <strong>{summary.graph.dependencyHubs?.length ?? 0}</strong>
            <em>
              {summary.graph.dependencyHubs
                ?.slice(0, 2)
                .map((file) => file.path)
                .join(", ") || "No hub files yet"}
            </em>
          </div>
          <div>
            <span>Import edges</span>
            <strong>{summary.graph.importEdges?.length ?? 0}</strong>
            <em>
              {summary.graph.importEdges
                ?.slice(0, 2)
                .map((edge) => `${edge.from} -> ${edge.to}`)
                .join(", ") || "No path links yet"}
            </em>
          </div>
          <div>
            <span>Reverse hubs</span>
            <strong>{summary.graph.reverseDependencyHubs?.length ?? 0}</strong>
            <em>
              {summary.graph.reverseDependencyHubs
                ?.slice(0, 2)
                .map((node) => `${node.label} (${node.count})`)
                .join(", ") || "No reverse hubs yet"}
            </em>
          </div>
          <div>
            <span>Symbols</span>
            <strong>{summary.graph.symbols?.length ?? 0}</strong>
            <em>
              {summary.graph.symbols
                ?.slice(0, 3)
                .map((symbol) => `${symbol.name} (${symbol.kind})`)
                .join(", ") || "No symbols yet"}
            </em>
          </div>
          <div className="repo-intelligence-graph__wide">
            <span>Agent graph signal</span>
            <strong>
              {`${summary.graph.dependencyHubs?.length ?? 0} hubs · ${
                summary.graph.importEdges?.length ?? 0
              } edges · ${summary.graph.reverseDependencyHubs?.length ?? 0} reverse hubs · ${
                summary.graph.symbols?.length ?? 0
              } symbols`}
            </strong>
            <em>Copied into manifests and handoffs without file contents.</em>
          </div>
        </div>
      ) : null}
      <div className="repo-intelligence-preview__grid">
        {summary.packs.map((pack) => (
          <article className="repo-intelligence-pack" key={pack.id}>
            <div className="repo-intelligence-pack__heading">
              <span>{pack.title}</span>
              <strong>{pack.savingsVsFullScanPct.toFixed(1)}%</strong>
            </div>
            <p>{pack.purpose}</p>
            <span className="repo-intelligence-pack__meta">
              {pack.files.length} files &middot; about{" "}
              {pack.estimatedTokens.toLocaleString()} tokens
            </span>
            {!isPreview ? (
              <button
                className="repo-intelligence-pack__copy"
                onClick={() => void copySingleContextPack(pack)}
                type="button"
              >
                Copy this pack
              </button>
            ) : null}
          </article>
        ))}
      </div>

      <div
        className="repo-intelligence-handoffs"
        aria-label="Agent-specific handoffs"
      >
        <div className="repo-intelligence-recipes__heading">
          <span>Agent handoffs</span>
          <strong>Ready to paste</strong>
        </div>
        <div className="repo-intelligence-handoffs__grid">
          {repoAgentHandoffGroups.map((group) => (
            <section
              className="repo-intelligence-handoff-group"
              key={group.label}
            >
              <div className="repo-intelligence-handoff-group__label">
                <span>{group.label}</span>
              </div>
              <div className="repo-intelligence-handoff-group__buttons">
                {group.profiles.map((profile) => (
                  <div className="repo-intelligence-handoff" key={profile.id}>
                    <div>
                      <strong>{profile.label}</strong>
                      <span>{repoAgentPackLabel(profile.defaultPackId)}</span>
                      <em>{profile.guidance}</em>
                    </div>
                    <div className="repo-intelligence-handoff__actions">
                      <button
                        disabled={isPreview}
                        onClick={() =>
                          void copyAgentHandoff(profile.id, profile.label)
                        }
                        type="button"
                      >
                        Markdown
                      </button>
                      <button
                        disabled={isPreview}
                        onClick={() =>
                          void copyAgentHandoffJson(profile.id, profile.label)
                        }
                        type="button"
                      >
                        JSON
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          ))}
        </div>
      </div>

      <div
        className="repo-intelligence-recipes"
        aria-label="Agent handoff recipes"
      >
        <div className="repo-intelligence-recipes__heading">
          <span>Agent recipes</span>
          <strong>Read-only handoff</strong>
        </div>
        <div className="repo-intelligence-recipes__list">
          {agentManifest.agentRecipes.map((recipe) => (
            <article className="repo-intelligence-recipe" key={recipe.id}>
              <div>
                <strong>{recipe.label}</strong>
                <span>{recipe.tools.join(", ")}</span>
              </div>
              <p>{recipe.instruction}</p>
              {!isPreview ? (
                <button
                  className="repo-intelligence-pack__copy"
                  onClick={() =>
                    void copyAgentRecipePack(recipe.packIds[0], recipe.label)
                  }
                  type="button"
                >
                  Copy recipe pack
                </button>
              ) : null}
            </article>
          ))}
        </div>
      </div>
    </div>
  );
}
