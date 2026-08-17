import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { LauncherRuntimeUpgradeStep } from "./LauncherRuntimeUpgradeStep";
import type { RuntimeUpgradeFailure, RuntimeUpgradeProgress } from "../lib/types";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(() => Promise.resolve()),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const progress: RuntimeUpgradeProgress = {
  running: false,
  complete: false,
  failed: false,
  currentStep: "Installing",
  message: "Working",
  overallPercent: 40,
  fromVersion: "1.0",
  toVersion: "2.0",
};
const failure: RuntimeUpgradeFailure = {
  appVersion: "1.0",
  targetHeadroomVersion: "2.0",
  fallbackHeadroomVersion: "1.9",
  failurePhase: "boot_validation",
  attempts: 3,
  firstAttemptAt: "2026-01-01T00:00:00Z",
  lastAttemptAt: "2026-01-01T00:01:00Z",
  errorMessage: "failed",
  rollbackRestored: true,
};

function renderStep(overrides: Partial<React.ComponentProps<typeof LauncherRuntimeUpgradeStep>> = {}) {
  const props: React.ComponentProps<typeof LauncherRuntimeUpgradeStep> = {
    appSemver: "1.0",
    runtimeUpgradeProgress: progress,
    showUpgradeModal: false,
    showUpgradeSuccess: false,
    upgradeFailure: null,
    upgradeExhausted: false,
    supportIssuesUrl: "https://example.test/issues",
    onMouseDown: vi.fn(),
    onFirstLaunchContinue: vi.fn(),
    ...overrides,
  };
  return { ...render(<LauncherRuntimeUpgradeStep {...props} />), props };
}

describe("LauncherRuntimeUpgradeStep", () => {
  beforeEach(() => invokeMock.mockClear());

  it("renders active and successful progress branches", () => {
    const { rerender } = renderStep({ showUpgradeModal: true });
    expect(screen.getByRole("heading", { name: /Finishing Headroom engine 2.0 update/ })).toBeVisible();
    expect(screen.getByText("Installing")).toBeVisible();

    rerender(<LauncherRuntimeUpgradeStep
      appSemver="1.0" runtimeUpgradeProgress={{ ...progress, message: "Done" }}
      showUpgradeModal={false} showUpgradeSuccess upgradeFailure={null} upgradeExhausted={false}
      supportIssuesUrl="https://example.test/issues" onMouseDown={vi.fn()} onFirstLaunchContinue={vi.fn()}
    />);
    expect(screen.getByRole("heading", { name: "Headroom 2.0 is ready" })).toBeVisible();
    expect(screen.getByText("Done")).toBeVisible();
  });

  it("wires every boot-validation recovery path", async () => {
    const user = userEvent.setup();
    const { props } = renderStep({ upgradeFailure: failure, upgradeExhausted: true });
    await user.click(screen.getByRole("button", { name: "Retry update" }));
    await user.click(screen.getByRole("button", { name: "Continue with previous version" }));
    await user.click(screen.getByRole("button", { name: "Retry with full rebuild" }));
    await user.click(screen.getByRole("button", { name: "Report issue" }));
    expect(invokeMock).toHaveBeenCalledWith("retry_runtime_upgrade");
    expect(props.onFirstLaunchContinue).toHaveBeenCalledOnce();
    expect(invokeMock).toHaveBeenCalledWith("retry_runtime_upgrade_with_rebuild");
    expect(invokeMock).toHaveBeenCalledWith("open_external_link", expect.objectContaining({ url: expect.stringContaining("https://example.test/issues") }));
  });
});
