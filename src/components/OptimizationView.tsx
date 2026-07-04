import { ArrowClockwise, Brain, Terminal } from "@phosphor-icons/react";
import type {
  ClaudeCodeProject,
  HeadroomLearnPrereqStatus,
  HeadroomLearnStatus,
  AppliedPatterns,
} from "../lib/types";
import type { TrayView } from "../lib/trayHelpers";
import { formatLearnStatus } from "../lib/dashboardHelpers";
import { OptimizationDashboard } from "./OptimizationDashboard";
import { OptimizePanel } from "./OptimizePanel";

const CLAUDE_CODE_INSTALL_CURL_CMD =
  "curl -fsSL https://claude.ai/install.sh | bash";
const CODEX_CLI_INSTALL_CMD = "npm install -g @openai/codex";
const CODEX_CLI_LOGIN_CMD = "codex login";
const CODEX_INSTALL_DOCS_URL = "https://developers.openai.com/codex/cli";

export interface OptimizationViewProps {
  activeView: TrayView;
  setActiveView: (view: TrayView) => void;
  headroomLearnSupported: boolean;
  headroomLearnDisabledReason: string | null;
  headroomLearnPrereq: HeadroomLearnPrereqStatus;
  headroomLearnStatus: HeadroomLearnStatus;
  headroomLearnBusy: boolean;
  claudeLearnEnabled: boolean;
  codexLearnEnabled: boolean;
  claudeProjectsBusy: boolean;
  claudeProjects: ClaudeCodeProject[];
  visibleClaudeProjects: ClaudeCodeProject[];
  sortedClaudeProjects: ClaudeCodeProject[];
  showAllClaudeProjects: boolean;
  setShowAllClaudeProjects: React.Dispatch<React.SetStateAction<boolean>>;
  handleRunHeadroomLearn: (
    source: "claude" | "codex",
    projectPath?: string,
  ) => Promise<void>;
  copyLearnInstallCommand: (command: string) => Promise<void>;
  openLearnInstallDocsLink: () => Promise<void>;
  refreshHeadroomLearnPrereq: (force?: boolean) => Promise<void>;
  learnInstallCopyNotice: string | null;
  optimizeAppliedByProject: Record<string, AppliedPatterns> | null | undefined;
  setOptimizeAppliedRefreshTick: React.Dispatch<
    React.SetStateAction<number>
  >;
  claudeProjectsError: string | null;
  learnBlurb: string;
}

