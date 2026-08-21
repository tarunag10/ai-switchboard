import { ArrowRight, CheckCircle, Circle, ClipboardText, FolderOpen, Gauge, ShieldCheck } from "@phosphor-icons/react";

import type { RuntimeStatus, SwitchboardMode } from "../lib/types";
import type { TrayView } from "../lib/trayHelpers";

type SessionReadyCardProps = {
  runtimeStatus: RuntimeStatus | null;
  switchboardMode: SwitchboardMode;
  setActiveView: (view: TrayView) => void;
};

function readiness(runtimeStatus: RuntimeStatus | null) {
  return runtimeStatus?.running && runtimeStatus.proxyReachable;
}

export function SessionReadyCard({ runtimeStatus, switchboardMode, setActiveView }: SessionReadyCardProps) {
  const runtimeReady = readiness(runtimeStatus);
  const steps = [
    {
      label: "Repo context",
      detail: "Freshness and bounded context pack",
      icon: FolderOpen,
      view: "repoIntelligence" as TrayView,
      complete: true,
    },
    {
      label: "Agent handoff",
      detail: "Stable-prefix session payload",
      icon: ClipboardText,
      view: "optimization" as TrayView,
      complete: false,
    },
    {
      label: "Runtime",
      detail: runtimeReady ? "Loopback route is healthy" : "Run Doctor before relying on routing",
      icon: ShieldCheck,
      view: "doctor" as TrayView,
      complete: runtimeReady,
    },
    {
      label: "Proof",
      detail: "Savings and caveats after the task",
      icon: Gauge,
      view: "usage" as TrayView,
      complete: false,
    },
  ];

  return (
    <section className="optimize-card session-ready-card" aria-labelledby="session-ready-title">
      <div className="optimize-card__head">
        <div className="optimize-card__title-row">
          <span className="optimize-card__title-icon"><ClipboardText weight="duotone" /></span>
          <div>
            <h2 id="session-ready-title">Session Ready</h2>
            <p className="optimize-minimal__meta">Prepare, run, and explain one coding-agent session from one path.</p>
          </div>
        </div>
        <span className="optimize-minimal__meta">Mode: {switchboardMode}</span>
      </div>
      <div className="session-ready-card__steps">
        {steps.map((step) => {
          const Icon = step.icon;
          return (
            <button className="session-ready-card__step" key={step.label} type="button" onClick={() => setActiveView(step.view)}>
              <span className="session-ready-card__step-icon" aria-hidden="true">
                {step.complete ? <CheckCircle weight="fill" /> : <Circle weight="regular" />}
              </span>
              <span className="session-ready-card__step-copy">
                <strong><Icon size={15} weight="duotone" /> {step.label}</strong>
                <small>{step.detail}</small>
              </span>
              <ArrowRight size={14} aria-hidden="true" />
            </button>
          );
        })}
      </div>
      <button className="primary-button" type="button" onClick={() => setActiveView("optimization")}>
        Prepare agent handoff <ArrowRight size={15} weight="bold" aria-hidden="true" />
      </button>
    </section>
  );
}
