import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import {
  optimizationEngines,
  optimizationEngineIds,
  previewOptimizationEngineConfig,
  summarizeOptimizationEngineStatus,
  createOptimizationLifecycleReceipt,
  type OptimizationReceipt,
  type OptimizationEngine,
  type OptimizationEngineId,
} from "../lib/optimizationEngines";
import type { RuntimeStatus } from "../lib/types";

type OptimizationAddonReadinessReport = {
  profileId: OptimizationEngineId;
  configuration: Array<{
    label: string;
    environmentVariable: string;
    present: boolean;
  }>;
  executablePresent: boolean;
  pathPresent: boolean;
  connectivity: {
    attempted: boolean;
    status: string;
    detail: string;
  };
  live: false;
  guidance: string;
};

type OptimizationEngineLocalState = {
  version: 1;
  enabled: Partial<Record<OptimizationEngineId, boolean>>;
  receipts: Array<OptimizationReceipt & { action: string }>;
};

type LeanctxSidecarStatus = {
  configured: boolean;
  enabled: boolean;
  running: boolean;
  executablePresent: boolean;
  loopbackOnly: boolean;
  baseUrl: string | null;
  version: string | null;
  mode: string;
  health: string;
  error: string | null;
  ownership: string;
  liveRequestRouting: boolean;
};

type SemanticCacheStatus = {
  enabled: boolean;
  entries: number;
  hits: number;
  misses: number;
  databasePath: string;
  policy: string;
  disclosure: string;
};

type DisplayState =
  | "configured"
  | "ready"
  | "running"
  | "shadow"
  | "blocked"
  | "off"
  | "checking"
  | "degraded"
  | "offline"
  | "paused"
  | "unavailable";

type LifecycleReceipt = OptimizationReceipt & { action: string };

function displayState(
  engine: OptimizationEngine,
  enabled: boolean,
  leanctxStatus: LeanctxSidecarStatus | null,
  semanticCacheStatus: SemanticCacheStatus | null,
  runtimeStatus: RuntimeStatus | null,
  statusError: string | null = null,
): DisplayState {
  if (engine.status === "blocked" || engine.status === "needs-repair") return "blocked";
  if (statusError) return "unavailable";
  if (engine.id === "headroom-native") {
    if (!runtimeStatus || runtimeStatus.kompressEnabled === null || runtimeStatus.kompressEnabled === undefined) return "checking";
    if (!runtimeStatus.installed) return "off";
    if (runtimeStatus.autoPaused || runtimeStatus.paused) return "paused";
    if (!runtimeStatus.running || !runtimeStatus.proxyReachable) return "offline";
    return runtimeStatus.kompressEnabled ? "running" : "degraded";
  }
  if (engine.id === "leanctx") {
    if (!leanctxStatus) return "checking";
    if (leanctxStatus.error && !leanctxStatus.running) return "unavailable";
    if (leanctxStatus?.running) return "shadow";
    if (leanctxStatus?.configured) return leanctxStatus.enabled ? "configured" : "off";
  }
  if (engine.id === "rtk") {
    const rtk = runtimeStatus?.rtk;
    if (!rtk) return "checking";
    if (rtk.installed && rtk.enabled) return "running";
    return rtk.installed ? "ready" : "off";
  }
  if (engine.id === "semantic-cache") {
    if (!semanticCacheStatus) return "checking";
    return semanticCacheStatus.enabled ? "running" : "ready";
  }
  if (enabled) return "configured";
  return engine.status === "available" ? "ready" : engine.status === "shadow" ? "shadow" : "off";
}

const engineSafety: Record<OptimizationEngineId, string> = {
  "headroom-native": "Live provider requests stay under Headroom’s control.",
  rtk: "Applies to local command output; it does not rewrite provider requests.",
  leanctx: "Shadow-only local observation. It never receives live provider traffic.",
  "llmlingua-2": "Blocked until local quality and protected-content gates pass.",
  chonkify: "Repo/context-pack scope only; original files remain the source of truth.",
  "semantic-cache": "Cache hits are cost-saving replays, not compression; protect secrets and tool state.",
  "pxpipe-text-image": "Blocked until Headroom exposes a versioned text_image seam and quality evidence.",
};

