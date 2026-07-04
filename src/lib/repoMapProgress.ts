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

export type RepoMapProgressState = "pending" | "running" | "ok" | "warning" | "error";

export interface RepoMapProgressStep {
  id: string;
  label: string;
  state: RepoMapProgressState;
  detail: string;
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
      state = "running";
    }

    steps.push({
      id,
      label,
      state,
      detail:
        run?.remediation ??
        run?.detail ??
        (state === "running" ? "Queued or running" : state === "pending" ? "Waiting" : "Completed"),
    });
  }

  return steps;
}
