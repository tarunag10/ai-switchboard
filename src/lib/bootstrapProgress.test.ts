import { describe, expect, it } from "vitest";

import {
  animatedBootstrapOverallPercent,
  bootstrapEtaCopy,
  bootstrapStepPercentSpan,
  bootstrapStepProgress,
} from "./bootstrapProgress";
import type { BootstrapProgress } from "./types";

const runningProgress: BootstrapProgress = {
  running: true,
  complete: false,
  failed: false,
  currentStep: "Installing Headroom",
  message: "Installing",
  currentStepEtaSeconds: 20,
  overallPercent: 40,
};

describe("bootstrap progress helpers", () => {
  it("maps known install steps to weighted progress spans", () => {
    expect(bootstrapStepPercentSpan("Preparing install")).toBe(13);
    expect(bootstrapStepPercentSpan("Installing Headroom")).toBe(20);
    expect(bootstrapStepPercentSpan("Finalizing")).toBe(4);
    expect(bootstrapStepPercentSpan("Unknown")).toBe(8);
  });

  it("animates running step progress without moving completed or stopped work", () => {
    expect(
      bootstrapStepProgress(runningProgress, {
        stepStartedAtMs: 1_000,
        stepEtaSeedSeconds: 20,
        stepBasePercent: 30,
        nowMs: 11_000,
      }),
    ).toBe(0.5);

    expect(
      bootstrapStepProgress(
        { ...runningProgress, running: false },
        {
          stepStartedAtMs: 1_000,
          stepEtaSeedSeconds: 20,
          stepBasePercent: 30,
          nowMs: 11_000,
        },
      ),
    ).toBe(0);
  });

  it("caps animated overall percent below completion", () => {
    expect(
      animatedBootstrapOverallPercent(runningProgress, {
        stepStartedAtMs: 1_000,
        stepEtaSeedSeconds: 20,
        stepBasePercent: 95,
        nowMs: 40_000,
      }),
    ).toBe(99);

    expect(
      animatedBootstrapOverallPercent(
        { ...runningProgress, complete: true, overallPercent: 100 },
        {
          stepStartedAtMs: 1_000,
          stepEtaSeedSeconds: 20,
          stepBasePercent: 95,
          nowMs: 40_000,
        },
      ),
    ).toBe(100);
  });

  it("formats ETA copy for hidden, failed, short, long, and overtime states", () => {
    const base = {
      progress: runningProgress,
      stepStartedAtMs: 1_000,
      stepEtaSeedSeconds: 120,
      stepBasePercent: 40,
      currentStepEtaSeconds: 120,
      nowMs: 31_000,
    };

    expect(bootstrapEtaCopy({ ...base, showInstallProgress: false })).toBe(
      "ETA: starts after install",
    );
    expect(
      bootstrapEtaCopy({
        ...base,
        showInstallProgress: true,
        currentStepEtaSeconds: 45,
        stepEtaSeedSeconds: 45,
      }),
    ).toBe("ETA: 15s");
    expect(bootstrapEtaCopy({ ...base, showInstallProgress: true })).toBe(
      "ETA: 1m 30s",
    );
    expect(
      bootstrapEtaCopy({
        ...base,
        showInstallProgress: true,
        progress: { ...runningProgress, failed: true },
      }),
    ).toBe("ETA: unavailable");
    expect(
      bootstrapEtaCopy({
        ...base,
        showInstallProgress: true,
        stepEtaSeedSeconds: 10,
        currentStepEtaSeconds: 10,
        nowMs: 20_000,
      }),
    ).toBe("ETA: finishing up");
  });
});
