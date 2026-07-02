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
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import repoMapSnapshot from "../../docs/repo-map/repo-map.json";

type RepoMapSnapshot = typeof repoMapSnapshot & {
  toolRuns?: Record<string, RepoMapToolRunStatus>;
  tokenSavings?: RepoMapTokenSavings;
};

interface RepoMapToolRunStatus {
  status: "ok" | "warning" | "not-run";
  detail: string;
  remediation: string | null;
}

interface RepoMapTokenSavings {
  compactContextEstimatedTokens: number;
  broadScanEstimatedTokens: number;
  estimatedTokensAvoided: number;
  method: string;
}

interface RepoMapViewProps {
  onOpenRepoIntelligence: () => void;
  onOpenDoctor: () => void;
}

interface RepoMapGenerationResponse {
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

interface RepoMapPreflightTool {
  id: string;
  label: string;
  available: boolean;
  detail: string;
  installHint: string | null;
}

interface RepoMapPreflightResponse {
  repoPath: string;
  exists: boolean;
  isDirectory: boolean;
  hasPackageJson: boolean;
  hasCargoManifest: boolean;
  tools: RepoMapPreflightTool[];
}

interface RepoMapHistoryItem {
  repoPath: string;
  generatedAt: string;
  outDir: string;
  graphNodes: number;
  estimatedTokensAvoided: number;
}

const HISTORY_KEY = "mac-ai-switchboard:repoMapHistory";
const DEFAULT_REPO_PATH = "/Users/tarunagarwal/Developer/Codex-Repos/mac-ai-switchboard";

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

function statusTone(map: RepoMapSnapshot): "healthy" | "warning" {
  return map.tauri.missingRustCommand.length === 0 &&
    map.tauri.missingHandler.length === 0 &&
    map.tools.madge.cycles === 0
    ? "healthy"
    : "warning";
}

function readHistory(): RepoMapHistoryItem[] {
  try {
    const raw = window.localStorage.getItem(HISTORY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.slice(0, 8) : [];
  } catch {
    return [];
  }
}

function writeHistory(items: RepoMapHistoryItem[]) {
  try {
    window.localStorage.setItem(HISTORY_KEY, JSON.stringify(items.slice(0, 8)));
  } catch {
    // History is a convenience only.
  }
}

export function RepoMapView({
  onOpenDoctor,
  onOpenRepoIntelligence,
}: RepoMapViewProps) {
  const [repoMap, setRepoMap] = useState<RepoMapSnapshot>(
    repoMapSnapshot as unknown as RepoMapSnapshot,
  );
  const [repoPath, setRepoPath] = useState(DEFAULT_REPO_PATH);
  const [generation, setGeneration] = useState<RepoMapGenerationResponse | null>(null);
  const [preflight, setPreflight] = useState<RepoMapPreflightResponse | null>(null);
  const [preflightBusy, setPreflightBusy] = useState(false);
  const [generateBusy, setGenerateBusy] = useState(false);
  const [generateError, setGenerateError] = useState<string | null>(null);
  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const [openNotice, setOpenNotice] = useState<string | null>(null);
  const [history, setHistory] = useState<RepoMapHistoryItem[]>(() => readHistory());

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

  const upsertHistory = (result: RepoMapGenerationResponse) => {
    const item: RepoMapHistoryItem = {
      repoPath: result.repoPath,
      generatedAt: result.map.generatedAt,
      outDir: result.outDir,
      graphNodes: result.map.tools.graphify.nodeCount,
      estimatedTokensAvoided: result.map.tokenSavings?.estimatedTokensAvoided ?? 0,
    };
    const next = [item, ...history.filter((entry) => entry.repoPath !== item.repoPath)].slice(0, 8);
    setHistory(next);
    writeHistory(next);
  };

  const runPreflight = async (showNotice = true) => {
    setPreflightBusy(true);
    setGenerateError(null);
    try {
      const result = await invoke<RepoMapPreflightResponse>("preflight_repo_map", {
        repoPath: repoPath.trim() || null,
      });
      setPreflight(result);
      if (showNotice) setCopyNotice("Preflight complete.");
    } catch (error) {
      setGenerateError(error instanceof Error ? error.message : String(error));
    } finally {
      setPreflightBusy(false);
    }
  };

  const runGeneration = async () => {
    setGenerateBusy(true);
    setGenerateError(null);
    setCopyNotice(null);
    setOpenNotice(null);
    try {
      const result = await invoke<RepoMapGenerationResponse>("generate_repo_map", {
        repoPath: repoPath.trim() || null,
      });
      setGeneration(result);
      setRepoMap(result.map);
      setRepoPath(result.repoPath);
      upsertHistory(result);
      void runPreflight(false);
    } catch (error) {
      setGenerateError(error instanceof Error ? error.message : String(error));
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
      await invoke<boolean>("open_repo_map_artifact", {
        request: { repoPath: selectedRepo, artifact },
      });
      setOpenNotice("Opened artifact.");
    } catch (error) {
      setGenerateError(error instanceof Error ? error.message : String(error));
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
    ] as const,
    [],
  );
  const generationSteps = [
    "Preflight",
    "Graphify",
    "Madge",
    "dependency-cruiser",
    "Cargo metadata",
    "Tauri invoke scan",
    "Compact context",
  ];

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
            disabled={preflightBusy}
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

      {generateError ? (
        <p className="repo-map-error" role="alert">
          {generateError}
        </p>
      ) : null}

      {generateBusy ? (
        <div className="repo-map-progress" aria-label="Repo map generation progress">
          {generationSteps.map((step, index) => (
            <span
              className={
                index === 0
                  ? "repo-map-progress__step repo-map-progress__step--active"
                  : "repo-map-progress__step"
              }
              key={step}
            >
              {step}
            </span>
          ))}
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

      <div className="repo-map-layout">
        <article className="repo-map-panel">
          <div className="repo-map-panel__header">
            <GitBranch size={18} weight="duotone" />
            <h2>Tool Status</h2>
          </div>
          <ul className="repo-map-tool-list">
            {toolRuns.map(([id, label]) => {
              const run = repoMap.toolRuns?.[id];
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
