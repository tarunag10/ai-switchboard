import {
  ArrowClockwise,
  CheckCircle,
  Copy,
  GitBranch,
  Pause,
  Play,
  ShieldCheck,
  Stop,
} from "@phosphor-icons/react";
import { useCallback, useEffect, useMemo, useState } from "react";

import type { SwitchboardMode } from "../lib/types";
import {
  listModelRoutingDecisionReferences,
  type ModelRoutingDecisionReference,
} from "../lib/optimization";
import { hasTauriRuntime } from "../lib/tauriRuntime";
import {
  WORKBENCH_CAPABILITIES,
  createWorkbenchSession,
  exportWorkbenchSession,
  forkWorkbenchSession,
  getWorkbenchCapabilityProjection,
  isWorkbenchDigest,
  listWorkbenchSessions,
  prepareWorkbenchRunPlan,
  transitionWorkbenchSession,
  type WorkbenchCapabilityProjection,
  type WorkbenchRunPlan,
  type WorkbenchSession,
  type WorkbenchSessionAction,
  type WorkbenchTaskClass,
} from "../lib/workbench";

interface WorkbenchViewProps {
  hidden: boolean;
}

const taskClasses: Array<{ id: WorkbenchTaskClass; label: string }> = [
  { id: "coding", label: "Coding" },
  { id: "review", label: "Review" },
  { id: "analysis", label: "Analysis" },
  { id: "planning", label: "Planning" },
];

const adapters = [
  { id: "codex", label: "Codex" },
  { id: "claude_code", label: "Claude Code" },
  { id: "gemini_cli", label: "Gemini CLI" },
] as const;

const modes: Array<{ id: SwitchboardMode; label: string }> = [
  { id: "full", label: "Full" },
  { id: "headroom", label: "Headroom" },
  { id: "rtk", label: "RTK" },
  { id: "off", label: "Off" },
];

function messageFrom(reason: unknown, fallback: string): string {
  return reason instanceof Error ? reason.message : fallback;
}

