import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState } from "react";

import { CLAUDE_CODE_INSTALL_DOCS_URL } from "./cliInstallCommands";
import { aggregateClientConnectors } from "./dashboardHelpers";
import { getClaudeConnector } from "./launcherHelpers";
import type { TrayView } from "./trayHelpers";
import type {
  AppliedPatterns,
  ClaudeCodeProject,
  ClientConnectorStatus,
  HeadroomLearnPrereqStatus,
  HeadroomLearnStatus,
  RuntimeStatus,
} from "./types";

const idleHeadroomLearnStatus: HeadroomLearnStatus = {
  running: false,
  progressPercent: 0,
  summary: "Pick a project to generate learnings.",
  outputTail: [],
};

const idleHeadroomLearnPrereqStatus: HeadroomLearnPrereqStatus = {
  claudeCliAvailable: false,
  claudeCliPath: null,
  codexCliAvailable: false,
  codexCliPath: null,
  codexLoggedIn: false,
};

function delay(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

interface HeadroomLearnControllerOptions {
  activeView: TrayView;
  trayWindowFocused: boolean;
  runtimeStatus: RuntimeStatus | null;
  connectors: ClientConnectorStatus[];
  claudeProjects: ClaudeCodeProject[];
  setClaudeProjects: React.Dispatch<React.SetStateAction<ClaudeCodeProject[]>>;
  refreshClaudeProjects: () => Promise<void>;
  openExternalLink: (url: string) => Promise<void>;
}

export function useHeadroomLearnController({
  activeView,
  trayWindowFocused,
  runtimeStatus,
  connectors,
  claudeProjects,
  setClaudeProjects,
  refreshClaudeProjects,
  openExternalLink,
}: HeadroomLearnControllerOptions) {
  const [showAllClaudeProjects, setShowAllClaudeProjects] = useState(false);
  const [selectedClaudeProjectPath, setSelectedClaudeProjectPath] = useState<
    string | null
  >(null);
  const [headroomLearnStatus, setHeadroomLearnStatus] =
    useState<HeadroomLearnStatus>(idleHeadroomLearnStatus);
  const [optimizeAppliedByProject, setOptimizeAppliedByProject] =
    useState<Record<string, AppliedPatterns> | null>(null);
  const [optimizeAppliedRefreshTick, setOptimizeAppliedRefreshTick] =
    useState(0);
  const previousHeadroomLearnRunningRef = useRef(false);
  const [headroomLearnBusy, setHeadroomLearnBusy] = useState(false);
  const [headroomLearnPrereq, setHeadroomLearnPrereq] =
    useState<HeadroomLearnPrereqStatus>(idleHeadroomLearnPrereqStatus);
  const [learnInstallCopyNotice, setLearnInstallCopyNotice] = useState<
    string | null
  >(null);

  const headroomLearnSupported =
    runtimeStatus?.headroomLearnSupported !== false;
  const headroomLearnDisabledReason =
    runtimeStatus?.headroomLearnDisabledReason ??
    "Headroom Learn is unavailable on this platform.";

  const claudeLearnEnabled = getClaudeConnector(connectors)?.enabled ?? false;
  const codexLearnEnabled = aggregateClientConnectors(connectors).some(
    (connector) => connector.clientId === "codex" && connector.enabled,
  );
  const learnBlurb =
    claudeLearnEnabled && codexLearnEnabled
      ? "Headroom learns from your Claude Code and Codex sessions. When an agent repeats a mistake, Headroom updates that agent's memory so it doesn't happen again."
      : codexLearnEnabled
        ? "Headroom learns from your Codex sessions. When Codex repeats a mistake, Headroom updates your ~/.codex/AGENTS.md and instructions.md so it doesn't happen again."
        : "Headroom helps Claude Code learn from experience. When Claude makes mistakes, Headroom automatically updates the project's MEMORY.md so they don't happen again. You can also ask Headroom to scan past sessions & add token-saving learnings to CLAUDE.md.";

  const sortedClaudeProjects = useMemo(
    () =>
      [...claudeProjects].sort((left, right) => {
        const leftTime = Date.parse(left.lastWorkedAt);
        const rightTime = Date.parse(right.lastWorkedAt);
        return (
          (Number.isNaN(rightTime) ? 0 : rightTime) -
          (Number.isNaN(leftTime) ? 0 : leftTime)
        );
      }),
    [claudeProjects],
  );

  const pinnedClaudeProject =
    !showAllClaudeProjects && headroomLearnStatus.projectPath
      ? (sortedClaudeProjects.find(
          (project) => project.projectPath === headroomLearnStatus.projectPath,
        ) ?? null)
      : null;
  const visibleClaudeProjects = (() => {
    if (showAllClaudeProjects) {
      return sortedClaudeProjects;
    }

    const topProjects = sortedClaudeProjects.slice(0, 3);
    if (
      !pinnedClaudeProject ||
      topProjects.some(
        (project) => project.projectPath === pinnedClaudeProject.projectPath,
      )
    ) {
      return topProjects;
    }
    return [...topProjects, pinnedClaudeProject];
  })();

  const claudeProjectPathsKey = useMemo(
    () =>
      claudeProjects
        .map((project) => project.projectPath)
        .sort()
        .join("\t"),
    [claudeProjects],
  );

  useEffect(() => {
    if (claudeProjects.length === 0) {
      setSelectedClaudeProjectPath(null);
      return;
    }

    setSelectedClaudeProjectPath((current) => {
      if (
        current &&
        claudeProjects.some((project) => project.projectPath === current)
      ) {
        return current;
      }
      return claudeProjects[0].projectPath;
    });
  }, [claudeProjects]);

  async function refreshHeadroomLearnPrereq(force = false) {
    try {
      const status = await invoke<HeadroomLearnPrereqStatus>(
        "get_headroom_learn_prereq_status",
        {
          force,
        },
      );
      setHeadroomLearnPrereq(status);
    } catch {
      setHeadroomLearnPrereq(idleHeadroomLearnPrereqStatus);
    }
  }

  useEffect(() => {
    if (activeView !== "optimization") {
      return;
    }
    void Promise.all([refreshClaudeProjects(), refreshHeadroomLearnPrereq()]);
  }, [activeView]);

  useEffect(() => {
    if (activeView !== "optimization" || !trayWindowFocused) {
      return;
    }

    let active = true;
    const refreshLearnStatus = () => {
      void invoke<HeadroomLearnStatus>("get_headroom_learn_status", {
        projectPath: selectedClaudeProjectPath,
      })
        .then((status) => {
          if (active) {
            setHeadroomLearnStatus(status);
          }
        })
        .catch(() => {
          if (active) {
            setHeadroomLearnStatus((current) => ({
              ...current,
              running: false,
              summary: "Could not read headroom learn status.",
            }));
          }
        });
    };

    refreshLearnStatus();
    const interval = window.setInterval(
      refreshLearnStatus,
      headroomLearnStatus.running ? 900 : 3200,
    );
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [
    activeView,
    selectedClaudeProjectPath,
    headroomLearnStatus.running,
    trayWindowFocused,
  ]);

  useEffect(() => {
    const wasRunning = previousHeadroomLearnRunningRef.current;
    previousHeadroomLearnRunningRef.current = headroomLearnStatus.running;

    if (!wasRunning || headroomLearnStatus.running) {
      return;
    }

    if (headroomLearnStatus.success && headroomLearnStatus.projectPath) {
      const completedAt =
        headroomLearnStatus.lastRunAt ??
        headroomLearnStatus.finishedAt ??
        new Date().toISOString();
      setClaudeProjects((current) =>
        current.map((project) =>
          project.projectPath === headroomLearnStatus.projectPath
            ? {
                ...project,
                lastLearnRanAt: completedAt,
                hasPersistedLearnings: true,
                activeDaysSinceLastLearn: 0,
              }
            : project,
        ),
      );
    }

    void refreshClaudeProjects();
  }, [
    headroomLearnStatus.finishedAt,
    headroomLearnStatus.lastRunAt,
    headroomLearnStatus.projectPath,
    headroomLearnStatus.running,
    headroomLearnStatus.success,
  ]);

  useEffect(() => {
    if (activeView !== "optimization") {
      return;
    }
    const paths =
      claudeProjectPathsKey === "" ? [] : claudeProjectPathsKey.split("\t");
    if (paths.length === 0) {
      setOptimizeAppliedByProject({});
      return;
    }
    let active = true;
    invoke<Record<string, AppliedPatterns>>(
      "list_applied_patterns_for_projects",
      {
        projectPaths: paths,
      },
    )
      .then((result) => {
        if (!active) return;
        setOptimizeAppliedByProject(result);
      })
      .catch(() => {
        if (!active) return;
        setOptimizeAppliedByProject(null);
      });
    return () => {
      active = false;
    };
  }, [
    activeView,
    claudeProjectPathsKey,
    headroomLearnStatus.finishedAt,
    optimizeAppliedRefreshTick,
  ]);

  async function copyLearnInstallCommand(command: string) {
    try {
      if (!navigator.clipboard) {
        throw new Error("Clipboard API unavailable");
      }
      await navigator.clipboard.writeText(command);
      setLearnInstallCopyNotice("Copied install command.");
      window.setTimeout(() => setLearnInstallCopyNotice(null), 2000);
    } catch {
      setLearnInstallCopyNotice(
        "Copy failed. Command remains visible below.",
      );
      window.setTimeout(() => setLearnInstallCopyNotice(null), 3000);
    }
  }

  async function runHeadroomLearn(
    agent: "claude" | "codex",
    projectPath?: string,
  ) {
    if (runtimeStatus?.headroomLearnSupported === false) {
      setHeadroomLearnStatus((current) => ({
        ...current,
        running: false,
        summary: "Headroom Learn is unavailable on this platform.",
        error:
          runtimeStatus.headroomLearnDisabledReason ??
          "Headroom Learn is unavailable on this platform.",
      }));
      return;
    }

    const runKey = agent === "codex" ? "codex" : (projectPath ?? "");
    const displayName =
      agent === "codex"
        ? "Codex sessions"
        : (claudeProjects.find((project) => project.projectPath === projectPath)
            ?.displayName ??
          projectPath ??
          "");
    const startupSummary = `Running headroom learn for ${displayName}.`;
    setHeadroomLearnBusy(true);
    setHeadroomLearnStatus((current) => ({
      ...current,
      running: true,
      projectPath: runKey,
      projectDisplayName: displayName,
      startedAt: new Date().toISOString(),
      finishedAt: null,
      progressPercent: Math.max(8, current.progressPercent || 0),
      summary: startupSummary,
      success: null,
      error: null,
    }));
    try {
      await invoke("start_headroom_learn", {
        agent,
        projectPath: projectPath ?? null,
      });
      for (const waitMs of [180, 350, 650, 900, 1200, 1800, 2400]) {
        await delay(waitMs);
        const status = await invoke<HeadroomLearnStatus>(
          "get_headroom_learn_status",
          {
            projectPath: runKey,
          },
        );
        setHeadroomLearnStatus(status);
        if (!status.running) {
          break;
        }
      }
    } catch (error) {
      setHeadroomLearnStatus((current) => ({
        ...current,
        running: false,
        summary: "headroom learn could not be started.",
        error:
          error instanceof Error
            ? error.message
            : "Failed to start headroom learn.",
      }));
    } finally {
      setHeadroomLearnBusy(false);
    }
  }

  async function handleRunHeadroomLearn(
    agent: "claude" | "codex",
    projectPath?: string,
  ) {
    if (agent === "claude" && projectPath) {
      setSelectedClaudeProjectPath(projectPath);
    }
    try {
      const status = await invoke<HeadroomLearnPrereqStatus>(
        "get_headroom_learn_prereq_status",
      );
      setHeadroomLearnPrereq(status);
      const ready =
        agent === "codex"
          ? status.codexCliAvailable && status.codexLoggedIn
          : status.claudeCliAvailable;
      if (!ready) {
        return;
      }
    } catch {
      setHeadroomLearnPrereq(idleHeadroomLearnPrereqStatus);
      return;
    }
    await runHeadroomLearn(agent, projectPath);
  }

  async function openLearnInstallDocsLink() {
    try {
      await openExternalLink(CLAUDE_CODE_INSTALL_DOCS_URL);
    } catch (error) {
      setLearnInstallCopyNotice(
        error instanceof Error
          ? error.message
          : "Could not open the install guide.",
      );
      window.setTimeout(() => setLearnInstallCopyNotice(null), 3000);
    }
  }

  return {
    claudeLearnEnabled,
    codexLearnEnabled,
    copyLearnInstallCommand,
    handleRunHeadroomLearn,
    headroomLearnBusy,
    headroomLearnDisabledReason,
    headroomLearnPrereq,
    headroomLearnStatus,
    headroomLearnSupported,
    learnBlurb,
    learnInstallCopyNotice,
    openLearnInstallDocsLink,
    optimizeAppliedByProject,
    refreshHeadroomLearnPrereq,
    setOptimizeAppliedRefreshTick,
    setShowAllClaudeProjects,
    showAllClaudeProjects,
    sortedClaudeProjects,
    visibleClaudeProjects,
  };
}
