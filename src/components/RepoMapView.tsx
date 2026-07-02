import {
  ArrowClockwise,
  CheckCircle,
  Copy,
  FileCode,
  GitBranch,
  Graph,
  WarningCircle,
} from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import repoMapSnapshot from "../../docs/repo-map/repo-map.json";

type RepoMapSnapshot = typeof repoMapSnapshot;

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
  stdoutTail: string;
  stderrTail: string;
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
  const [repoMap, setRepoMap] = useState<RepoMapSnapshot>(repoMapSnapshot);
  const [generation, setGeneration] = useState<RepoMapGenerationResponse | null>(null);
  const [generateBusy, setGenerateBusy] = useState(false);
  const [generateError, setGenerateError] = useState<string | null>(null);
  const [copyNotice, setCopyNotice] = useState<string | null>(null);

  const tone = statusTone(repoMap);
  const graphifyReady = repoMap.tools.graphify.nodeCount > 0;
  const wiringHealthy =
    repoMap.tauri.missingRustCommand.length === 0 &&
    repoMap.tauri.missingHandler.length === 0;
  const frontendHealthy = repoMap.tools.madge.cycles === 0;

  const runGeneration = async () => {
    setGenerateBusy(true);
    setGenerateError(null);
    setCopyNotice(null);
    try {
      const result = await invoke<RepoMapGenerationResponse>("generate_repo_map");
      setGeneration(result);
      setRepoMap(result.map);
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
      detail: `${formatNumber(repoMap.tools.madge.edgeCount)} edges, ${
        repoMap.tools.madge.cycles
      } cycles`,
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
      label: "Rust dependencies",
      value: `${formatNumber(repoMap.tools.cargoMetadata.dependencyCount)} direct`,
      detail: `${formatNumber(repoMap.inventory.rustFiles)} Rust files`,
      healthy: true,
    },
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
            Generated {formatGeneratedAt(repoMap.generatedAt)} from
            Graphify, Madge, dependency-cruiser, Cargo metadata, and Tauri
            invoke wiring.
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
        <button
          className="secondary-button"
          onClick={copyCompactContext}
          type="button"
        >
          <Copy size={15} weight="bold" />
          Copy compact context
        </button>
        {generation ? <span>{generation.outDir}</span> : null}
        {copyNotice ? <span>{copyNotice}</span> : null}
      </div>

      {generateError ? (
        <p className="repo-map-error" role="alert">
          {generateError}
        </p>
      ) : null}

      <div className="repo-map-health-grid" aria-label="Repo map health">
        {healthCards.map((card) => (
          <article
            className={`repo-map-health-card${
              card.healthy ? " is-healthy" : " is-warning"
            }`}
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
        <div className="repo-map-artifacts">
          {repoMap.tools.graphify.files.map((file) => (
            <code key={file}>{file}</code>
          ))}
          <code>{repoMap.tools.madge.file}</code>
          <code>{repoMap.tools.dependencyCruiser.file}</code>
          <code>{repoMap.tools.cargoMetadata.file}</code>
        </div>
      </article>

      <div className="repo-map-actions">
        <button
          className="secondary-button"
          onClick={onOpenRepoIntelligence}
          type="button"
        >
          Open Repo Intelligence
        </button>
        <button className="secondary-button" onClick={onOpenDoctor} type="button">
          Open Doctor
        </button>
      </div>
    </section>
  );
}