const engineBlocker: Partial<Record<OptimizationEngineId, string>> = {
  chonkify: "Blocked: license and source-provenance evidence is incomplete.",
  "pxpipe-text-image": "Blocked: no upstream Headroom text_image capability is available.",
  "llmlingua-2": "Blocked: local model and quality baseline are not approved.",
};

const storageKey = "ai-switchboard.optimization-engines.v1";
const readinessProfiles = new Set<OptimizationEngineId>([
  "leanctx",
  "llmlingua-2",
  "chonkify",
  "semantic-cache",
  "pxpipe-text-image",
]);

function emptyState(): OptimizationEngineLocalState {
  return { version: 1, enabled: {}, receipts: [] };
}

function loadState(): OptimizationEngineLocalState {
  if (typeof window === "undefined") return emptyState();
  try {
    const parsed = JSON.parse(window.localStorage.getItem(storageKey) ?? "null") as Partial<OptimizationEngineLocalState> | null;
    if (parsed?.version !== 1 || !parsed.enabled || !Array.isArray(parsed.receipts)) return emptyState();
    return { version: 1, enabled: parsed.enabled, receipts: parsed.receipts.slice(0, 30) };
  } catch {
    return emptyState();
  }
}

function supportsReadiness(engine: OptimizationEngine): engine is OptimizationEngine & { id: Exclude<OptimizationEngineId, "headroom-native" | "rtk"> } {
  return readinessProfiles.has(engine.id);
}

function formatReceiptTimestamp(createdAt: string): string {
  const date = new Date(createdAt);
  return Number.isNaN(date.getTime()) ? createdAt : date.toLocaleString();
}

function formatLifecycleReceipts(receipts: LifecycleReceipt[]): string {
  return [
    "# AI Switchboard optimization lifecycle receipts",
    "# These are configuration events, not measured savings.",
    ...receipts.map((receipt) => [
      `- ${receipt.createdAt} | ${receipt.engine} | ${receipt.action}`,
      `  scope=${receipt.scope}; evidence=${receipt.evidence}; ${receipt.fallbackReason ?? "no additional detail"}`,
    ].join("\n")),
  ].join("\n");
}

