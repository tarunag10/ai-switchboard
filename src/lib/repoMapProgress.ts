export type RepoMapToolStatus = "ok" | "warning" | "not-run";

export interface RepoMapToolRunLike {
  status?: RepoMapToolStatus;
  detail?: string | null;
  remediation?: string | null;
}

export interface RepoMapPreflightToolLike {
  label: string;
  available: boolean;
  installHint?: string | null;
}

export type RepoMapProgressState =
  | "pending"
  | "queued"
  | "running"
  | "ok"
  | "warning"
  | "error";

export interface RepoMapProgressStep {
  id: string;
  label: string;
  state: RepoMapProgressState;
  detail: string;
}

export interface RepoMapProgressSummary {
  percent: number;
  completed: number;
  total: number;
  currentToolId: string | null;
  state: "idle" | "running" | "complete" | "warning" | "error";
}

const toolLabels = [
  ["graphify", "Graphify"],
  ["madge", "Madge"],
  ["dependencyCruiser", "dependency-cruiser"],
  ["cargoMetadata", "Cargo metadata"],
  ["tauriScan", "Tauri scan"],
] as const;

export function buildRepoMapProgressSteps(options: {
  generateBusy: boolean;
  generateError?: string | null;
  preflightTools?: RepoMapPreflightToolLike[] | null;
  toolRuns?: Record<string, RepoMapToolRunLike | undefined> | null;
  currentToolId?: string | null;
}): RepoMapProgressStep[] {
  const missingTools = options.preflightTools?.filter((tool) => !tool.available) ?? [];
  const hasToolRuns = Boolean(options.toolRuns && Object.keys(options.toolRuns).length > 0);

  const preflightState: RepoMapProgressState =
    options.generateError && !hasToolRuns ? "error" : missingTools.length > 0 ? "warning" : "ok";

  const steps: RepoMapProgressStep[] = [
    {
      id: "preflight",
      label: "Preflight",
      state: options.generateBusy && !hasToolRuns ? "running" : preflightState,
      detail:
        missingTools.length > 0
          ? `${missingTools.length} missing tool${missingTools.length === 1 ? "" : "s"}`
          : "Tooling ready",
    },
  ];

  for (const [id, label] of toolLabels) {
    const run = options.toolRuns?.[id];
    let state: RepoMapProgressState = "pending";
    if (run?.status === "ok") {
      state = "ok";
    } else if (run?.status === "warning") {
      state = "warning";
    } else if (options.generateError && !run) {
      state = "error";
    } else if (options.generateBusy && !run) {
      const firstPending = toolLabels.find(([toolId]) => !options.toolRuns?.[toolId]);
      state =
        options.currentToolId === id || (!options.currentToolId && firstPending?.[0] === id)
          ? "running"
          : "queued";
    }

    steps.push({
      id,
      label,
      state,
      detail:
        run?.remediation ??
        run?.detail ??
        (state === "running"
          ? "Running"
          : state === "queued"
            ? "Queued"
            : state === "pending"
              ? "Waiting"
              : "Completed"),
    });
  }

  return steps;
}

export function buildRepoMapProgressSummary(
  steps: RepoMapProgressStep[],
  options: {
    generateBusy: boolean;
    generateError?: string | null;
    currentToolId?: string | null;
    progressPercent?: number | null;
    completedTools?: number | null;
    totalTools?: number | null;
  },
): RepoMapProgressSummary {
  const toolSteps = steps.filter((step) => step.id !== "preflight");
  const completed =
    options.completedTools ??
    toolSteps.filter((step) => step.state === "ok" || step.state === "warning").length;
  const total = Math.max(options.totalTools ?? toolSteps.length, toolSteps.length, 1);
  const fallbackPercent = Math.round(Math.min(1, completed / total) * 100);
  const percent = Math.max(
    0,
    Math.min(100, options.progressPercent ?? fallbackPercent),
  );
  const currentToolId =
    options.currentToolId ??
    toolSteps.find((step) => step.state === "running")?.id ??
    null;
  let state: RepoMapProgressSummary["state"] = "idle";
  if (options.generateError || steps.some((step) => step.state === "error")) {
    state = "error";
  } else if (toolSteps.some((step) => step.state === "warning")) {
    state = options.generateBusy ? "running" : "warning";
  } else if (options.generateBusy) {
    state = "running";
  } else if (completed > 0 && completed >= total) {
    state = "complete";
  }
  return { percent, completed, total, currentToolId, state };
}
