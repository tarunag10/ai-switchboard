import {
  ArrowSquareOut,
  CheckCircle,
  CircleNotch,
  Cpu,
  Gear,
  Info,
  Lightning,
  LockKey,
  Play,
  Warning,
  Wrench,
} from "@phosphor-icons/react";
import { useId } from "react";

export type MasterFeatureId =
  | "agent-memory"
  | "token-xray"
  | "daily-briefing"
  | "agent-session"
  | "repo-intelligence"
  | "addons"
  | "gateway-mcp"
  | "doctor"
  | "rollback";

export type MasterFeatureStatus =
  | "idle"
  | "ready"
  | "running"
  | "complete"
  | "partial"
  | "gated"
  | "manual"
  | "error";

export interface MasterFeatureState {
  status?: MasterFeatureStatus;
  detail?: string;
  actionLabel?: string;
  disabled?: boolean;
}

export interface MasterActivationCardProps {
  activationState?: MasterFeatureStatus;
  progress?: { completed: number; total: number };
  featureStates?: Partial<Record<MasterFeatureId, MasterFeatureState>>;
  onActivateAll: () => void | Promise<void>;
  /** Optional so existing activation-only callers remain source-compatible. */
  onDeactivateAll?: () => void | Promise<void>;
  onActivateFeature: (featureId: MasterFeatureId) => void | Promise<void>;
  /** Optional so existing activation-only callers remain source-compatible. */
  onDeactivateFeature?: (featureId: MasterFeatureId) => void | Promise<void>;
  onOpenFeature?: (featureId: MasterFeatureId) => void;
  /** Overrides the derived active state when activation is managed externally. */
  isActive?: boolean;
  /** Identifies which operation is currently represented by a running state. */
  operation?: "activate" | "deactivate";
  title?: string;
  description?: string;
  className?: string;
}

interface FeatureDefinition {
  id: MasterFeatureId;
  label: string;
  description: string;
  icon: typeof Cpu;
  defaultDetail: string;
}

const FEATURES: FeatureDefinition[] = [
  { id: "agent-memory", label: "Agent Memory", description: "Screen, compact, and safely prepare durable context.", icon: Cpu, defaultDetail: "Ready for local memory preparation." },
  { id: "token-xray", label: "Token X-Ray", description: "Inspect token pressure, cacheability, and context health.", icon: Lightning, defaultDetail: "Ready to inspect the current context." },
  { id: "daily-briefing", label: "Daily Briefing", description: "Refresh the local usage summary and recommendations.", icon: Info, defaultDetail: "Ready to refresh today's briefing." },
  { id: "agent-session", label: "Agent Session", description: "Prepare a stable-prefix handoff for your selected agent.", icon: Play, defaultDetail: "Opens Optimize to copy a session payload." },
  { id: "repo-intelligence", label: "Repo Intelligence", description: "Index the repository and build a focused context pack.", icon: Gear, defaultDetail: "Ready to index the active repository." },
  { id: "addons", label: "Add-ons", description: "Check governed sidecars and local optimization engines.", icon: Wrench, defaultDetail: "Ready to inspect add-on health." },
  { id: "gateway-mcp", label: "Gateway / MCP", description: "Review governed profiles and Repo Memory MCP readiness.", icon: LockKey, defaultDetail: "Guided setup; no external traffic is changed automatically." },
  { id: "doctor", label: "Doctor", description: "Run local diagnostics and surface repairable drift.", icon: Wrench, defaultDetail: "Ready to run local diagnostics." },
  { id: "rollback", label: "Rollback inventory", description: "Refresh Doctor and open Settings rollback coverage.", icon: ArrowSquareOut, defaultDetail: "Ready to inspect rollback inventory." },
];

function statusLabel(status: MasterFeatureStatus): string {
  return ({ idle: "Not started", ready: "Ready", running: "Working", complete: "Complete", partial: "Partial", gated: "Gated", manual: "Manual", error: "Needs attention" })[status];
}

function StatusIcon({ status }: { status: MasterFeatureStatus }) {
  if (status === "complete") return <CheckCircle weight="fill" aria-hidden="true" />;
  if (status === "running") return <CircleNotch className="master-activation-card__spin" weight="bold" aria-hidden="true" />;
  if (status === "gated" || status === "manual" || status === "error") return <Warning weight="fill" aria-hidden="true" />;
  return <span className="master-activation-card__status-dot" aria-hidden="true" />;
}