export function OptimizationEngineProfilesCard({
  onCopyGuidance,
  runtimeStatus = null,
}: {
  onCopyGuidance: (markdown: string, label: string) => void;
  runtimeStatus?: RuntimeStatus | null;
}) {
  const [localState, setLocalState] = useState<OptimizationEngineLocalState>(loadState);

  useEffect(() => {
    window.localStorage.setItem(storageKey, JSON.stringify(localState));
  }, [localState]);

  const recordAction = (engine: OptimizationEngine, enabled: boolean, actionOverride?: string) => {
    if (engine.id === "rtk" || engine.id === "headroom-native") return;
    const action = actionOverride ?? (enabled ? "enabled" : "disabled");
    const receipt = createOptimizationLifecycleReceipt(engine.id, action);
    setLocalState((current) => ({
      version: 1,
      enabled: { ...current.enabled, [engine.id]: enabled },
      receipts: [
        { ...receipt, action },
        ...current.receipts,
      ].slice(0, 30),
    }));
  };

  return (
    <li className="addon-card addon-card--planned gateway-profiles-card">
      <div className="addon-card__body">
        <div className="addon-card__heading">
          <span className="addon-card__name">Token optimization engines</span>
          <span className="addon-card__badge addon-card__badge--planned">Opt-in / gated</span>
        </div>
        <p className="addon-card__description">
          Local-first engine readiness and lifecycle receipts. Leanctx is a guided, shadow-only sidecar; Headroom remains the sole provider proxy.
        </p>
        <p className="addon-card__hint">Headroom Native is the only live provider compressor. Third-party engines are advisory, shadow-only, or blocked until their evidence gates pass.</p>
        <p className="addon-card__hint" aria-label="Optimization engine status summary">{summarizeOptimizationEngineStatus(optimizationEngines)}</p>
        <div className="gateway-profiles-card__list">
          {optimizationEngines.map((engine) => (
            <OptimizationEngineRow
              key={engine.id}
              engine={engine}
              enabled={engine.id === "headroom-native"
                ? runtimeStatus?.kompressEnabled === true
                : engine.id === "rtk"
                  ? runtimeStatus?.rtk?.enabled === true
                  : localState.enabled[engine.id] === true}
              onToggle={recordAction}
              onCopyGuidance={onCopyGuidance}
              runtimeStatus={runtimeStatus}
            />
          ))}
        </div>
        <section className="gateway-profile__receipts" aria-labelledby="optimization-receipts-title">
          <div className="gateway-profile__receipts-heading">
            <div>
              <strong id="optimization-receipts-title">Recent lifecycle receipts</strong>
              <p>These record configuration actions only; measured savings appear in the Savings ledger.</p>
            </div>
            <button
              type="button"
              className="addon-card__action"
              disabled={localState.receipts.length === 0}
              onClick={() => onCopyGuidance(formatLifecycleReceipts(localState.receipts), "optimization lifecycle receipts")}
            >
              Copy receipts
            </button>
          </div>
          {localState.receipts.length === 0 ? (
            <p className="gateway-profile__receipt" role="status">No optimization lifecycle actions recorded yet.</p>
          ) : (
            <ul className="gateway-profile__receipt-list">
              {localState.receipts.slice(0, 8).map((receipt) => (
                <li key={receipt.id}>
                  <strong>{receipt.engine}</strong>
                  <span>{receipt.action} · {receipt.scope} · {receipt.evidence} evidence</span>
                  <time dateTime={receipt.createdAt}>{formatReceiptTimestamp(receipt.createdAt)}</time>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </li>
  );
}

function OptimizationEngineRow({
  engine,
  enabled,
  onToggle,
  onCopyGuidance,
  runtimeStatus,
}: {
  engine: OptimizationEngine;
  enabled: boolean;
  onToggle: (engine: OptimizationEngine, enabled: boolean, action?: string) => void;
  onCopyGuidance: (markdown: string, label: string) => void;
  runtimeStatus: RuntimeStatus | null;
}) {
  const [open, setOpen] = useState(false);
  const [readiness, setReadiness] = useState<OptimizationAddonReadinessReport | null>(null);
  const [leanctxStatus, setLeanctxStatus] = useState<LeanctxSidecarStatus | null>(null);
  const [semanticCacheStatus, setSemanticCacheStatus] = useState<SemanticCacheStatus | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const blocked = engine.status === "blocked";

  const refreshBackendStatus = async () => {
    if (engine.id !== "leanctx" && engine.id !== "semantic-cache") return;
    setStatusError(null);
    try {
      if (engine.id === "leanctx") {
        setLeanctxStatus(await invoke<LeanctxSidecarStatus>("get_leanctx_sidecar_status"));
      } else {
        setSemanticCacheStatus(await invoke<SemanticCacheStatus>("get_semantic_cache_status"));
      }
    } catch (error: unknown) {
      setStatusError(error instanceof Error ? error.message : String(error));
    }
  };

  useEffect(() => {
    void refreshBackendStatus();
  }, [engine.id, runtimeStatus?.running, runtimeStatus?.proxyReachable, runtimeStatus?.paused, runtimeStatus?.autoPaused, runtimeStatus?.rtk?.enabled]);

  const refreshLeanctxStatus = async () => {
    if (engine.id === "leanctx") await refreshBackendStatus();
  };

  const refreshSemanticCacheStatus = async () => {
    if (engine.id === "semantic-cache") await refreshBackendStatus();
  };

  const handleToggle = async () => {
    if (statusError || (engine.id === "leanctx" && !leanctxStatus) || (engine.id === "semantic-cache" && !semanticCacheStatus)) {
      await refreshBackendStatus();
      return;
    }
    if (engine.id === "semantic-cache") {
      setChecking(true);
      setActionError(null);
      try {
        const nextEnabled = !(semanticCacheStatus?.enabled ?? false);
        await invoke("set_addon_enabled", { id: "semantic-cache", enabled: nextEnabled });
        onToggle(engine, nextEnabled);
        await refreshSemanticCacheStatus();
      } catch (error: unknown) {
        setActionError(error instanceof Error ? error.message : String(error));
      } finally {
        setChecking(false);
      }
      return;
    }
    if (engine.id !== "leanctx") {
      setActionError("RTK follows the live Switchboard mode. Use the Switchboard mode control to change it; this card only reports runtime evidence.");
      return;
    }
    setChecking(true);
    setActionError(null);
    try {
      if (!leanctxStatus?.configured) {
        await invoke("install_addon", { id: "leanctx" });
        onToggle(engine, false, "registered");
        await refreshLeanctxStatus();
      } else {
        const nextEnabled = !leanctxStatus.enabled;
        await invoke("set_addon_enabled", { id: "leanctx", enabled: nextEnabled });
        onToggle(engine, nextEnabled);
        await refreshLeanctxStatus();
      }
    } catch (error: unknown) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setChecking(false);
    }
  };

  const checkReadiness = async (runLocalConnectivity: boolean) => {
    if (!supportsReadiness(engine)) return;
    setChecking(true);
    setOpen(true);
    setActionError(null);
    try {
      const report = await invoke<OptimizationAddonReadinessReport>("get_optimization_addon_readiness", {
        profileId: engine.id,
        runLocalConnectivity,
      });
      setReadiness(report);
    } catch (error: unknown) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setChecking(false);
    }
  };

  const effectiveEnabled = engine.id === "leanctx"
    ? (leanctxStatus?.enabled ?? false)
    : engine.id === "semantic-cache"
      ? (semanticCacheStatus?.enabled ?? false)
      : engine.id === "headroom-native"
        ? runtimeStatus?.kompressEnabled === true
        : engine.id === "rtk"
          ? runtimeStatus?.rtk?.enabled === true
          : enabled;
  const state = displayState(engine, effectiveEnabled, leanctxStatus, semanticCacheStatus, runtimeStatus, statusError);
  const statusManaged = engine.id === "leanctx" || engine.id === "semantic-cache";
  const statusKnown = !statusManaged || (!statusError && (engine.id === "leanctx" ? leanctxStatus !== null : semanticCacheStatus !== null));
  const preview = previewOptimizationEngineConfig(engine);
  const previewText = [
    `# ${engine.label} — Switchboard preview only; not applied`,
    `# Status: ${engine.status}; boundary: ${engine.boundary}; evidence: ${engine.evidenceType}`,
    `# Effective UI state: ${state}`,
    ...Object.entries(preview).map(([key, value]) => `${key}=${value}`),
    `# Setup: ${engine.setup}`,
    `# Rollback: ${engine.rollback}`,
    `# Off: ${engine.off}`,
  ].join("\n");
  const toggleLabel = engine.id === "headroom-native"
    ? "Managed by Headroom runtime"
      : engine.id === "rtk"
      ? "Manage in Switchboard modes"
      : engine.id === "leanctx"
    ? leanctxStatus?.configured
      ? effectiveEnabled ? "Disable shadow sidecar" : "Enable shadow sidecar"
      : "Register guided sidecar"
      : effectiveEnabled ? "Disable local profile" : "Enable local profile";

  return (
    <section className="gateway-profile" aria-labelledby={`${engine.id}-title`}>
      <div className="gateway-profile__heading">
        <div>
          <strong id={`${engine.id}-title`}>{engine.label}</strong>
          <span>{engine.id} · {engine.boundary} boundary · {engine.evidenceType} evidence</span>
        </div>
        <span className={`gateway-profile__state gateway-profile__state--${state}`} role="status" aria-live="polite" aria-label={`${engine.label} state: ${state}`}>{state}</span>
      </div>
      <p>{engine.lossiness === "lossless" ? "Lossless or protocol-preserving path." : "Lossy or quality-sensitive path; exact-content protection applies."} Scope: {engine.supportedScope}.</p>
      <p><strong>Safety:</strong> {engineSafety[engine.id]}</p>
      {engineBlocker[engine.id] && <p role="note"><strong>{engineBlocker[engine.id]}</strong></p>}
      {engine.id === "leanctx" && <p role="note"><strong>Setup:</strong> provide LEANCTX_EXECUTABLE and a loopback-only LEANCTX_BASE_URL; optional LEANCTX_ARGS_JSON and LEANCTX_VERSION are user-supplied.</p>}
      {engine.id === "semantic-cache" && <p role="note"><strong>Safety scope:</strong> cache only eligible repeated text requests. Obvious secret markers in requests or responses bypass the cache; response bodies remain local until TTL or clear.</p>}
      <div className="gateway-profile__facts">
        <span>{engine.visibility === "none" ? "No prompt/output visibility" : `${engine.visibility} visibility`}</span>
        <span>{engine.governance.userOptIn ? "User opt-in" : "No user opt-in"}</span>
        <span>{engine.governance.reversible ? "Reversible" : "Not reversible"}</span>
      </div>
      <div className="gateway-profile__actions">
        <button
          type="button"
          className="addon-card__action"
          aria-label={`${toggleLabel} for ${engine.label}`}
          disabled={blocked || engine.id === "headroom-native" || engine.id === "rtk" || checking || !statusKnown}
          onClick={() => void handleToggle()}
        >
          {checking ? "Working…" : toggleLabel}
        </button>
        {statusManaged && (
          <button
            type="button"
            className="addon-card__action"
            aria-label={`${statusError ? "Retry" : "Refresh"} ${engine.label} status`}
            disabled={checking}
            onClick={() => void refreshBackendStatus()}
          >
            {statusError ? "Retry status" : "Refresh status"}
          </button>
        )}
        {supportsReadiness(engine) && (
          <button type="button" className="addon-card__action" aria-label={`Check ${engine.label} readiness`} disabled={checking} onClick={() => void checkReadiness(false)}>
            {checking ? "Checking…" : "Check readiness"}
          </button>
        )}
        {supportsReadiness(engine) && (engine.id === "leanctx" || engine.id === "semantic-cache") && (
          <button type="button" className="addon-card__action" aria-label={`Run ${engine.label} loopback preflight`} disabled={checking} onClick={() => void checkReadiness(true)}>
            Loopback preflight
          </button>
        )}
        <button type="button" className="addon-card__action" aria-expanded={open} aria-controls={`${engine.id}-details`} onClick={() => setOpen((value) => !value)}>
          {open ? "Hide evidence" : "View evidence"}
        </button>
        <button type="button" className="addon-card__action addon-card__action--primary" onClick={() => onCopyGuidance(previewText, `${engine.label} preview`)}>
          Copy preview
        </button>
      </div>
      {statusError && <p className="gateway-profile__inline-feedback" role="alert" aria-live="assertive"><strong>Status unavailable:</strong> {statusError} Try again before changing this engine.</p>}
      {actionError && <p className="gateway-profile__inline-feedback" role="alert" aria-live="polite"><strong>Action failed:</strong> {actionError}</p>}
      {readiness && <p className="gateway-profile__inline-feedback" role="status" aria-live="polite"><strong>Readiness:</strong> {readiness.guidance} Connectivity: {readiness.connectivity.status}.</p>}
      {open && (
        <div className="gateway-profile__details" id={`${engine.id}-details`}>
          <p><strong>Setup:</strong> {engine.setup}</p>
          <p><strong>Rollback:</strong> {engine.rollback}</p>
          <p><strong>Off mode:</strong> {engine.off}</p>
          <p><strong>Visibility:</strong> {engine.visibility}. <strong>Lossiness:</strong> {engine.lossiness}.</p>
          {engine.id === "leanctx" && leanctxStatus && (
            <div>
              <p><strong>Managed sidecar:</strong> {leanctxStatus.configured ? "configured" : "not configured"}; mode: {leanctxStatus.mode}; running: {leanctxStatus.running ? "yes" : "no"}.</p>
              <p><strong>Health:</strong> {leanctxStatus.health}. Executable present: {leanctxStatus.executablePresent ? "yes" : "no"}; loopback-only: {leanctxStatus.loopbackOnly ? "yes" : "no"}.</p>
              <p><strong>Live provider routing:</strong> No. Headroom remains the sole provider proxy. {leanctxStatus.ownership}</p>
              {leanctxStatus.error && <p><strong>Last error:</strong> {leanctxStatus.error}</p>}
            </div>
          )}
          {engine.id === "semantic-cache" && semanticCacheStatus && (
            <div>
              <p><strong>Backend:</strong> {semanticCacheStatus.enabled ? "enabled" : "disabled"}; {semanticCacheStatus.entries} live entries; {semanticCacheStatus.hits} hits / {semanticCacheStatus.misses} misses.</p>
              <p><strong>Policy:</strong> {semanticCacheStatus.policy}. {semanticCacheStatus.disclosure}</p>
              <p><strong>Database:</strong> local app storage only; prompt text is never persisted as a key, while eligible response bodies remain until TTL or clear. Secret-marker screening is conservative and not a guarantee of secrecy.</p>
              <button
                type="button"
                className="addon-card__action"
                onClick={async () => {
                  setChecking(true);
                  try {
                    await invoke("clear_semantic_cache");
                    await refreshSemanticCacheStatus();
                  } catch (error: unknown) {
                    setActionError(error instanceof Error ? error.message : String(error));
                  } finally {
                    setChecking(false);
                  }
                }}
                disabled={checking || semanticCacheStatus.entries === 0}
              >
                Clear cached responses
              </button>
            </div>
          )}
          {engine.id === "headroom-native" && runtimeStatus && (
            <div>
              <p><strong>Live routing:</strong> {runtimeStatus.running && runtimeStatus.proxyReachable ? "Headroom proxy is reachable." : "Headroom proxy is not currently reachable."}</p>
              <p><strong>Native compressor:</strong> {runtimeStatus.kompressEnabled === true ? "enabled" : runtimeStatus.kompressEnabled === false ? "not enabled" : "status unavailable"}. ML runtime: {runtimeStatus.mlInstalled === true ? "installed" : runtimeStatus.mlInstalled === false ? "not installed" : "status unavailable"}.</p>
              {runtimeStatus.startupErrorHint || runtimeStatus.startupError ? <p role="alert"><strong>Runtime evidence:</strong> {runtimeStatus.startupErrorHint ?? runtimeStatus.startupError}</p> : null}
            </div>
          )}
          {readiness && (
            <div>
              <p><strong>Readiness:</strong> {readiness.guidance}</p>
              <p><strong>Live:</strong> No. Executable present: {readiness.executablePresent ? "yes" : "no"}; path present: {readiness.pathPresent ? "yes" : "no"}.</p>
              <p><strong>Connectivity:</strong> {readiness.connectivity.status} — {readiness.connectivity.detail}</p>
              <ul>{readiness.configuration.map((item) => <li key={item.environmentVariable}>{item.label} ({item.environmentVariable}): {item.present ? "present" : "not detected"}</li>)}</ul>
            </div>
          )}
        </div>
      )}
    </section>
  );
}
