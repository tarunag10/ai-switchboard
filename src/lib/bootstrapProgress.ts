import type { BootstrapProgress } from "./types";

export interface BootstrapProgressTiming {
  stepStartedAtMs: number | null;
  stepEtaSeedSeconds: number;
  stepBasePercent: number;
  nowMs?: number;
}

export interface BootstrapEtaCopyInput extends BootstrapProgressTiming {
  showInstallProgress: boolean;
  currentStepEtaSeconds: number;
  progress: BootstrapProgress;
}

export function bootstrapStepPercentSpan(step: string) {
  switch (step) {
    case "Preparing install":
      return 13;
    case "Downloading Python":
      return 13;
    case "Creating environment":
      return 17;
    case "Installing Headroom":
      return 20;
    case "Finalizing":
      return 4;
    default:
      return 8;
  }
}

export function bootstrapStepProgress(
  progress: BootstrapProgress,
  timing: BootstrapProgressTiming,
) {
  if (progress.complete) {
    return 1;
  }
  if (!progress.running || !timing.stepStartedAtMs) {
    return 0;
  }

  const nowMs = timing.nowMs ?? Date.now();
  const elapsedSeconds = Math.max(0, (nowMs - timing.stepStartedAtMs) / 1000);
  const eta = Math.max(
    8,
    timing.stepEtaSeedSeconds || progress.currentStepEtaSeconds || 20,
  );
  const linear = Math.min(0.96, elapsedSeconds / eta);

  if (elapsedSeconds <= eta) {
    return linear;
  }

  const overtime = elapsedSeconds - eta;
  return Math.min(0.995, linear + overtime / (eta * 10));
}

export function animatedBootstrapOverallPercent(
  progress: BootstrapProgress,
  timing: BootstrapProgressTiming,
) {
  if (progress.complete || progress.failed || !progress.running) {
    return progress.overallPercent;
  }

  const span = bootstrapStepPercentSpan(progress.currentStep);
  const animated =
    timing.stepBasePercent + span * bootstrapStepProgress(progress, timing);
  return Math.min(99, Math.max(progress.overallPercent, animated));
}

export function bootstrapEtaCopy(input: BootstrapEtaCopyInput) {
  const { progress } = input;
  if (!input.showInstallProgress) {
    return "ETA: starts after install";
  }
  if (progress.complete) {
    return "ETA: complete";
  }
  if (progress.failed) {
    return "ETA: unavailable";
  }

  const nowMs = input.nowMs ?? Date.now();
  const elapsedSeconds = input.stepStartedAtMs
    ? Math.max(0, Math.round((nowMs - input.stepStartedAtMs) / 1000))
    : 0;
  const baselineEta = Math.max(
    input.stepEtaSeedSeconds,
    input.currentStepEtaSeconds,
  );
  const remainingSeconds = Math.max(0, baselineEta - elapsedSeconds);

  if (remainingSeconds <= 0 && progress.running) {
    return "ETA: finishing up";
  }
  if (remainingSeconds <= 0) {
    return "ETA: --";
  }
  if (remainingSeconds < 60) {
    return `ETA: ${remainingSeconds}s`;
  }
  const mins = Math.floor(remainingSeconds / 60);
  const secs = remainingSeconds % 60;
  return `ETA: ${mins}m ${secs}s`;
}