export function MasterActivationCard({
  activationState = "ready",
  progress,
  featureStates = {},
  onActivateAll,
  onDeactivateAll,
  onActivateFeature,
  onDeactivateFeature,
  onOpenFeature,
  isActive,
  operation,
  title = "Activate your AI workspace",
  description = "Bring the local intelligence, safety, and visibility layers online in one coordinated pass.",
  className = "",
}: MasterActivationCardProps) {
  const statusId = useId();
  const total = progress?.total ?? FEATURES.length;
  const completed = Math.min(progress?.completed ?? FEATURES.filter((feature) => featureStates[feature.id]?.status === "complete").length, total);
  const percent = total > 0 ? Math.round((completed / total) * 100) : 0;
  const isRunning = activationState === "running";
  const isWorkspaceActive = isActive ?? activationState === "complete";
  const isDeactivating = operation === "deactivate" || (isWorkspaceActive && isRunning);
  const isPartial = activationState === "partial" || activationState === "gated" || activationState === "manual";
  const primaryAction = isWorkspaceActive ? onDeactivateAll : onActivateAll;
  const primaryActionLabel = isDeactivating
    ? "Deactivating workspace…"
    : isWorkspaceActive
      ? "Deactivate local workspace"
      : isRunning
        ? "Activating workspace…"
        : "Activate everything";

  return (
    <section className={`master-activation-card ${className}`.trim()} aria-labelledby={`${statusId}-title`}>
      <style>{`
        .master-activation-card{--master-ink:#18212b;--master-muted:#66717d;--master-line:#dce3e8;--master-accent:#e35b36;--master-green:#287a5a;position:relative;overflow:hidden;border:1px solid #cad5dc;border-radius:18px;background:linear-gradient(135deg,#fffdf8 0%,#f4f7f5 100%);box-shadow:0 12px 28px rgba(24,33,43,.09);color:var(--master-ink);padding:24px}
        .master-activation-card:before{content:"";position:absolute;inset:0 0 auto;height:4px;background:linear-gradient(90deg,var(--master-accent),#e6a43a,var(--master-green))}
        .master-activation-card__head{display:flex;justify-content:space-between;gap:20px;align-items:flex-start}.master-activation-card__eyebrow{margin:0 0 8px;color:var(--master-accent);font-size:11px;font-weight:800;letter-spacing:.13em;text-transform:uppercase}.master-activation-card h2{margin:0;font-size:clamp(20px,2.5vw,30px);letter-spacing:-.03em}.master-activation-card__description{max-width:590px;margin:8px 0 0;color:var(--master-muted);line-height:1.5}.master-activation-card__summary{flex:0 0 auto;text-align:right}.master-activation-card__summary strong{display:block;font-size:25px;line-height:1}.master-activation-card__summary span{color:var(--master-muted);font-size:12px}
        .master-activation-card__primary{display:inline-flex;align-items:center;justify-content:center;gap:9px;margin-top:20px;border:0;border-radius:10px;background:var(--master-ink);color:#fff;padding:13px 18px;font:inherit;font-weight:750;cursor:pointer;transition:transform .16s ease,background .16s ease}.master-activation-card__primary:hover:not(:disabled){background:#263746;transform:translateY(-1px)}.master-activation-card__primary:focus-visible,.master-activation-card button:focus-visible{outline:3px solid #e6a43a;outline-offset:3px}.master-activation-card__primary:disabled{cursor:wait;opacity:.65}
        .master-activation-card__progress{margin-top:20px}.master-activation-card__progress-label{display:flex;justify-content:space-between;gap:12px;color:var(--master-muted);font-size:12px;margin-bottom:7px}.master-activation-card__progress-track{height:7px;border-radius:99px;background:#e6ece8;overflow:hidden}.master-activation-card__progress-fill{height:100%;border-radius:inherit;background:linear-gradient(90deg,var(--master-accent),var(--master-green));transition:width .25s ease}.master-activation-card__state{margin:12px 0 0;color:var(--master-muted);font-size:13px}.master-activation-card__state strong{color:var(--master-ink)}
        .master-activation-card__list{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px;margin:22px 0 0;padding:0;list-style:none}.master-activation-card__row{display:flex;align-items:center;gap:12px;min-width:0;border:1px solid var(--master-line);border-radius:12px;background:rgba(255,255,255,.7);padding:12px}.master-activation-card__icon{display:grid;place-items:center;flex:none;width:31px;height:31px;border-radius:9px;background:#edf2ef;color:var(--master-green)}.master-activation-card__copy{min-width:0;flex:1}.master-activation-card__copy strong{display:block;font-size:13px}.master-activation-card__copy p{margin:3px 0 0;color:var(--master-muted);font-size:11px;line-height:1.35}.master-activation-card__meta{display:flex;align-items:center;gap:5px;margin-top:5px;color:var(--master-muted);font-size:10px}.master-activation-card__meta svg{width:14px;color:var(--master-green)}.master-activation-card__row[data-status="gated"] .master-activation-card__meta,.master-activation-card__row[data-status="manual"] .master-activation-card__meta,.master-activation-card__row[data-status="error"] .master-activation-card__meta{color:#a2492f}.master-activation-card__row[data-status="complete"] .master-activation-card__icon{background:#dff1e8;color:var(--master-green)}.master-activation-card__status-dot{display:block;width:8px;height:8px;border-radius:50%;background:#aebbc2}.master-activation-card__actions{display:flex;flex-direction:column;gap:5px;flex:none}.master-activation-card__action{border:0;background:transparent;color:var(--master-ink);font:inherit;font-size:11px;font-weight:700;cursor:pointer;padding:4px}.master-activation-card__action:hover{text-decoration:underline}.master-activation-card__action:disabled{cursor:not-allowed;opacity:.45;text-decoration:none}
        .master-activation-card__disclosure{display:flex;gap:8px;align-items:flex-start;margin:18px 0 0;padding:11px 12px;border-radius:10px;background:#eef2f2;color:var(--master-muted);font-size:11px;line-height:1.45}.master-activation-card__disclosure svg{flex:none;color:#65777e;margin-top:1px}.master-activation-card__spin{animation:master-spin .9s linear infinite}@keyframes master-spin{to{transform:rotate(360deg)}}
        @media (max-width:650px){.master-activation-card{padding:18px}.master-activation-card__head{display:block}.master-activation-card__summary{text-align:left;margin-top:16px}.master-activation-card__list{grid-template-columns:1fr}.master-activation-card__primary{width:100%}}
        @media (prefers-reduced-motion:reduce){.master-activation-card__spin{animation:none}.master-activation-card__primary,.master-activation-card__progress-fill{transition:none}}
      `}</style>
      <div className="master-activation-card__head">
        <div>
          <p className="master-activation-card__eyebrow">Switchboard control center</p>
          <h2 id={`${statusId}-title`}>{title}</h2>
          <p className="master-activation-card__description">{description}</p>
          <button className="master-activation-card__primary" type="button" onClick={primaryAction} disabled={isRunning || (isWorkspaceActive && !onDeactivateAll)} aria-busy={isRunning} aria-describedby={`${statusId}-status`} aria-label={primaryActionLabel}>
            {isRunning ? <CircleNotch className="master-activation-card__spin" weight="bold" aria-hidden="true" /> : <Play weight="fill" aria-hidden="true" />}
            {primaryActionLabel}
          </button>
        </div>
        <div className="master-activation-card__summary" aria-label={`${completed} of ${total} features complete`}>
          <strong>{completed}/{total}</strong><span>features ready</span>
        </div>
      </div>
      <div className="master-activation-card__progress" aria-label="Activation progress">
        <div className="master-activation-card__progress-label"><span>{isWorkspaceActive ? "All local features activated" : isPartial ? "Activation needs a follow-up" : isRunning ? (isDeactivating ? "Reversing local activation plan" : "Applying local activation plan") : "Activation coverage"}</span><span>{percent}%</span></div>
        <div className="master-activation-card__progress-track"><div className="master-activation-card__progress-fill" style={{ width: `${percent}%` }} /></div>
      </div>
      <p className="master-activation-card__state" id={`${statusId}-status`} aria-live="polite"><strong>{statusLabel(activationState)}.</strong>{" "}{isPartial ? "Some steps require credentials, infrastructure, or an explicit manual confirmation." : "Each feature can also be run independently below."}</p>
      <ul className="master-activation-card__list" aria-label="Workspace features">
        {FEATURES.map((feature) => {
          const state = featureStates[feature.id] ?? {};
          const status = state.status ?? "ready";
          const Icon = feature.icon;
          const canOpen = Boolean(onOpenFeature);
          return <li className="master-activation-card__row" data-status={status} key={feature.id}>
            <span className="master-activation-card__icon"><Icon weight="duotone" aria-hidden="true" /></span>
            <div className="master-activation-card__copy"><strong>{feature.label}</strong><p>{feature.description}</p><span className="master-activation-card__meta"><StatusIcon status={status} />{state.detail ?? feature.defaultDetail} · {statusLabel(status)}</span></div>
            <div className="master-activation-card__actions">{(() => {
              const featureIsActive = isWorkspaceActive && status === "complete";
              const actionText = featureIsActive ? "Deactivate" : state.actionLabel ?? (status === "complete" ? "Run again" : "Activate");
              const action = featureIsActive ? onDeactivateFeature : onActivateFeature;
              return <button className="master-activation-card__action" type="button" onClick={() => action?.(feature.id)} disabled={Boolean(state.disabled) || status === "running" || (featureIsActive && !onDeactivateFeature)} aria-label={`${actionText} ${feature.label}`}>{actionText}</button>;
            })()}{canOpen ? <button className="master-activation-card__action" type="button" onClick={() => onOpenFeature?.(feature.id)} aria-label={`Open ${feature.label}`}>Open</button> : null}</div>
          </li>;
        })}
      </ul>
      <p className="master-activation-card__disclosure"><Info weight="duotone" aria-hidden="true" /><span><strong>Safety boundary:</strong> activation prepares and verifies local capabilities. Gateway/MCP setup, provider credentials, external infrastructure, and any destructive rollback action remain explicitly gated or manual.</span></p>
    </section>
  );
}
