import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  buildAgentSessionPreparation,
  buildAgentSessionDisplayState,
  buildRepoAgentHandoffPayload,
  buildRepoAgentManifest,
  buildRepoIntelligenceSummary,
  buildRepoPackCompressionConfig,
  estimateRepoIntelligenceSavings,
  estimateRepoPackCompressionSavings,
  formatRepoAgentHandoffMarkdown,
  formatAgentSessionPreparationJson,
  formatAgentSessionSelectedPackMarkdown,
  formatAgentSessionSummaryMarkdown,
  formatRepoAgentManifestJson,
  formatRepoContextPackMarkdown,
  formatSingleRepoContextPackMarkdown,
  getRepoIndexFreshness,
  repoPackCompressionPreferenceEvent,
  repoPackCompressionPreferenceKey,
  normalizeRepoIndexRequest,
  repoAgentPackLabel,
  repoAgentHandoffProfiles,
  type AgentSessionTaskType,
  type RepoContextPack,
  type RepoAgentHandoffTarget,
  type RepoGraphEdge,
  type RepoIntelligenceSummary,
  type RepoPackCompressionConfig,
  type RepoPackCompressionMode,
  type RepoSavingsEstimate,
} from "../lib/repoIntelligence";
import { canActivateChonkifyRepoPack } from "../lib/chonkifyPromotionGate";
import {
  loadAuthoritativeRepoPackCompressionPreference,
  saveNativeRepoPackCompressionPreference,
} from "../lib/repoPackCompressionPreference";

type NativeIndexFreshness = {
  label: string;
  detail: string;
  status: string;
  apiAvailable: boolean;
  graphAvailable: boolean;
  indexHealth: string;
  parserHealth: string;
  indexedFileCount?: number;
  skippedFileCount?: number;
};

type NativeSymbolSearch = {
  symbols: Array<{ name: string; kind: string; file: string; line: number }>;
};

type NativeDependentsSearch = {
  edges: Array<{ from: string; to: string; kind: string; reason: string }>;
};