export function OptimizationView({
  activeView,
  setActiveView,
  headroomLearnSupported,
  headroomLearnDisabledReason,
  headroomLearnPrereq,
  headroomLearnStatus,
  headroomLearnBusy,
  claudeLearnEnabled,
  codexLearnEnabled,
  claudeProjectsBusy,
  claudeProjects,
  visibleClaudeProjects,
  sortedClaudeProjects,
  showAllClaudeProjects,
  setShowAllClaudeProjects,
  handleRunHeadroomLearn,
  copyLearnInstallCommand,
  openLearnInstallDocsLink,
  refreshHeadroomLearnPrereq,
  learnInstallCopyNotice,
  optimizeAppliedByProject,
  setOptimizeAppliedRefreshTick,
  claudeProjectsError,
  learnBlurb,
}: OptimizationViewProps) {
  return (
    <div className="tray-content" hidden={activeView !== "optimization"}>
      <article className="soft-card optimize-card">
        <header className="optimize-card__head">
          <div className="optimize-card__title-row">
            <span className="optimize-card__title-icon" aria-hidden="true">
              <Brain weight="duotone" />
            </span>
            <h1>Project learnings</h1>
          </div>
          <p className="optimize-card__blurb">{learnBlurb}</p>
        </header>
        <div className="optimize-card__body">
          <OptimizationDashboard />
          <div className="optimize-learn-setup" role="note">
            <strong>Where this lives</strong>
            <span>
              Enable Claude Code or Codex in Addons, then return here and
              run a visible scan button for the project or session history.
            </span>
          </div>
          {!headroomLearnSupported ? (
            <div className="optimize-minimal">
              <p className="optimize-minimal__meta">
                {headroomLearnDisabledReason}
              </p>
              <p className="optimize-minimal__meta">
                Linux preview currently supports the core Headroom proxy,
                Claude Code routing, and RTK activity tracking.
              </p>
            </div>
          ) : !claudeLearnEnabled && !codexLearnEnabled ? (
            <div className="optimize-empty-action">
              <p className="loading-copy">
                No learning source is enabled yet. Turn on the Claude Code
                or Codex connector in Addons, then the scan controls appear
                here.
              </p>
              <button
                type="button"
                className="secondary-button secondary-button--small"
                onClick={() => setActiveView("addons")}
              >
                Open Addons
              </button>
            </div>
          ) : (
            <div className="optimize-minimal">
              {claudeLearnEnabled &&
              claudeProjectsBusy &&
              claudeProjects.length === 0 ? (
                <p className="loading-copy">Loading projects…</p>
              ) : claudeLearnEnabled && claudeProjects.length === 0 ? (
                <p className="loading-copy">
                  No Claude Code projects found in{" "}
                  <code>~/.claude/projects</code>.
                </p>
              ) : claudeLearnEnabled ? (
                <>
                  {!headroomLearnPrereq.claudeCliAvailable ? (
                    <div className="install-prompt" role="status">
                      <header className="install-prompt__head">
                        <span
                          className="install-prompt__icon"
                          aria-hidden="true"
                        >
                          <Terminal weight="duotone" />
                        </span>
                        <div className="install-prompt__head-text">
                          <h2 className="install-prompt__title">
                            Install the Claude Code CLI
                          </h2>
                          <p className="install-prompt__body">
                            Headroom Learn uses the <code>claude</code> CLI
                            to analyze your sessions.
                          </p>
                        </div>
                      </header>
                      <div className="install-prompt__cmd">
                        <code className="install-prompt__cmd-text">
                          {CLAUDE_CODE_INSTALL_CURL_CMD}
                        </code>
                        <button
                          className="install-prompt__cmd-copy"
                          type="button"
                          onClick={() =>
                            void copyLearnInstallCommand(
                              CLAUDE_CODE_INSTALL_CURL_CMD,
                            )
                          }
                        >
                          Copy
                        </button>
                      </div>
                      <div className="install-prompt__foot">
                        <button
                          className="install-prompt__link"
                          type="button"
                          onClick={() => void openLearnInstallDocsLink()}
                        >
                          Open install docs
                        </button>
                        <span
                          className="install-prompt__foot-sep"
                          aria-hidden="true"
                        >
                          ·
                        </span>
                        <button
                          className="install-prompt__link install-prompt__link--recheck"
                          type="button"
                          onClick={() =>
                            void refreshHeadroomLearnPrereq(true)
                          }
                        >
                          <ArrowClockwise
                            weight="bold"
                            size={12}
                            aria-hidden="true"
                          />
                          Re-check
                        </button>
                        {learnInstallCopyNotice ? (
                          <span className="install-prompt__notice">
                            {learnInstallCopyNotice}
                          </span>
                        ) : null}
                      </div>
                    </div>
                  ) : null}
                  <div className="optimize-projects">
                    {visibleClaudeProjects.map((project) => {
                      const isRunning =
                        headroomLearnStatus.running &&
                        headroomLearnStatus.projectPath ===
                          project.projectPath;
                      const isLatestLearnProject =
                        headroomLearnStatus.projectPath ===
                        project.projectPath;
                      const disableLearn =
                        !headroomLearnPrereq.claudeCliAvailable ||
                        headroomLearnBusy ||
                        claudeProjectsBusy ||
                        (headroomLearnStatus.running && !isRunning);
                      const learnMeta = formatLearnStatus(project);
                      const projectResultTone =
                        headroomLearnStatus.success === true
                          ? "success"
                          : headroomLearnStatus.success === false ||
                              headroomLearnStatus.error
                            ? "failure"
                            : "idle";
                      const projectResultLabel =
                        headroomLearnStatus.success === true
                          ? "Run succeeded"
                          : headroomLearnStatus.success === false ||
                              headroomLearnStatus.error
                            ? "Last run failed"
                            : "No completed run yet";
                      const showInlineResult =
                        isLatestLearnProject &&
                        !headroomLearnStatus.running &&
                        (headroomLearnStatus.success !== null ||
                          Boolean(headroomLearnStatus.error) ||
                          headroomLearnStatus.outputTail.length > 0);
                      return (
                        <div
                          className={`optimize-project-row${isRunning || showInlineResult ? " optimize-project-row--active" : ""}`}
                          key={project.id}
                        >
                          <div className="optimize-project-row__main">
                            <span className="optimize-project-row__name">
                              <strong>{project.displayName}</strong>
                              <small>
                                <span
                                  className="optimize-project-row__training"
                                  aria-live="polite"
                                >
                                  {isRunning
                                    ? `Scanning sessions${
                                        typeof headroomLearnStatus.elapsedSeconds ===
                                        "number"
                                          ? ` · ${headroomLearnStatus.elapsedSeconds}s`
                                          : ""
                                      }`
                                    : learnMeta}
                                </span>
                                <OptimizePanel
                                  projectPath={project.projectPath}
                                  refreshSignal={
                                    isLatestLearnProject &&
                                    !headroomLearnStatus.running
                                      ? Date.parse(
                                          headroomLearnStatus.finishedAt ??
                                            "",
                                        ) || 0
                                      : 0
                                  }
                                  preloadedApplied={
                                    optimizeAppliedByProject
                                      ? (optimizeAppliedByProject[
                                          project.projectPath
                                        ] ?? {
                                          claudeMd: [],
                                          memoryMd: [],
                                        })
                                      : undefined
                                  }
                                  onAppliedMutated={() =>
                                    setOptimizeAppliedRefreshTick(
                                      (tick) => tick + 1,
                                    )
                                  }
                                />
                              </small>
                            </span>
                            <div className="optimize-project-row__actions">
                              <button
                                type="button"
                                className={`secondary-button secondary-button--small optimize-project-row__scan${isRunning ? " is-spinning" : ""}`}
                                onClick={() =>
                                  void handleRunHeadroomLearn(
                                    "claude",
                                    project.projectPath,
                                  )
                                }
                                disabled={disableLearn}
                              >
                                <ArrowClockwise
                                  weight="bold"
                                  size={12}
                                  aria-hidden="true"
                                />
                                {isRunning
                                  ? "Scanning"
                                  : "Scan Claude project"}
                              </button>
                              {showInlineResult ? (
                                <span
                                  className={`optimize-project-row__status optimize-minimal__result--${projectResultTone}`}
                                >
                                  {projectResultLabel}
                                </span>
                              ) : null}
                            </div>
                          </div>
                          {showInlineResult && headroomLearnStatus.error ? (
                            <div className="optimize-project-row__result">
                              <p className="install-progress__error">
                                {headroomLearnStatus.error}
                              </p>
                            </div>
                          ) : null}
                        </div>
                      );
                    })}
                  </div>
                  {sortedClaudeProjects.length > 3 ? (
                    <button
                      className="optimize-minimal__inline-action optimize-projects__toggle"
                      onClick={() =>
                        setShowAllClaudeProjects((current) => !current)
                      }
                      type="button"
                    >
                      {showAllClaudeProjects
                        ? "fewer projects"
                        : "more projects"}
                    </button>
                  ) : null}
                </>
              ) : null}
              {codexLearnEnabled
                ? (() => {
                    const codexReady =
                      headroomLearnPrereq.codexCliAvailable &&
                      headroomLearnPrereq.codexLoggedIn;
                    const codexRunning =
                      headroomLearnStatus.running &&
                      headroomLearnStatus.projectPath === "codex";
                    const codexIsLatest =
                      headroomLearnStatus.projectPath === "codex";
                    const codexDisable =
                      !codexReady ||
                      headroomLearnBusy ||
                      (headroomLearnStatus.running && !codexRunning);
                    const codexShowResult =
                      codexIsLatest &&
                      !headroomLearnStatus.running &&
                      (headroomLearnStatus.success !== null ||
                        Boolean(headroomLearnStatus.error) ||
                        headroomLearnStatus.outputTail.length > 0);
                    const codexResultTone =
                      headroomLearnStatus.success === true
                        ? "success"
                        : headroomLearnStatus.success === false ||
                            headroomLearnStatus.error
                          ? "failure"
                          : "idle";
                    const codexResultLabel =
                      headroomLearnStatus.success === true
                        ? "Run succeeded"
                        : headroomLearnStatus.success === false ||
                            headroomLearnStatus.error
                          ? "Last run failed"
                          : "No completed run yet";
                    if (!codexReady) {
                      const codexCmd = headroomLearnPrereq.codexCliAvailable
                        ? CODEX_CLI_LOGIN_CMD
                        : CODEX_CLI_INSTALL_CMD;
                      return (
                        <div className="install-prompt" role="status">
                          <header className="install-prompt__head">
                            <span
                              className="install-prompt__icon"
                              aria-hidden="true"
                            >
                              <Terminal weight="duotone" />
                            </span>
                            <div className="install-prompt__head-text">
                              <h2 className="install-prompt__title">
                                {headroomLearnPrereq.codexCliAvailable
                                  ? "Sign in to the Codex CLI"
                                  : "Install the Codex CLI"}
                              </h2>
                              <p className="install-prompt__body">
                                Headroom Learn analyzes your Codex sessions
                                with the <code>codex</code> CLI on your
                                ChatGPT subscription.
                                {headroomLearnPrereq.codexCliAvailable
                                  ? " Sign in to continue."
                                  : ""}
                              </p>
                            </div>
                          </header>
                          <div className="install-prompt__cmd">
                            <code className="install-prompt__cmd-text">
                              {codexCmd}
                            </code>
                            <button
                              className="install-prompt__cmd-copy"
                              type="button"
                              onClick={() =>
                                void copyLearnInstallCommand(codexCmd)
                              }
                            >
                              Copy
                            </button>
                          </div>
                          <div className="install-prompt__foot">
                            <button
                              className="install-prompt__link"
                              type="button"
                              onClick={() =>
                                void openLearnInstallDocsLink()
                              }
                            >
                              Open install docs
                            </button>
                            <span
                              className="install-prompt__foot-sep"
                              aria-hidden="true"
                            >
                              ·
                            </span>
                            <button
                              className="install-prompt__link install-prompt__link--recheck"
                              type="button"
                              onClick={() =>
                                void refreshHeadroomLearnPrereq(true)
                              }
                            >
                              <ArrowClockwise
                                weight="bold"
                                size={12}
                                aria-hidden="true"
                              />
                              Re-check
                            </button>
                            {learnInstallCopyNotice ? (
                              <span className="install-prompt__notice">
                                {learnInstallCopyNotice}
                              </span>
                            ) : null}
                          </div>
                        </div>
                      );
                    }
                    return (
                      <div className="optimize-projects">
                        <div
                          className={`optimize-project-row${codexRunning || codexShowResult ? " optimize-project-row--active" : ""}`}
                        >
                          <div className="optimize-project-row__main">
                            <span className="optimize-project-row__name">
                              <strong>Codex sessions</strong>
                              <small>
                                <span
                                  className="optimize-project-row__training"
                                  aria-live="polite"
                                >
                                  {codexRunning
                                    ? `Scanning sessions${
                                        typeof headroomLearnStatus.elapsedSeconds ===
                                        "number"
                                          ? ` · ${headroomLearnStatus.elapsedSeconds}s`
                                          : ""
                                      }`
                                    : "Scans ~/.codex/sessions into AGENTS.md"}
                                </span>
                              </small>
                            </span>
                            <div className="optimize-project-row__actions">
                              <button
                                type="button"
                                className={`secondary-button secondary-button--small optimize-project-row__scan${codexRunning ? " is-spinning" : ""}`}
                                onClick={() =>
                                  void handleRunHeadroomLearn("codex")
                                }
                                disabled={codexDisable}
                              >
                                <ArrowClockwise
                                  weight="bold"
                                  size={12}
                                  aria-hidden="true"
                                />
                                {codexRunning
                                  ? "Scanning"
                                  : "Scan Codex sessions"}
                              </button>
                              {codexShowResult ? (
                                <span
                                  className={`optimize-project-row__status optimize-minimal__result--${codexResultTone}`}
                                >
                                  {codexResultLabel}
                                </span>
                              ) : null}
                            </div>
                          </div>
                          {codexShowResult && headroomLearnStatus.error ? (
                            <div className="optimize-project-row__result">
                              <p className="install-progress__error">
                                {headroomLearnStatus.error}
                              </p>
                            </div>
                          ) : null}
                        </div>
                      </div>
                    );
                  })()
                : null}
            </div>
          )}
          {claudeProjectsError ? (
            <p className="install-progress__error">{claudeProjectsError}</p>
          ) : null}
          {headroomLearnStatus.error &&
          headroomLearnStatus.projectPath !== "codex" &&
          !claudeProjects.some(
            (project) =>
              project.projectPath === headroomLearnStatus.projectPath,
          ) ? (
            <p className="install-progress__error">
              {headroomLearnStatus.error}
            </p>
          ) : null}
        </div>
      </article>
    </div>
  );
}
