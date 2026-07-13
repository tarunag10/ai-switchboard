import {
  ArrowClockwise,
  CheckCircle,
  Copy,
  FileCode,
  FolderOpen,
  GitBranch,
  Graph,
  MagnifyingGlass,
  WarningCircle,
} from "@phosphor-icons/react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import repoMapSnapshot from "../../docs/repo-map/repo-map.json";
import {
  DEFAULT_REPO_MAP_REPO_PATH,
  normalizeRepoMapError,
  readRepoMapHistory,
  repoMapTauriAdapter,
  type RepoMapGenerationResponse,
  type RepoMapHistoryItem,
  type RepoMapPreflightResponse,
  type RepoMapSnapshot,
  upsertRepoMapHistory,
  writeRepoMapHistory,
} from "../lib/repoMapJob";
import {
  buildRepoMapProgressSteps,
  buildRepoMapProgressSummary,
} from "../lib/repoMapProgress";
import { hasTauriEventRuntime } from "../lib/tauriRuntime";

interface RepoMapViewProps {
  onOpenRepoIntelligence: () => void;
  onOpenDoctor: () => void;
}

interface RepoMapGenerationEvent {
  repoPath: string;
  phase: "started" | "running" | "finished" | "failed";
  stream: "status" | "stdout" | "stderr";
  message: string;
  toolId?: string;
  toolStatus?: "ok" | "warning" | "not-run";
  progressPercent?: number;
  completedTools?: number;
  totalTools?: number;
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

function formatGeneratedAt(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

function formatElapsedSeconds(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  if (minutes === 0) return `${remainder}s`;
  return `${minutes}m ${remainder.toString().padStart(2, "0")}s`;
}

function compactLogTail(value: string): string {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : "No output captured.";
}

function statusTone(map: RepoMapSnapshot): "healthy" | "warning" {
  return map.tauri.missingRustCommand.length === 0 &&
    map.tauri.missingHandler.length === 0 &&
    map.tools.madge.cycles === 0
    ? "healthy"
    : "warning";
}

export function RepoMapView({
  onOpenDoctor,
  onOpenRepoIntelligence,
}: RepoMapViewProps) {
  const [repoMap, setRepoMap] = useState<RepoMapSnapshot>(
    repoMapSnapshot as unknown as RepoMapSnapshot,
  );
  const [repoPath, setRepoPath] = useState(DEFAULT_REPO_MAP_REPO_PATH);
  const [generation, setGeneration] = useState<RepoMapGenerationResponse | null>(null);
  const [preflight, setPreflight] = useState<RepoMapPreflightResponse | null>(null);
  const [preflightBusy, setPreflightBusy] = useState(false);
  const [pickerBusy, setPickerBusy] = useState(false);
  const [generateBusy, setGenerateBusy] = useState(false);
  const [generationStartedAt, setGenerationStartedAt] = useState<number | null>(null);
  const [generationElapsedSeconds, setGenerationElapsedSeconds] = useState(0);
  const [liveGenerationEvents, setLiveGenerationEvents] = useState<
    RepoMapGenerationEvent[]
  >([]);
  const [liveToolRuns, setLiveToolRuns] = useState<
    Record<string, { status: "ok" | "warning" | "not-run"; detail: string; remediation: null }>
  >({});
  const [liveProgressEvidence, setLiveProgressEvidence] = useState<{
    percent: number;
    completed: number;
    total: number;
    currentToolId: string | null;
  } | null>(null);
  const [generateError, setGenerateError] = useState<string | null>(null);
  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const [openNotice, setOpenNotice] = useState<string | null>(null);
  const [history, setHistory] = useState<RepoMapHistoryItem[]>(() =>
    readRepoMapHistory(window.localStorage),
  );
  const [showToolInstallDetails, setShowToolInstallDetails] = useState(false);
  const [showGraphDiagnostics, setShowGraphDiagnostics] = useState(false);
  const [showRunOutput, setShowRunOutput] = useState(false);

  const tone = statusTone(repoMap);
  const graphifyReady = repoMap.tools.graphify.nodeCount > 0;
  const wiringHealthy =
    repoMap.tauri.missingRustCommand.length === 0 &&
    repoMap.tauri.missingHandler.length === 0;
  const frontendHealthy = repoMap.tools.madge.cycles === 0;
  const tokenSavings = repoMap.tokenSavings;
  const selectedRepo = generation?.repoPath ?? repoPath;

  useEffect(() => {
    void runPreflight(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!generateBusy || generationStartedAt === null) return undefined;
    const tick = () => {
      setGenerationElapsedSeconds(
        Math.max(0, Math.floor((Date.now() - generationStartedAt) / 1000)),
      );
    };
    tick();
    const interval = window.setInterval(tick, 1000);
    return () => window.clearInterval(interval);
  }, [generateBusy, generationStartedAt]);

  useEffect(() => {
    if (!hasTauriEventRuntime()) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<RepoMapGenerationEvent>(
      "repo_map_generation_event",
      (event) => {
        if (disposed) return;
        const payload = event.payload;
        if (payload.repoPath !== repoPath && payload.repoPath !== generation?.repoPath) {
          return;
        }
        setLiveGenerationEvents((events) => [...events, payload].slice(-80));
        if (payload.toolId && payload.toolStatus) {
          setLiveToolRuns((runs) => ({
            ...runs,
            [payload.toolId ?? ""]: {
              status: payload.toolStatus ?? "not-run",
              detail: payload.message,
              remediation: null,
            },
          }));
          setLiveProgressEvidence({
            percent: Math.max(0, Math.min(100, payload.progressPercent ?? 0)),
            completed: payload.completedTools ?? 0,
            total: payload.totalTools ?? 0,
            currentToolId: payload.toolId,
          });
        }
        if (payload.phase === "started" || payload.phase === "running") {
          setShowRunOutput(true);
        }
      },
    ).then((cleanup) => {
      if (disposed) {
        cleanup();
      } else {
        unlisten = cleanup;
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [generation?.repoPath, repoPath]);

  const upsertHistory = (result: RepoMapGenerationResponse) => {
    const next = upsertRepoMapHistory(history, result);
    setHistory(next);
    writeRepoMapHistory(window.localStorage, next);
  };

  const runPreflight = async (showNotice = true) => {
    setPreflightBusy(true);
    setGenerateError(null);
    try {
      const result = await repoMapTauriAdapter.preflight(repoPath.trim() || null);
      setPreflight(result);
      if (showNotice) setCopyNotice("Preflight complete.");
    } catch (error) {
      setGenerateError(normalizeRepoMapError(error));
    } finally {
      setPreflightBusy(false);
    }
  };

  const chooseRepoFolder = async () => {
    setPickerBusy(true);
    setGenerateError(null);
    setCopyNotice(null);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Choose repository folder",
      });
      if (typeof selected !== "string") return;
      setRepoPath(selected);
      const result = await repoMapTauriAdapter.preflight(selected);
      setPreflight(result);
      setCopyNotice("Repository folder selected.");
    } catch (error) {
      setGenerateError(normalizeRepoMapError(error));
    } finally {
      setPickerBusy(false);
    }
  };

  const runGeneration = async () => {
    setGenerateBusy(true);
    setGenerationStartedAt(Date.now());
    setGenerationElapsedSeconds(0);
    setLiveGenerationEvents([]);
    setLiveToolRuns({});
    setLiveProgressEvidence(null);
    setGenerateError(null);
    setCopyNotice(null);
    setOpenNotice(null);
    setShowRunOutput(true);
    try {
      const result = await repoMapTauriAdapter.generate(repoPath.trim() || null);
      setGeneration(result);
      setRepoMap(result.map);
      setRepoPath(result.repoPath);
      upsertHistory(result);
      void runPreflight(false);
    } catch (error) {
      setGenerateError(normalizeRepoMapError(error));
    } finally {
      setGenerateBusy(false);
    }
  };

  const copyCompactContext = async () => {
    const text =
      generation?.compactContext ??
      `Repo Map Compact Context\nGraphify: ${repoMap.tools.graphify.nodeCount} nodes, ${repoMap.tools.graphify.linkCount} links.\nMadge: ${repoMap.tools.madge.moduleCount} modules, ${repoMap.tools.madge.edgeCount} edges, ${repoMap.tools.madge.cycles} cycles.\nTauri invoke wiring: ${repoMap.tauri.invokedCommandCount} invokes, ${repoMap.tauri.commandCount} Rust commands, ${repoMap.tauri.missingRustCommand.length} missing Rust commands, ${repoMap.tauri.missingHandler.length} missing handlers.`;
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }
    await navigator.clipboard.writeText(text);
    setCopyNotice("Compact context copied.");
  };

  const copyInstallHint = async (hint: string) => {
    if (!navigator.clipboard) {
      setCopyNotice("Clipboard unavailable.");
      return;
    }
    await navigator.clipboard.writeText(hint);
    setCopyNotice("Install hint copied.");
  };

  const openArtifact = async (artifact: string) => {
    setOpenNotice(null);
    setGenerateError(null);
    try {
      await repoMapTauriAdapter.openArtifact({ repoPath: selectedRepo, artifact });
      setOpenNotice("Opened artifact.");
    } catch (error) {
      setGenerateError(normalizeRepoMapError(error));
    }
  };

  const loadHistoryItem = (item: RepoMapHistoryItem) => {
    setRepoPath(item.repoPath);
    setCopyNotice("History path loaded. Generate to refresh this map.");
  };

  const healthCards = [
    {
      label: "Graphify graph",
      value: `${formatNumber(repoMap.tools.graphify.nodeCount)} nodes`,
      detail: `${formatNumber(repoMap.tools.graphify.linkCount)} links`,
      healthy: graphifyReady,
    },
    {
      label: "Frontend imports",
      value: `${formatNumber(repoMap.tools.madge.moduleCount)} modules`,
      detail: `${formatNumber(repoMap.tools.madge.edgeCount)} edges, ${repoMap.tools.madge.cycles} cycles`,
      healthy: frontendHealthy,
    },
    {
      label: "Tauri wiring",
      value: `${formatNumber(repoMap.tauri.invokedCommandCount)} invokes`,
      detail: wiringHealthy
        ? "0 missing commands or handlers"
        : `${repoMap.tauri.missingRustCommand.length} missing commands, ${repoMap.tauri.missingHandler.length} missing handlers`,
      healthy: wiringHealthy,
    },
    {
      label: "Token context",
      value: tokenSavings
        ? `${formatNumber(tokenSavings.estimatedTokensAvoided)} avoided`
        : `${formatNumber(repoMap.tools.cargoMetadata.dependencyCount)} deps`,
      detail: tokenSavings
        ? `${formatNumber(tokenSavings.compactContextEstimatedTokens)} compact tokens`
        : `${formatNumber(repoMap.inventory.rustFiles)} Rust files`,
      healthy: true,
    },
  ];

  const toolRuns = useMemo(
    () => [
      ["graphify", "Graphify"],
      ["madge", "Madge"],
      ["dependencyCruiser", "dependency-cruiser"],
      ["cargoMetadata", "Cargo metadata"],
      ["tauriScan", "Tauri scan"],
    ] as const,
    [],
  );
  const observedToolRuns = {
    ...(repoMap.toolRuns ?? {}),
    ...(generation?.map.toolRuns ?? {}),
    ...liveToolRuns,
  };
  const generationSteps = buildRepoMapProgressSteps({
    generateBusy,
    generateError,
    preflightTools: preflight?.tools,
    currentToolId: liveProgressEvidence?.currentToolId,
    toolRuns: observedToolRuns,
  });
  const generationProgress = buildRepoMapProgressSummary(generationSteps, {
    generateBusy,
    generateError,
    currentToolId: liveProgressEvidence?.currentToolId,
    progressPercent: liveProgressEvidence?.percent,
    completedTools: liveProgressEvidence?.completed,
    totalTools: liveProgressEvidence?.total,
  });
  const activeGenerationStep =
    generationSteps.find((step) => step.state === "running") ??
    generationSteps.find((step) => step.state === "warning") ??
    generationSteps[generationSteps.length - 1];
  const runOutputId = "repo-map-run-output";
  const liveStdout = liveGenerationEvents
    .filter((event) => event.stream === "stdout")
    .map((event) => event.message)
    .join("\n");
  const liveStderr = liveGenerationEvents
    .filter((event) => event.stream === "stderr")
    .map((event) => event.message)
    .join("\n");
  const liveStatus = liveGenerationEvents
    .filter((event) => event.stream === "status")
    .slice(-4);
  const missingPreflightTools =
    preflight?.tools.filter((tool) => !tool.available) ?? [];
  const toolInstallDetailsId = "repo-map-tool-install-details";
  const graphDiagnosticsId = "repo-map-graph-diagnostics";

  return (
    <section className="repo-map-view" aria-labelledby="repo-map-title">
      <header className={`repo-map-hero repo-map-hero--${tone}`}>
        <div className="repo-map-hero__icon" aria-hidden="true">
          <Graph size={28} weight="duotone" />
        </div>
        <div className="repo-map-hero__copy">
          <p className="repo-map-eyebrow">App Health</p>
          <h1 id="repo-map-title">Repo Map</h1>
          <p>
            Generated {formatGeneratedAt(repoMap.generatedAt)} from Graphify,
            Madge, dependency-cruiser, Cargo metadata, and Tauri invoke wiring.
          </p>
        </div>
        <div className="repo-map-hero__status" role="status">
          {tone === "healthy" ? (
            <CheckCircle size={18} weight="fill" />
          ) : (
            <WarningCircle size={18} weight="fill" />
          )}
          <span>{tone === "healthy" ? "Healthy" : "Needs review"}</span>
        </div>
      </header>

      <article className="repo-map-panel repo-map-picker">
        <div className="repo-map-panel__header">
          <FolderOpen size={18} weight="duotone" />
          <h2>Repository</h2>
        </div>
        <div className="repo-map-picker__row">
          <input
            aria-label="Repository path"
            className="repo-map-picker__input"
            onChange={(event) => setRepoPath(event.target.value)}
            spellCheck={false}
            value={repoPath}
          />
          <button
            className="secondary-button"
            disabled={pickerBusy}
            onClick={() => void chooseRepoFolder()}
            type="button"
          >
            <FolderOpen size={15} weight="bold" />
            {pickerBusy ? "Opening..." : "Browse"}
          </button>
          <button
            className="secondary-button"
            disabled={preflightBusy || pickerBusy}
            onClick={() => runPreflight()}
            type="button"
          >
            <MagnifyingGlass size={15} weight="bold" />
            {preflightBusy ? "Checking..." : "Check"}
          </button>
        </div>
        {preflight ? (
          <div className="repo-map-preflight" aria-label="Repo map preflight">
            <span className={preflight.exists && preflight.isDirectory ? "is-ok" : "is-warning"}>
              {preflight.exists && preflight.isDirectory ? "Path ready" : "Path missing"}
            </span>
            <span className={preflight.hasPackageJson ? "is-ok" : "is-muted"}>
              package.json {preflight.hasPackageJson ? "found" : "missing"}
            </span>
            <span className={preflight.hasCargoManifest ? "is-ok" : "is-muted"}>
              Cargo {preflight.hasCargoManifest ? "found" : "missing"}
            </span>
          </div>
        ) : null}
      </article>

      <div className="repo-map-actions repo-map-actions--primary">
        <button
          className="primary-button"
          disabled={generateBusy}
          onClick={runGeneration}
          type="button"
        >
          <ArrowClockwise
            className={generateBusy ? "is-spinning" : undefined}
            size={15}
            weight="bold"
          />
          {generateBusy ? "Generating map..." : "Generate repo map"}
        </button>
        <button className="secondary-button" onClick={copyCompactContext} type="button">
          <Copy size={15} weight="bold" />
          Copy compact context
        </button>
        {generation ? <span>{generation.outDir}</span> : null}
        {copyNotice ? <span>{copyNotice}</span> : null}
        {openNotice ? <span>{openNotice}</span> : null}
      </div>

      {missingPreflightTools.length > 0 ? (
        <div className="repo-map-disclosure">
          <button
            aria-controls={toolInstallDetailsId}
            aria-expanded={showToolInstallDetails}
            className="repo-map-disclosure__button"
            onClick={() => setShowToolInstallDetails((open) => !open)}
            type="button"
          >
            {showToolInstallDetails ? "Hide install checks" : "Details"}
          </button>
          {showToolInstallDetails ? (
            <div
              className="repo-map-preflight-fixes repo-map-disclosure__panel"
              id={toolInstallDetailsId}
              aria-label="Repo map tool install checks"
            >
              <strong>Tool install checks</strong>
              <ul>
                {missingPreflightTools.map((tool) => (
                  <li key={`${tool.label}-${tool.installHint}`}>
                    <span>{tool.label}</span>
                    <code>{tool.installHint}</code>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
        </div>
      ) : null}

      {generateError ? (
        <p className="repo-map-error" role="alert">
          {generateError}
        </p>
      ) : null}

      {generateBusy ? (
        <article className="repo-map-panel repo-map-run-status" aria-live="polite">
          <div className="repo-map-panel__header">
            <ArrowClockwise
              className="is-spinning"
              size={18}
              weight="duotone"
            />
            <h2>{activeGenerationStep?.label ?? "Generating"}</h2>
          </div>
          <div className="repo-map-run-status__meta">
            <span>{formatElapsedSeconds(generationElapsedSeconds)}</span>
            <span>
              {generationProgress.percent}% · {activeGenerationStep?.detail ?? "Running"}
            </span>
          </div>
          <progress
            aria-label="Repo map generation percent"
            max={100}
            value={generationProgress.percent}
          />
          <div className="repo-map-progress" aria-label="Repo map generation progress">
            {generationSteps.map((step) => (
              <span
                className={
                  step.state === "running"
                    ? "repo-map-progress__step repo-map-progress__step--active"
                    : `repo-map-progress__step repo-map-progress__step--${step.state}`
                }
                key={step.id}
                title={step.detail}
              >
                {step.label}
              </span>
            ))}
          </div>
        </article>
      ) : null}

      {generation || liveGenerationEvents.length > 0 ? (
        <div className="repo-map-disclosure">
          <button
            aria-controls={runOutputId}
            aria-expanded={showRunOutput}
            className="repo-map-disclosure__button"
            onClick={() => setShowRunOutput((open) => !open)}
            type="button"
          >
            {showRunOutput ? "Hide run output" : "Run output"}
          </button>
          {showRunOutput ? (
            <div
              className="repo-map-run-output repo-map-disclosure__panel"
              id={runOutputId}
              aria-label="Repo map run output"
            >
              {liveStatus.length > 0 ? (
                <div className="repo-map-run-output__status">
                  <strong>status</strong>
                  <ul>
                    {liveStatus.map((event, index) => (
                      <li key={`${event.phase}-${index}`}>
                        <span>{event.phase}</span>
                        <small>{event.message}</small>
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
              <div>
                <strong>{generateBusy ? "live stdout" : "stdout"}</strong>
                <pre>
                  {compactLogTail(
                    generateBusy ? liveStdout : generation?.stdoutTail ?? liveStdout,
                  )}
                </pre>
              </div>
              <div>
                <strong>{generateBusy ? "live stderr" : "stderr"}</strong>
                <pre>
                  {compactLogTail(
                    generateBusy ? liveStderr : generation?.stderrTail ?? liveStderr,
                  )}
                </pre>
              </div>
            </div>
          ) : null}
        </div>
      ) : null}

      <div className="repo-map-health-grid" aria-label="Repo map health">
        {healthCards.map((card) => (
          <article
            className={`repo-map-health-card${card.healthy ? " is-healthy" : " is-warning"}`}
            key={card.label}
          >
            <span className="repo-map-health-card__label">{card.label}</span>
            <strong>{card.value}</strong>
            <span>{card.detail}</span>
          </article>
        ))}
      </div>

      <div className="repo-map-disclosure">
        <button
          aria-controls={graphDiagnosticsId}
          aria-expanded={showGraphDiagnostics}
          className="repo-map-disclosure__button"
          onClick={() => setShowGraphDiagnostics((open) => !open)}
          type="button"
        >
          {showGraphDiagnostics ? "Hide diagnostics" : "Learn more"}
        </button>
        {showGraphDiagnostics ? (
          <div
            className="repo-map-layout repo-map-disclosure__panel"
            id={graphDiagnosticsId}
            aria-label="Repo map graph diagnostics"
          >
            <article className="repo-map-panel">
              <div className="repo-map-panel__header">
                <GitBranch size={18} weight="duotone" />
                <h2>Tool Status</h2>
              </div>
              <ul className="repo-map-tool-list">
                {toolRuns.map(([id, label]) => {
                  const run = observedToolRuns[id];
                  const status = run?.status ?? "not-run";
                  return (
                    <li className={`repo-map-tool-list__item repo-map-tool-list__item--${status}`} key={id}>
                      <span>{label}</span>
                      <strong>{status}</strong>
                      {run?.remediation ? <small>{run.remediation}</small> : null}
                    </li>
                  );
                })}
              </ul>
            </article>

            <article className="repo-map-panel">
              <div className="repo-map-panel__header">
                <FileCode size={18} weight="duotone" />
                <h2>Tool Checks</h2>
              </div>
              <ul className="repo-map-tool-list">
                {(preflight?.tools ?? []).map((tool) => (
                  <li
                    className={`repo-map-tool-list__item repo-map-tool-list__item--${
                      tool.available ? "ok" : "warning"
                    }`}
                    key={tool.id}
                  >
                    <span>{tool.label}</span>
                    <strong>{tool.available ? "ready" : "missing"}</strong>
                    {tool.installHint ? (
                      <button
                        className="repo-map-tool-list__copy"
                        onClick={() => copyInstallHint(tool.installHint ?? "")}
                        type="button"
                      >
                        Copy fix
                      </button>
                    ) : null}
                  </li>
                ))}
              </ul>
            </article>
          </div>
        ) : null}
      </div>

      <div className="repo-map-layout">
        <article className="repo-map-panel">
          <div className="repo-map-panel__header">
            <GitBranch size={18} weight="duotone" />
            <h2>Architecture Shape</h2>
          </div>
          <dl className="repo-map-metrics">
            <div>
              <dt>Frontend files</dt>
              <dd>{formatNumber(repoMap.inventory.frontendFiles)}</dd>
            </div>
            <div>
              <dt>Rust files</dt>
              <dd>{formatNumber(repoMap.inventory.rustFiles)}</dd>
            </div>
            <div>
              <dt>Docs</dt>
              <dd>{formatNumber(repoMap.inventory.docs)}</dd>
            </div>
            <div>
              <dt>Scripts</dt>
              <dd>{formatNumber(repoMap.inventory.scripts)}</dd>
            </div>
          </dl>
        </article>

        <article className="repo-map-panel">
          <div className="repo-map-panel__header">
            <FileCode size={18} weight="duotone" />
            <h2>Frontend Hotspots</h2>
          </div>
          <ul className="repo-map-hotspots">
            {repoMap.frontend.topFanOut.slice(0, 6).map((item) => (
              <li key={item.file}>
                <span>{item.file}</span>
                <strong>{item.imports}</strong>
              </li>
            ))}
          </ul>
        </article>
      </div>

      <article className="repo-map-panel">
        <div className="repo-map-panel__header">
          <ArrowClockwise size={18} weight="duotone" />
          <h2>Artifacts</h2>
        </div>
        <div className="repo-map-artifact-actions">
          {[
            ["folder", "Map folder"],
            ["readme", "README"],
            ["compactContext", "Compact context"],
            ["graphTree", "Graph tree"],
            ["repoMapJson", "JSON"],
          ].map(([artifact, label]) => (
            <button
              className="secondary-button secondary-button--small"
              key={artifact}
              onClick={() => openArtifact(artifact)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
        <div className="repo-map-artifacts">
          {repoMap.tools.graphify.files.map((file) => (
            <code key={file}>{file}</code>
          ))}
          <code>{repoMap.tools.madge.file}</code>
          <code>{repoMap.tools.dependencyCruiser.file}</code>
          <code>{repoMap.tools.cargoMetadata.file}</code>
        </div>
      </article>

      {history.length ? (
        <article className="repo-map-panel">
          <div className="repo-map-panel__header">
            <FolderOpen size={18} weight="duotone" />
            <h2>Recent Maps</h2>
          </div>
          <ul className="repo-map-history">
            {history.map((item) => (
              <li key={`${item.repoPath}-${item.generatedAt}`}>
                <button onClick={() => loadHistoryItem(item)} type="button">
                  <span>{item.repoPath}</span>
                  <small>
                    {formatGeneratedAt(item.generatedAt)} · {formatNumber(item.graphNodes)} nodes · {formatNumber(item.estimatedTokensAvoided)} tokens avoided
                  </small>
                </button>
              </li>
            ))}
          </ul>
        </article>
      ) : null}

      <div className="repo-map-actions">
        <button className="secondary-button" onClick={onOpenRepoIntelligence} type="button">
          Open Repo Intelligence
        </button>
        <button className="secondary-button" onClick={onOpenDoctor} type="button">
          Open Doctor
        </button>
      </div>
    </section>
  );
}
