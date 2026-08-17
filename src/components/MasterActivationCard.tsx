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
  /** Enables the evidence-gated max compression preset. */
  onActivateMaxCompression?: () => void | Promise<void>;
  maxCompressionDisclosure?: string;
  maxCompressionBusy?: boolean;
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
  onActivateMaxCompression,
  maxCompressionDisclosure,
  maxCompressionBusy = false,
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
  const isWorkspaceActive =
    activationState === "complete" ||
    (isActive === true && activationState !== "partial" && activationState !== "error");
  const isDeactivating = operation === "deactivate" || (isWorkspaceActive && isRunning);
  const isPartial = activationState === "partial" || activationState === "gated" || activationState === "manual";
  const primaryAction = isWorkspaceActive ? onDeactivateAll : onActivateAll;
  const primaryActionLabel = isDeactivating
    ? "Deactivating workspace…"
    : isWorkspaceActive
      ? "Deactivate local workspace"
      : isRunning
        ? "Activating workspace…"
        : activationState === "error" || isPartial
          ? "Retry activation"
        : "Activate everything";
  const firstProblem = FEATURES.map(
    (feature) => featureStates[feature.id],
  ).find((state) => state?.status === "error" || state?.status === "gated");
  const statusMessage = isRunning
    ? isDeactivating
      ? "Reversing Switchboard-owned local changes. Controls stay disabled until the operation finishes."
      : "Activation started. Local capabilities are being prepared and verified."
    : activationState === "error"
      ? `${firstProblem?.detail ?? "Activation did not complete."} Review the failed step below, then retry activation.`
      : isPartial
        ? `${firstProblem?.detail ?? "Some steps need follow-up."} Completed steps remain available; retry after resolving the blocked step.`
        : activationState === "complete"
          ? "Activation completed. All available local capabilities were refreshed."
          : "Each feature can also be run independently below.";

  return (
    <section className={`master-activation-card ${className}`.trim()} aria-labelledby={`${statusId}-title`}>
      <div className="master-activation-card__head">
        <div>
          <p className="master-activation-card__eyebrow">Switchboard control center</p>
          <h2 id={`${statusId}-title`}>{title}</h2>
          <p className="master-activation-card__description">{description}</p>
          <button className="master-activation-card__primary" type="button" onClick={() => void primaryAction?.()} disabled={isRunning || (isWorkspaceActive && !onDeactivateAll)} aria-busy={isRunning} aria-describedby={`${statusId}-status`} aria-label={primaryActionLabel}>
            {isRunning ? <CircleNotch className="master-activation-card__spin" weight="bold" aria-hidden="true" /> : <Play weight="fill" aria-hidden="true" />}
            {primaryActionLabel}
          </button>
          {onActivateMaxCompression ? (
            <button
              className="master-activation-card__secondary"
              type="button"
              onClick={() => void onActivateMaxCompression()}
              disabled={isRunning || maxCompressionBusy || isWorkspaceActive}
              aria-busy={maxCompressionBusy}
              aria-label="Enable max compression"
            >
              <Lightning weight="fill" aria-hidden="true" />
              {maxCompressionBusy ? "Enabling max compression…" : "Enable max compression"}
            </button>
          ) : null}
        </div>
        <div className="master-activation-card__summary" aria-label={`${completed} of ${total} features complete`}>
          <strong>{completed}/{total}</strong><span>features ready</span>
        </div>
      </div>
      <div className="master-activation-card__progress" aria-label="Activation progress">
        <div className="master-activation-card__progress-label"><span>{isWorkspaceActive ? "All local features activated" : isPartial ? "Activation needs a follow-up" : isRunning ? (isDeactivating ? "Reversing local activation plan" : "Applying local activation plan") : "Activation coverage"}</span><span>{percent}%</span></div>
        <div className="master-activation-card__progress-track"><div className="master-activation-card__progress-fill" style={{ width: `${percent}%` }} /></div>
      </div>
      <p className="master-activation-card__state" id={`${statusId}-status`} role="status" aria-live={activationState === "error" ? "assertive" : "polite"}><strong>{statusLabel(activationState)}.</strong>{" "}{statusMessage}</p>
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
              return <button className="master-activation-card__action" type="button" onClick={() => void action?.(feature.id)} disabled={isRunning || Boolean(state.disabled) || status === "running" || (featureIsActive && !onDeactivateFeature)} aria-label={`${actionText} ${feature.label}`}>{actionText}</button>;
            })()}{canOpen ? <button className="master-activation-card__action" type="button" onClick={() => onOpenFeature?.(feature.id)} disabled={isRunning} aria-label={`Open ${feature.label}`}>Open</button> : null}</div>
          </li>;
        })}
      </ul>
      <p className="master-activation-card__disclosure"><Info weight="duotone" aria-hidden="true" /><span><strong>Safety boundary:</strong> activation prepares and verifies local capabilities. Gateway/MCP setup, provider credentials, external infrastructure, and any destructive rollback action remain explicitly gated or manual.{maxCompressionDisclosure ? <> <strong>Max compression:</strong> {maxCompressionDisclosure}</> : null}</span></p>
    </section>
  );
}