export const repoIntelligencePreview = buildRepoIntelligenceSummary([
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
  const [packCompressionMode, setPackCompressionMode] =
    useState<RepoPackCompressionMode>("off");
  const [compressionError, setCompressionError] = useState<string | null>(null);
  const [summary, setSummary] = useState<RepoIntelligenceSummary>(
    repoIntelligencePreview,
  );
  const [indexing, setIndexing] = useState(false);
  const [indexError, setIndexError] = useState<string | null>(null);
  const [savedIndexError, setSavedIndexError] = useState<string | null>(null);
  const [savedIndexLoading, setSavedIndexLoading] = useState(true);
  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const [showVerificationDetails, setShowVerificationDetails] = useState(false);
  const [showModeReasoning, setShowModeReasoning] = useState(false);
  const [showGraphDiagnostics, setShowGraphDiagnostics] = useState(false);
  const [relationshipFilter, setRelationshipFilter] = useState<
    "all" | "tests" | "imports" | "reverse"
  >("all");
  const [relationshipQuery, setRelationshipQuery] = useState("");
  const [nativeFreshness, setNativeFreshness] = useState<NativeIndexFreshness | null>(null);
  const [nativeDiagnosticsError, setNativeDiagnosticsError] = useState<string | null>(null);
  const [symbolQuery, setSymbolQuery] = useState("");
  const [symbolResults, setSymbolResults] = useState<NativeSymbolSearch | null>(null);
  const [dependentTarget, setDependentTarget] = useState("");
  const [dependentResults, setDependentResults] = useState<NativeDependentsSearch | null>(null);
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
  const packCompressionConfig = buildRepoPackCompressionConfig(packCompressionMode);
  const chonkifyEligible = canActivateChonkifyRepoPack();

  useEffect(() => {
    let active = true;
    const refreshCompressionPreference = () => void loadAuthoritativeRepoPackCompressionPreference()
      .then((preference) => {
        if (!active) return;
        const effectiveMode: RepoPackCompressionMode =
          preference?.effectiveMode === "chonkify" ? "chonkify" : "off";
        if (active) setPackCompressionMode(effectiveMode);
      })
      .catch(() => {
        if (active) setPackCompressionMode("off");
      });
    refreshCompressionPreference();
    window.addEventListener(repoPackCompressionPreferenceEvent, refreshCompressionPreference);
    return () => {
      active = false;
      window.removeEventListener(repoPackCompressionPreferenceEvent, refreshCompressionPreference);
    };
  }, [chonkifyEligible]);
  const verificationDetailsId = "repo-intelligence-verification-details";
  const modeReasoningId = "repo-intelligence-mode-reasoning";
  const graphDiagnosticsId = "repo-intelligence-graph-diagnostics";
  const relationshipExplorerId = "repo-intelligence-relationship-explorer";

  const relationshipRows = summary.graph
    ? [
        ...(summary.graph.testRelationships ?? []).map((relationship) => ({
          kind: "tests" as const,
          label: "Test",
          from: relationship.testPath,
          to: relationship.sourcePath,
          reason: relationship.reason,
        })),
        ...(summary.graph.importEdges ?? []).map((edge: RepoGraphEdge) => ({
          kind: "imports" as const,
          label: "Import",
          from: edge.from,
          to: edge.to,
          reason: edge.reason,
        })),
        ...(summary.graph.reverseDependencyHubs ?? []).map((hub) => ({
          kind: "reverse" as const,
          label: "Reverse hub",
          from: hub.label,
          to: `${hub.count.toLocaleString()} dependents`,
          reason: "High fan-in dependency surface",
        })),
      ]
    : [];
  const normalizedRelationshipQuery = relationshipQuery.trim().toLowerCase();
  const visibleRelationshipRows = relationshipRows
    .filter(
      (row) => relationshipFilter === "all" || row.kind === relationshipFilter,
    )
    .filter((row) => {
      if (!normalizedRelationshipQuery) return true;
      return [row.from, row.to, row.reason, row.label].some((value) =>
        value.toLowerCase().includes(normalizedRelationshipQuery),
      );
    })
    .slice(0, 40);

  async function loadSavedRepoIndex() {
    setSavedIndexLoading(true);
    setSavedIndexError(null);
    try {
      const latest = await invoke<RepoIntelligenceSummary | null>(
        "get_latest_repo_intelligence_summary",
      );
      if (latest) {
        setSummary(latest);
        setRepoPath(latest.repoRoot ?? "");
        onSummaryChange?.(latest);
        await refreshNativeDiagnostics();
      }
    } catch (error) {
      setSavedIndexError(
        error instanceof Error
          ? error.message
          : "Saved Repo Intelligence index could not be loaded.",
      );
    } finally {
      setSavedIndexLoading(false);
    }
  }

  async function refreshNativeDiagnostics() {
    if (!hasRealIndex) return;
    setNativeDiagnosticsError(null);
    try {
      const freshness = await invoke<NativeIndexFreshness>("get_index_freshness");
      setNativeFreshness(freshness);
    } catch (error) {
      setNativeDiagnosticsError(error instanceof Error ? error.message : "Native index diagnostics unavailable.");
    }
  }

  useEffect(() => {
    void loadSavedRepoIndex();
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
      await refreshNativeDiagnostics();
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

  async function chooseRepoFolder() {
    setIndexError(null);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Choose repository folder",
      });
      if (typeof selected === "string") {
        setRepoPath(selected);
      }
    } catch (error) {
      setIndexError(
        error instanceof Error
          ? error.message
          : "Repo Intelligence could not choose that folder.",
      );
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
        formatRepoContextPackMarkdown(summary, packCompressionConfig),
      );
      setCopyNotice("Context pack copied.");
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Pack details remain visible below.");
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
      setCopyNotice("Copy failed. Manifest details remain visible below.");
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
        formatSingleRepoContextPackMarkdown(summary, pack, packCompressionConfig),
      );
      setCopyNotice(`${pack.title} copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Pack details remain visible below.");
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
        formatSingleRepoContextPackMarkdown(summary, pack, packCompressionConfig),
      );
      setCopyNotice(`${label} copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Pack details remain visible below.");
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
        formatRepoAgentHandoffMarkdown(summary, target, undefined, packCompressionConfig),
      );
      setCopyNotice(`${label} handoff copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. Handoff details remain visible below.");
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
        JSON.stringify(buildRepoAgentHandoffPayload(summary, target, undefined, packCompressionConfig), null, 2),
      );
      setCopyNotice(`${label} JSON handoff copied.`);
      window.setTimeout(() => setCopyNotice(null), 2000);
    } catch {
      setCopyNotice("Copy failed. JSON handoff remains visible below.");
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
      setCopyNotice("Copy failed. Session details remain visible below.");
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
      setCopyNotice("Copy failed. Session summary remains visible below.");
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
      setCopyNotice("Copy failed. Session JSON remains visible below.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  async function copyPreparedAgentSessionPack() {
    const packMarkdown = formatAgentSessionSelectedPackMarkdown(
      summary,
      sessionPreparation,
      packCompressionConfig,
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
      setCopyNotice("Copy failed. Session pack remains visible below.");
      window.setTimeout(() => setCopyNotice(null), 3000);
    }
  }

  return (
    <div
      className="repo-intelligence-preview"
      aria-label="Repo Intelligence context pack preview"
    >
      <div className="repo-intelligence-preview__topline" role="status" aria-live="polite" aria-busy={indexing}>
        <span>{savedIndexLoading ? "Loading saved index…" : indexStatusLabel}</span>
        <strong>
          {summary.indexedFiles} indexed signals
          {summary.skippedFiles ? `, ${summary.skippedFiles} skipped` : ""}
        </strong>
      </div>
      {nativeFreshness ? (
        <div className="repo-intelligence-preview__topline" role="status">
          <span>Native index: {nativeFreshness.label}</span>
          <strong>{nativeFreshness.indexHealth} · parser {nativeFreshness.parserHealth}</strong>
        </div>
      ) : null}
      {nativeDiagnosticsError ? <p role="status">Native diagnostics: {nativeDiagnosticsError}</p> : null}
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
          className="addon-card__action"
          disabled={indexing}
          onClick={() => void chooseRepoFolder()}
          type="button"
        >
          Choose folder
        </button>
        <button
          className="addon-card__action addon-card__action--primary"
          disabled={indexing}
          onClick={() => void runRepoIndex()}
          type="button"
        >
          {indexing ? "Indexing..." : "Index"}
        </button>
        <label className="repo-intelligence-preview__compression-control">
          Pack compression
          <select
            aria-label="Repo pack compression mode"
            disabled={indexing}
            onChange={(event) => {
              const mode = event.target.value as RepoPackCompressionMode;
              setCompressionError(null);
              if (mode === "off" || chonkifyEligible) setPackCompressionMode(mode);
              void saveNativeRepoPackCompressionPreference(mode)
                .then((preference) => {
                  setPackCompressionMode(preference.effectiveMode === "chonkify" ? "chonkify" : "off");
                  window.localStorage.removeItem(repoPackCompressionPreferenceKey);
                })
                .catch((error: unknown) => {
                  setPackCompressionMode("off");
                  setCompressionError(error instanceof Error ? error.message : String(error));
                });
            }}
            value={packCompressionMode}
          >
            <option value="off">Native deterministic (recommended)</option>
            <option value="chonkify">
              Chonkify {chonkifyEligible ? "(repo-pack eligible)" : "(blocked pending license)"}
            </option>
          </select>
        </label>
        {compressionError ? <p role="alert">Pack compression: {compressionError}</p> : null}
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
      <p className="repo-intelligence-preview__path" role="note">
        {packCompressionMode === "chonkify"
          ? chonkifyEligible
            ? "Chonkify is eligible for read-only Repo Intelligence packs. Copy dialogs show native vs chonkify token estimates; savings remain labelled estimated."
            : "Chonkify is selected for evidence preview only. Current license metadata is NOASSERTION, so native deterministic packs remain unchanged and savings stay unclaimed."
          : "Native deterministic packs are the default. Chonkify can be enabled explicitly for eligible read-only packs; source spans are retained and savings are labelled estimated."}
      </p>
      {copyNotice ? (
        <p className="repo-intelligence-preview__path" role="status" aria-live="polite">{copyNotice}</p>
      ) : null}
      {savedIndexError ? (
        <div className="repo-intelligence-preview__load-error" role="alert" aria-live="assertive">
          <p>Could not load the saved Repo Intelligence index: {savedIndexError}</p>
          <button
            className="addon-card__action"
            disabled={savedIndexLoading || indexing}
            onClick={() => void loadSavedRepoIndex()}
            type="button"
          >
            Retry saved index
          </button>
        </div>
      ) : null}
      {indexError ? (
        <p className="install-progress__error" role="alert">{indexError}</p>
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
        <div className="repo-intelligence-disclosure">
          <button
            aria-controls={verificationDetailsId}
            aria-expanded={showVerificationDetails}
            className="repo-intelligence-disclosure__button"
            onClick={() => setShowVerificationDetails((open) => !open)}
            type="button"
          >
            {showVerificationDetails ? "Hide details" : "Details"}
          </button>
          {showVerificationDetails ? (
            <div
              className="repo-intelligence-disclosure__panel"
              id={verificationDetailsId}
            >
              <p className="repo-intelligence-session__detail">
                {sessionDisplayState.copyDetail} Doctor still verifies runtime
                and connector health before any app-managed setup.
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
            </div>
          ) : null}
        </div>
        <div className="repo-intelligence-disclosure">
          <button
            aria-controls={modeReasoningId}
            aria-expanded={showModeReasoning}
            className="repo-intelligence-disclosure__button"
            onClick={() => setShowModeReasoning((open) => !open)}
            type="button"
          >
            {showModeReasoning ? "Hide reasoning" : "Learn more"}
          </button>
          {showModeReasoning ? (
            <p
              className="repo-intelligence-session__detail repo-intelligence-disclosure__panel"
              id={modeReasoningId}
            >
              {sessionPreparation.recommendedModeReason}
            </p>
          ) : null}
        </div>
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
        <div>
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
          </div>
          <section
            className="repo-intelligence-relationship-explorer"
            aria-labelledby={relationshipExplorerId}
          >
            <div className="repo-intelligence-relationship-explorer__heading">
              <div>
                <span className="repo-intelligence-section-label">Relationship explorer</span>
                <h3 id={relationshipExplorerId}>Read-only repository connections</h3>
              </div>
              <span className="repo-intelligence-relationship-explorer__count">
                {relationshipRows.length.toLocaleString()} indexed
              </span>
            </div>
            <p className="repo-intelligence-relationship-explorer__description">
              Explore test coverage, imports, and reverse dependency hubs without exposing file contents.
            </p>
            <div className="repo-intelligence-preview__controls repo-intelligence-relationship-explorer__controls">
              <label>
                <span>Filter</span>
                <select
                  aria-label="Repo Intelligence relationship filter"
                  onChange={(event) =>
                    setRelationshipFilter(
                      event.target.value as "all" | "tests" | "imports" | "reverse",
                    )
                  }
                  value={relationshipFilter}
                >
                  <option value="all">All relationships</option>
                  <option value="tests">Tests</option>
                  <option value="imports">Imports</option>
                  <option value="reverse">Reverse hubs</option>
                </select>
              </label>
              <label>
                <span>Search</span>
                <input
                  aria-label="Search Repo Intelligence relationships"
                  onChange={(event) => setRelationshipQuery(event.target.value)}
                  placeholder="Path or reason"
                  value={relationshipQuery}
                />
              </label>
            </div>
            {relationshipRows.length === 0 ? (
              <p className="repo-intelligence-relationship-explorer__empty">
                {hasRealIndex
                  ? "No relationships were indexed for this repository. Re-index after adding imports or tests."
                  : "Index a repository to activate relationship exploration. The sample graph remains read-only."}
              </p>
            ) : visibleRelationshipRows.length === 0 ? (
              <p className="repo-intelligence-relationship-explorer__empty">
                No relationships match this filter.
              </p>
            ) : (
              <div className="repo-intelligence-relationship-table" role="table" aria-label="Repo Intelligence relationships">
                <div className="repo-intelligence-relationship-table__row repo-intelligence-relationship-table__row--header" role="row">
                  <span role="columnheader">Type</span>
                  <span role="columnheader">From</span>
                  <span role="columnheader">To</span>
                  <span role="columnheader">Evidence</span>
                </div>
                {visibleRelationshipRows.map((row, index) => (
                  <div className="repo-intelligence-relationship-table__row" key={`${row.kind}-${row.from}-${row.to}-${index}`} role="row">
                    <span role="cell" className="repo-intelligence-relationship-table__kind">{row.label}</span>
                    <code role="cell" title={row.from}>{row.from}</code>
                    <code role="cell" title={row.to}>{row.to}</code>
                    <span role="cell" title={row.reason}>{row.reason}</span>
                  </div>
                ))}
              </div>
            )}
            {relationshipRows.length > visibleRelationshipRows.length && visibleRelationshipRows.length > 0 ? (
              <span className="repo-intelligence-relationship-explorer__meta">
                Showing {visibleRelationshipRows.length.toLocaleString()} of {relationshipRows.length.toLocaleString()} relationships.
              </span>
            ) : null}
          </section>
          <div className="repo-intelligence-disclosure repo-intelligence-disclosure--graph">
            <button
              aria-controls={graphDiagnosticsId}
              aria-expanded={showGraphDiagnostics}
              className="repo-intelligence-disclosure__button"
              onClick={() => setShowGraphDiagnostics((open) => !open)}
              type="button"
            >
              {showGraphDiagnostics ? "Hide diagnostics" : "Learn more"}
            </button>
            {showGraphDiagnostics ? (
              <div
                className="repo-intelligence-graph repo-intelligence-graph--diagnostics repo-intelligence-disclosure__panel"
                id={graphDiagnosticsId}
                aria-label="Repo Intelligence graph diagnostics"
              >
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
                <div className="repo-intelligence-graph__wide">
                  <span>Native read-only queries</span>
                  <div className="repo-intelligence-preview__controls">
                    <input
                      aria-label="Repo Intelligence symbol query"
                      onChange={(event) => setSymbolQuery(event.target.value)}
                      placeholder="Search symbols"
                      value={symbolQuery}
                    />
                    <button
                      className="addon-card__action"
                      disabled={!hasRealIndex || !symbolQuery.trim()}
                      onClick={() => void (async () => {
                        try {
                          setNativeDiagnosticsError(null);
                          setSymbolResults(await invoke<NativeSymbolSearch>("search_repo_intelligence_symbols", { query: symbolQuery.trim(), limit: 20 }));
                        } catch (error) {
                          setNativeDiagnosticsError(error instanceof Error ? error.message : "Symbol search unavailable.");
                        }
                      })()}
                      type="button"
                    >
                      Search symbols
                    </button>
                  </div>
                  {symbolResults ? <em>{symbolResults.symbols.map((symbol) => `${symbol.name} (${symbol.kind})`).join(", ") || "No symbols found"}</em> : null}
                  <div className="repo-intelligence-preview__controls">
                    <input
                      aria-label="Repo Intelligence dependent target"
                      onChange={(event) => setDependentTarget(event.target.value)}
                      placeholder="Target path or symbol"
                      value={dependentTarget}
                    />
                    <button
                      className="addon-card__action"
                      disabled={!hasRealIndex || !dependentTarget.trim()}
                      onClick={() => void (async () => {
                        try {
                          setNativeDiagnosticsError(null);
                          setDependentResults(await invoke<NativeDependentsSearch>("get_repo_intelligence_dependents", { target: dependentTarget.trim(), limit: 20 }));
                        } catch (error) {
                          setNativeDiagnosticsError(error instanceof Error ? error.message : "Dependent search unavailable.");
                        }
                      })()}
                      type="button"
                    >
                      Find dependents
                    </button>
                  </div>
                  {dependentResults ? <em>{dependentResults.edges.map((edge) => `${edge.from} -> ${edge.to}`).join(", ") || "No dependents found"}</em> : null}
                  <button className="addon-card__action" onClick={() => void refreshNativeDiagnostics()} type="button">Refresh native freshness</button>
                </div>
              </div>
            ) : null}
          </div>
        </div>
      ) : null}
      <div className="repo-intelligence-preview__grid">
        {summary.packs.map((pack) => {
          const compressionEstimate =
            packCompressionMode === "chonkify"
              ? estimateRepoPackCompressionSavings(pack, packCompressionConfig)
              : null;
          return (
          <article className="repo-intelligence-pack" key={pack.id}>
            <div className="repo-intelligence-pack__heading">
              <span>{pack.title}</span>
              <strong>{pack.savingsVsFullScanPct.toFixed(1)}%</strong>
            </div>
            <p>{pack.purpose}</p>
            <span className="repo-intelligence-pack__meta">
              {pack.files.length} files &middot; about{" "}
              {pack.estimatedTokens.toLocaleString()} tokens
              {compressionEstimate && !compressionEstimate.blocked && compressionEstimate.compressedTokens !== null
                ? ` · chonkify ~${compressionEstimate.compressedTokens.toLocaleString()} tokens`
                : ""}
              {compressionEstimate?.blocked
                ? " · chonkify blocked"
                : ""}
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
        );
        })}
      </div>

      <section
        className="repo-intelligence-handoffs"
        aria-labelledby="repo-intelligence-handoffs-title"
      >
        <div className="repo-intelligence-recipes__heading">
          <h3 id="repo-intelligence-handoffs-title">Agent handoffs</h3>
          <strong>{isPreview ? "Index a repo to enable copying" : "Ready to paste"}</strong>
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
      </section>

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
                  Copy agent-ready pack
                </button>
              ) : null}
            </article>
          ))}
        </div>
      </div>
    </div>
  );
}