function replaceSession(
  sessions: WorkbenchSession[],
  next: WorkbenchSession,
): WorkbenchSession[] {
  return [next, ...sessions.filter((session) => session.sessionId !== next.sessionId)]
    .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

function eventLabel(kind: string): string {
  return kind.replace(/_/g, " ");
}

function actionPastTense(action: WorkbenchSessionAction): string {
  return action === "cancel" ? "cancelled" : `${action}d`;
}

function formatTimestamp(value: string): string {
  const timestamp = new Date(value);
  if (Number.isNaN(timestamp.getTime())) return value;
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(timestamp);
}

export function WorkbenchView({ hidden }: WorkbenchViewProps) {
  const [sessions, setSessions] = useState<WorkbenchSession[]>([]);
  const [projection, setProjection] = useState<WorkbenchCapabilityProjection | null>(null);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [workspaceDigest, setWorkspaceDigest] = useState("");
  const [taskClass, setTaskClass] = useState<WorkbenchTaskClass>("coding");
  const [adapterId, setAdapterId] = useState<(typeof adapters)[number]["id"]>("codex");
  const [requestedMode, setRequestedMode] = useState<SwitchboardMode>("full");
  const [contextPackDigest, setContextPackDigest] = useState("");
  const [routerDecisionId, setRouterDecisionId] = useState("");
  const [routerDecisionReferences, setRouterDecisionReferences] = useState<ModelRoutingDecisionReference[]>([]);
  const [capabilityIds, setCapabilityIds] = useState<string[]>([
    "router_observe",
    "client_adapter_plan",
  ]);
  const [runPlan, setRunPlan] = useState<WorkbenchRunPlan | null>(null);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const selectedSession = useMemo(
    () => sessions.find((session) => session.sessionId === selectedSessionId) ?? null,
    [selectedSessionId, sessions],
  );
  const desktopRuntime = hasTauriRuntime();

  const refresh = useCallback(async () => {
    if (!desktopRuntime) {
      setError("Workbench requires the AI Switchboard desktop runtime. No session data is available in this browser preview.");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const [nextSessions, nextProjection, nextRouterDecisionReferences] = await Promise.all([
        listWorkbenchSessions(),
        getWorkbenchCapabilityProjection(),
        listModelRoutingDecisionReferences(),
      ]);
      const ordered = [...nextSessions].sort((left, right) =>
        right.updatedAt.localeCompare(left.updatedAt),
      );
      setSessions(ordered);
      setProjection(nextProjection);
      setRouterDecisionReferences(nextRouterDecisionReferences);
      setRouterDecisionId((current) =>
        current && nextRouterDecisionReferences.some((reference) => reference.decisionId === current)
          ? current
          : "",
      );
      setSelectedSessionId((current) =>
        current && ordered.some((session) => session.sessionId === current)
          ? current
          : (ordered[0]?.sessionId ?? null),
      );
    } catch (reason) {
      setError(messageFrom(reason, "Workbench metadata could not be loaded."));
    } finally {
      setLoading(false);
    }
  }, [desktopRuntime]);

  useEffect(() => {
    if (!hidden) void refresh();
  }, [hidden, refresh]);

  async function createSession() {
    const digest = workspaceDigest.trim();
    if (!isWorkbenchDigest(digest)) {
      setError("Enter a SHA-256 workspace digest. Paths and workspace contents are not accepted here.");
      return;
    }
    setCreating(true);
    setError(null);
    setNotice(null);
    try {
      const created = await createWorkbenchSession({
        workspaceDigest: digest,
        taskClass,
      });
      setSessions((current) => replaceSession(current, created));
      setSelectedSessionId(created.sessionId);
      setRunPlan(null);
      setNotice("Local Workbench session created. No provider traffic or configuration changes occurred.");
    } catch (reason) {
      setError(messageFrom(reason, "Workbench session could not be created."));
    } finally {
      setCreating(false);
    }
  }

  async function transition(action: WorkbenchSessionAction) {
    if (!selectedSession) return;
    setBusyAction(action);
    setError(null);
    setNotice(null);
    try {
      const next = await transitionWorkbenchSession(selectedSession.sessionId, action);
      setSessions((current) => replaceSession(current, next));
      setRunPlan(null);
      setNotice(`Session ${actionPastTense(action)} in the local content-free ledger.`);
    } catch (reason) {
      setError(messageFrom(reason, `Session could not ${action}.`));
    } finally {
      setBusyAction(null);
    }
  }

  async function forkSelectedSession() {
    const event = selectedSession?.events[selectedSession.events.length - 1];
    if (!selectedSession || !event) return;
    setBusyAction("fork");
    setError(null);
    setNotice(null);
    try {
      const fork = await forkWorkbenchSession(selectedSession.sessionId, event.eventId);
      await refresh();
      setSelectedSessionId(fork.sessionId);
      setRunPlan(null);
      setNotice("Fork created from the latest ledger event. The parent and child remain content-free.");
    } catch (reason) {
      setError(messageFrom(reason, "Session fork could not be created."));
    } finally {
      setBusyAction(null);
    }
  }

  async function exportSelectedSession() {
    if (!selectedSession) return;
    setBusyAction("export");
    setError(null);
    try {
      const exported = await exportWorkbenchSession(selectedSession.sessionId);
      await navigator.clipboard?.writeText(JSON.stringify(exported, null, 2));
      setNotice("Validated content-free session ledger copied to the clipboard. No file was written.");
    } catch (reason) {
      setError(messageFrom(reason, "Session ledger could not be exported."));
    } finally {
      setBusyAction(null);
    }
  }

  function toggleCapability(id: string) {
    setCapabilityIds((current) =>
      current.includes(id)
        ? current.filter((capabilityId) => capabilityId !== id)
        : [...current, id],
    );
  }

  async function preparePlan() {
    if (!selectedSession) return;
    const contextDigest = contextPackDigest.trim();
    const decisionId = routerDecisionId.trim();
    if (!decisionId) {
      setError("Select a native observe-only Router decision before preparing a plan.");
      return;
    }
    if (contextDigest && !isWorkbenchDigest(contextDigest)) {
      setError("Context packs must be referenced by a SHA-256 digest, not a path or raw context.");
      return;
    }
    setBusyAction("plan");
    setError(null);
    setNotice(null);
    try {
      const plan = await prepareWorkbenchRunPlan({
        sessionId: selectedSession.sessionId,
        adapterId,
        workspaceDigest: selectedSession.workspaceDigest,
        contextPackDigest: contextDigest || null,
        routerDecisionId: decisionId,
        requiredCapabilityIds: capabilityIds,
        requestedMode,
      });
      setRunPlan(plan);
      setNotice("Adapter plan prepared only. It has not changed any client configuration.");
    } catch (reason) {
      setRunPlan(null);
      setError(messageFrom(reason, "Workbench run plan could not be prepared."));
    } finally {
      setBusyAction(null);
    }
  }

  const canFork = selectedSession?.status === "active" || selectedSession?.status === "paused";
  const latestEvent = selectedSession
    ? selectedSession.events[selectedSession.events.length - 1] ?? null
    : null;

  return (
    <div className="tray-content" hidden={hidden}>
      <section className="workbench-view" aria-labelledby="workbench-title">
        <header className="repo-intelligence-view__header">
          <div>
            <h1 id="workbench-title">Workbench</h1>
            <p className="repo-intelligence-view__subtitle">
              Inspect and coordinate local Router decisions and client adapter plans from one content-free ledger.
            </p>
          </div>
          <span className="repo-intelligence-view__badge">Plan only</span>
        </header>

        <article className="soft-card panel-card workbench-boundary" aria-labelledby="workbench-boundary-title">
          <div className="panel-card__header">
            <div>
              <h2 id="workbench-boundary-title">Safe by current contract</h2>
              <p>
                Sessions hold hashes, opaque IDs, lifecycle events, and plan references only. Prompts, route payloads, paths, credentials, tool arguments, provider traffic, and writes are outside this surface.
              </p>
            </div>
            <ShieldCheck weight="duotone" aria-hidden="true" />
          </div>
          <div className="workbench-boundary__badges" role="status">
            <span>Execution: {projection?.executionMode ?? "plan_only"}</span>
            <span>Provider traffic: {projection?.providerTraffic ?? "none"}</span>
            <span>Writes: {projection?.writesEnabled ? "enabled" : "disabled"}</span>
          </div>
        </article>

        <div className="workbench-grid">
          <article className="soft-card panel-card">
            <div className="panel-card__header">
              <div>
                <h2>New local session</h2>
                <p>Start from a workspace SHA-256 digest; the underlying directory is neither requested nor stored.</p>
              </div>
            </div>
            <label className="workbench-field">
              <span>Workspace SHA-256 digest</span>
              <input
                aria-label="Workspace SHA-256 digest"
                autoCapitalize="none"
                autoCorrect="off"
                onChange={(event) => setWorkspaceDigest(event.target.value)}
                placeholder="sha256:…"
                spellCheck={false}
                value={workspaceDigest}
              />
            </label>
            <label className="workbench-field">
              <span>Task class</span>
              <select aria-label="Task class" onChange={(event) => setTaskClass(event.target.value as WorkbenchTaskClass)} value={taskClass}>
                {taskClasses.map((task) => <option key={task.id} value={task.id}>{task.label}</option>)}
              </select>
            </label>
            <button className="primary-button" disabled={creating || !desktopRuntime} onClick={() => void createSession()} type="button">
              <CheckCircle size={16} weight="bold" aria-hidden="true" />
              {creating ? "Creating…" : "Create local session"}
            </button>
          </article>

          <article className="soft-card panel-card">
            <div className="panel-card__header">
              <div>
                <h2>Capability projection</h2>
                <p>Existing OSS capability metadata is projected from the shared registry; this page does not own a duplicate catalog.</p>
              </div>
              <button aria-label="Refresh Workbench" className="secondary-button secondary-button--small" disabled={loading || !desktopRuntime} onClick={() => void refresh()} type="button">
                <ArrowClockwise className={loading ? "is-spinning" : undefined} size={15} aria-hidden="true" />
                {loading ? "Loading…" : "Refresh"}
              </button>
            </div>
            {projection ? (
              <div className="workbench-projection">
                <p><strong>{projection.registry.providers.length}</strong> providers · <strong>{projection.registry.tools.length}</strong> tools · registry {projection.registry.registryMode}</p>
                <p className="optimize-minimal__meta">Approval mode: {projection.registry.approvalMode}. Capability requests remain pending and non-executable.</p>
              </div>
            ) : <p className="optimize-minimal__meta">Load the desktop Workbench to inspect the current shared capability registry.</p>}
          </article>
        </div>

        {error ? <article className="repo-map-error" role="alert">{error}</article> : null}
        {notice ? <article className="workbench-notice" role="status">{notice}</article> : null}

        <article className="soft-card panel-card" aria-labelledby="workbench-sessions-title">
          <div className="panel-card__header">
            <div>
              <h2 id="workbench-sessions-title">Session ledger</h2>
              <p>Durable local session metadata with explicit lifecycle transitions and deterministic forks.</p>
            </div>
          </div>
          {sessions.length === 0 && !loading ? <p className="optimize-minimal__meta">No Workbench sessions yet. Create one from a workspace digest to prepare a local plan.</p> : null}
          {sessions.length > 0 ? (
            <div className="workbench-sessions" role="list" aria-label="Workbench sessions">
              {sessions.map((session) => (
                <button
                  aria-pressed={selectedSessionId === session.sessionId}
                  className={`workbench-session${selectedSessionId === session.sessionId ? " is-selected" : ""}`}
                  key={session.sessionId}
                  onClick={() => { setSelectedSessionId(session.sessionId); setRunPlan(null); }}
                  type="button"
                >
                  <span><strong>{session.taskClass}</strong> · {session.status}</span>
                  <small>{session.events.length} events · {formatTimestamp(session.updatedAt)}</small>
                  <code>{session.sessionId}</code>
                </button>
              ))}
            </div>
          ) : null}
        </article>

        {selectedSession ? (
          <>
            <div className="workbench-grid">
              <article className="soft-card panel-card" aria-labelledby="workbench-lifecycle-title">
                <div className="panel-card__header">
                  <div>
                    <h2 id="workbench-lifecycle-title">Session lifecycle</h2>
                    <p>Selected session: <code>{selectedSession.sessionId}</code></p>
                  </div>
                </div>
                <div className="workbench-actions">
                  {selectedSession.status === "active" ? <button className="secondary-button secondary-button--small" disabled={busyAction !== null} onClick={() => void transition("pause")} type="button"><Pause size={14} aria-hidden="true" />{busyAction === "pause" ? "Pausing…" : "Pause"}</button> : null}
                  {selectedSession.status === "paused" ? <button className="secondary-button secondary-button--small" disabled={busyAction !== null} onClick={() => void transition("resume")} type="button"><Play size={14} aria-hidden="true" />{busyAction === "resume" ? "Resuming…" : "Resume"}</button> : null}
                  {selectedSession.status === "active" ? <button className="secondary-button secondary-button--small" disabled={busyAction !== null} onClick={() => void transition("complete")} type="button"><CheckCircle size={14} aria-hidden="true" />{busyAction === "complete" ? "Completing…" : "Complete"}</button> : null}
                  {canFork ? <button className="secondary-button secondary-button--small" disabled={busyAction !== null || !latestEvent} onClick={() => void forkSelectedSession()} type="button"><GitBranch size={14} aria-hidden="true" />{busyAction === "fork" ? "Forking…" : "Fork latest event"}</button> : null}
                  {(selectedSession.status === "active" || selectedSession.status === "paused") ? <button className="secondary-button secondary-button--small" disabled={busyAction !== null} onClick={() => void transition("cancel")} type="button"><Stop size={14} aria-hidden="true" />{busyAction === "cancel" ? "Cancelling…" : "Cancel"}</button> : null}
                  <button className="secondary-button secondary-button--small" disabled={busyAction !== null} onClick={() => void exportSelectedSession()} type="button"><Copy size={14} aria-hidden="true" />{busyAction === "export" ? "Exporting…" : "Copy ledger"}</button>
                </div>
                <ol className="workbench-events" aria-label="Selected session events">
                  {selectedSession.events.map((event) => <li key={event.eventId}><span>{event.sequence}. {eventLabel(event.kind)}</span><small>{formatTimestamp(event.occurredAt)}</small></li>)}
                </ol>
              </article>

              <article className="soft-card panel-card" aria-labelledby="workbench-plan-title">
                <div className="panel-card__header">
                  <div>
                    <h2 id="workbench-plan-title">Router and adapter plan</h2>
                    <p>References a Router decision in observe-only mode, then calls the existing adapter <code>plan()</code> contract only.</p>
                  </div>
                </div>
                <div className="workbench-plan-fields">
                  <label className="workbench-field"><span>Client adapter</span><select aria-label="Client adapter" onChange={(event) => setAdapterId(event.target.value as (typeof adapters)[number]["id"])} value={adapterId}>{adapters.map((adapter) => <option key={adapter.id} value={adapter.id}>{adapter.label}</option>)}</select></label>
                  <label className="workbench-field"><span>Requested Switchboard mode</span><select aria-label="Requested Switchboard mode" onChange={(event) => setRequestedMode(event.target.value as SwitchboardMode)} value={requestedMode}>{modes.map((mode) => <option key={mode.id} value={mode.id}>{mode.label}</option>)}</select></label>
                  <label className="workbench-field"><span>Context pack SHA-256 digest (optional)</span><input aria-label="Context pack SHA-256 digest" autoCapitalize="none" autoCorrect="off" onChange={(event) => setContextPackDigest(event.target.value)} placeholder="sha256:…" spellCheck={false} value={contextPackDigest} /></label>
                  <label className="workbench-field"><span>Observe-only Router decision</span><select aria-label="Observe-only Router decision" onChange={(event) => setRouterDecisionId(event.target.value)} value={routerDecisionId}><option value="">Select a completed Router decision</option>{routerDecisionReferences.map((reference) => <option key={reference.decisionId} value={reference.decisionId}>{reference.taskClass} · {formatTimestamp(reference.capturedAt)} · {reference.decisionId}</option>)}</select></label>
                </div>
                <p className="optimize-minimal__meta">Router references are native-issued, content-free receipts. The Workbench resolves the selected ID again before it creates a plan; replay digests are not accepted here.</p>
                <fieldset className="workbench-capabilities">
                  <legend>Required capabilities</legend>
                  {WORKBENCH_CAPABILITIES.map((capability) => <label key={capability.id}><input checked={capabilityIds.includes(capability.id)} onChange={() => toggleCapability(capability.id)} type="checkbox" /><span><strong>{capability.label}</strong><small>{capability.detail}</small></span></label>)}
                </fieldset>
                <button className="primary-button" disabled={busyAction !== null || selectedSession.status === "cancelled" || selectedSession.status === "completed"} onClick={() => void preparePlan()} type="button">
                  <ShieldCheck size={16} weight="bold" aria-hidden="true" />
                  {busyAction === "plan" ? "Preparing…" : "Prepare plan only"}
                </button>
              </article>
            </div>

            {runPlan ? (
              <article className="soft-card panel-card workbench-plan-result" aria-labelledby="workbench-plan-result-title">
                <div className="panel-card__header"><div><h2 id="workbench-plan-result-title">Prepared plan</h2><p>This is an inspectable adapter plan; it cannot execute or alter configuration from Workbench.</p></div><ShieldCheck weight="duotone" aria-hidden="true" /></div>
                <div className="optimization-evidence-capture__grid">
                  <p><strong>Adapter:</strong> {runPlan.adapterId} · {runPlan.adapterAction.replace(/_/g, " ")} · {runPlan.adapterReversible ? "reversible" : "non-reversible"}</p>
                  <p><strong>Requested mode:</strong> {runPlan.requestedMode} · <strong>Capabilities:</strong> {runPlan.capabilityRequests.length} pending approval</p>
                  <p><strong>Execution:</strong> {runPlan.executionMode} · <strong>Provider traffic:</strong> {runPlan.providerTraffic} · <strong>Writes:</strong> disabled</p>
                  <p className="optimize-minimal__meta">Plan ID: {runPlan.planId}</p>
                </div>
              </article>
            ) : null}
          </>
        ) : null}
      </section>
    </div>
  );
}
