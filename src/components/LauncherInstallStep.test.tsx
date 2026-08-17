import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { LauncherInstallStep } from "./LauncherInstallStep";
import type { BootstrapProgress } from "../lib/types";

const idle: BootstrapProgress = {
  running: false,
  complete: false,
  failed: false,
  currentStep: "idle",
  message: "Ready",
  currentStepEtaSeconds: 0,
  overallPercent: 0,
};

function renderStep(overrides: Partial<React.ComponentProps<typeof LauncherInstallStep>> = {}) {
  const props: React.ComponentProps<typeof LauncherInstallStep> = {
    appSemver: "1.0",
    bootstrapping: false,
    bootstrapError: null,
    bootstrapProgress: idle,
    bootstrapComplete: false,
    copyFirstRunFootprint: vi.fn(),
    handleBootstrap: vi.fn(),
    handleFirstLaunchContinue: vi.fn(),
    onMouseDown: vi.fn(),
    onboardingFootprintCopyNotice: null,
    runtimeStatus: null,
    showInstallProgress: false,
    stepBasePercent: 0,
    stepEtaSeedSeconds: 60,
    stepStartedAtMs: null,
    ...overrides,
  };
  return { ...render(<LauncherInstallStep {...props} />), props };
}

describe("LauncherInstallStep", () => {
  it("wires installation and footprint disclosure actions", async () => {
    const user = userEvent.setup();
    const { props } = renderStep();
    await user.click(screen.getByRole("button", { name: "Install AI Switchboard for Mac" }));
    await user.click(screen.getByRole("button", { name: "Copy footprint" }));
    expect(props.handleBootstrap).toHaveBeenCalledOnce();
    expect(props.copyFirstRunFootprint).toHaveBeenCalledOnce();
    expect(screen.getByText(/Your system Python is untouched/)).toBeVisible();
  });

  it("shows progress, errors, and prevents duplicate installs", () => {
    const { container } = renderStep({
      bootstrapping: true,
      bootstrapError: "Download failed",
      bootstrapProgress: { ...idle, running: true, currentStep: "download", message: "Downloading", currentStepEtaSeconds: 30, overallPercent: 25 },
      showInstallProgress: true,
    });
    expect(screen.getByRole("button", { name: "Installing local engine…" })).toBeDisabled();
    expect(screen.getByText(/Downloading/)).toBeVisible();
    expect(screen.getByText("Download failed")).toBeVisible();
    expect(container.querySelector(".install-progress__bar-fill")).toBeInTheDocument();
  });

  it("waits for runtime health before enabling first-launch continuation", async () => {
    const user = userEvent.setup();
    const { rerender, props } = renderStep({ bootstrapComplete: true });
    expect(screen.getByRole("button", { name: "Starting engine…" })).toBeDisabled();

    rerender(<LauncherInstallStep {...props} runtimeStatus={{ running: true } as never} />);
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(props.handleFirstLaunchContinue).toHaveBeenCalledOnce();
  });
});
