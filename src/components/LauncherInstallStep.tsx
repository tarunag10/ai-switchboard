import type { MouseEvent } from "react";
import { Copy } from "@phosphor-icons/react";

import {
  animatedBootstrapOverallPercent,
  bootstrapEtaCopy,
  bootstrapStepProgress,
} from "../lib/bootstrapProgress";
import type { BootstrapProgress, RuntimeStatus } from "../lib/types";
import { LauncherShell } from "./LauncherShell";

interface LauncherInstallStepProps {
  appSemver: string;
  bootstrapping: boolean;
  bootstrapError: string | null;
  bootstrapProgress: BootstrapProgress;
  bootstrapComplete: boolean;
  copyFirstRunFootprint: () => void | Promise<void>;
  handleBootstrap: () => void | Promise<void>;
  handleFirstLaunchContinue: () => void | Promise<void>;
  onMouseDown: (event: MouseEvent<HTMLElement>) => void;
  onboardingFootprintCopyNotice: string | null;
  runtimeStatus: RuntimeStatus | null;
  showInstallProgress: boolean;
  stepBasePercent: number;
  stepEtaSeedSeconds: number;
  stepStartedAtMs: number | null;
}

export function LauncherInstallStep({
  appSemver,
  bootstrapping,
  bootstrapError,
  bootstrapProgress,
  bootstrapComplete,
  copyFirstRunFootprint,
  handleBootstrap,
  handleFirstLaunchContinue,
  onMouseDown,
  onboardingFootprintCopyNotice,
  runtimeStatus,
  showInstallProgress,
  stepBasePercent,
  stepEtaSeedSeconds,
  stepStartedAtMs,
}: LauncherInstallStepProps) {
  const stepProgress = Math.round(
    bootstrapStepProgress(bootstrapProgress, {
      stepBasePercent,
      stepEtaSeedSeconds,
      stepStartedAtMs,
    }) * 100,
  );
  const renderPercent = animatedBootstrapOverallPercent(bootstrapProgress, {
    stepBasePercent,
    stepEtaSeedSeconds,
    stepStartedAtMs,
  });
  const installComplete = bootstrapProgress.complete || bootstrapComplete;
  const statusCopy = showInstallProgress
    ? `${bootstrapProgress.message} ${
        bootstrapProgress.running && !bootstrapProgress.complete
          ? `(${stepProgress}% of this step)`
          : ""
      }`.trim()
    : "";

  return (
    <LauncherShell
      shellClassName="intro-shell"
      spinnerClassName="intro-shell__spinner"
      copyClassName="intro-shell__copy intro-shell__copy--first-run"
      onMouseDown={onMouseDown}
      version={appSemver}
      showSpinner={bootstrapping}
    >
      <h1>AI Switchboard keeps coding-agent work lean, local, and reversible.</h1>
      <div className="intro-shell__checklist">
        <article>
          <strong>Local-first</strong>
          <p>
            Routing, client setup, Doctor repairs, and add-ons run on your Mac.
            Model calls still go to your normal provider accounts.
          </p>
        </article>
        <article>
          <strong>Self-contained runtime</strong>
          <p>
            Installs Headroom helper tools into app-owned storage without
            changing your system Python.
          </p>
        </article>
        <article>
          <strong>Managed local files</strong>
          <p>
            May write app storage, shell profile blocks, Claude settings or
            hooks, Codex provider blocks, and recovery backups with managed
            markers.
          </p>
        </article>
        <article>
          <strong>Off means off</strong>
          <p>
            Switchboard can remove routing hooks, and Doctor can repair stale
            local setup if a client or proxy drifts.
          </p>
        </article>
        <article>
          <strong>Privacy and network</strong>
          <p>
            Local-free builds do not require telemetry or accounts. Provider
            model calls still leave your Mac through Claude, OpenAI, or the
            provider you choose.
          </p>
        </article>
        <article>
          <strong>Choose initial mode later</strong>
          <p>
            Start in Off, RTK only, Headroom only, or Full optimization after
            install; managed routing is not required to finish onboarding.
          </p>
        </article>
      </div>
      {installComplete ? (
        <>
          {runtimeStatus?.running !== true ? (
            <>
              <p className="launcher-install-notice">
                Starting the local Headroom engine for the first time (this can
                take 1-2 minutes)…
              </p>
              <button
                className="primary-button primary-button--large primary-button--install launcher-step1-continue"
                disabled
                type="button"
              >
                Starting engine…
              </button>
            </>
          ) : (
            <>
              <p className="launcher-install-notice">
                Local switchboard runtime is ready
              </p>
              <button
                className="primary-button primary-button--large primary-button--success launcher-step1-continue"
                onClick={() => void handleFirstLaunchContinue()}
                type="button"
              >
                Continue
              </button>
            </>
          )}
        </>
      ) : (
        <>
          {!bootstrapping && (
            <p className="install-pre-notice">
              Takes a minute or two to install the local engine.
            </p>
          )}
          <button
            className="primary-button primary-button--large primary-button--install"
            disabled={bootstrapping}
            onClick={() => void handleBootstrap()}
            type="button"
          >
            {bootstrapping
              ? "Installing local engine…"
              : bootstrapProgress.failed
                ? "Try again"
                : "Install AI Switchboard for Mac"}
          </button>
          {!bootstrapping && (
            <div className="install-disclosure">
              <p className="install-disclosure__lead">Clicking Install will:</p>
              <ul className="install-disclosure__list">
                <li>
                  Download a self-contained Python runtime (~2 GB) to{" "}
                  <code>~/.headroom</code>. Your system Python is untouched.
                </li>
                <li>
                  Ask before routing supported coding clients through the local
                  proxy: Claude Code via <code>ANTHROPIC_BASE_URL</code> and{" "}
                  <code>~/.claude/settings.json</code>; Codex via{" "}
                  <code>OPENAI_BASE_URL</code> and a managed provider block in{" "}
                  <code>~/.codex/config.toml</code>.
                </li>
                <li>
                  Write timestamped backups before local config edits. Off mode
                  removes routing hooks; Doctor can re-apply or repair stale
                  setup.
                </li>
                <li>
                  Keep RTK, Ponytail, MarkItDown, and future Repo Intelligence
                  as optional add-ons you control separately.
                </li>
              </ul>
              <button
                className="secondary-button secondary-button--small install-disclosure__copy"
                onClick={() => void copyFirstRunFootprint()}
                type="button"
              >
                <Copy aria-hidden="true" weight="bold" />
                <span>{onboardingFootprintCopyNotice ?? "Copy footprint"}</span>
              </button>
            </div>
          )}
        </>
      )}
      <div className="install-progress-shell">
        {showInstallProgress ? (
          <div className="install-progress" aria-live="polite">
            <div className="install-progress__bar-track">
              <div
                className="install-progress__bar-fill"
                style={{ width: `${renderPercent}%` }}
              />
            </div>
            <div className="install-progress__meta">
              <p>{statusCopy}</p>
              <span>
                {bootstrapEtaCopy({
                  currentStepEtaSeconds:
                    bootstrapProgress.currentStepEtaSeconds,
                  progress: bootstrapProgress,
                  showInstallProgress,
                  stepBasePercent,
                  stepEtaSeedSeconds,
                  stepStartedAtMs,
                })}
              </span>
            </div>
            {bootstrapError ? (
              <p className="install-progress__error">{bootstrapError}</p>
            ) : null}
          </div>
        ) : null}
      </div>
    </LauncherShell>
  );
}
