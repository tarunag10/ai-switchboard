import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { LauncherPostInstallStep } from "./LauncherPostInstallStep";
import type { DashboardState } from "../lib/types";

const dashboard = {
  launchExperience: "returning",
  lifetimeEstimatedSavingsUsd: 12.34,
  lifetimeEstimatedTokensSaved: 12_000,
} as unknown as DashboardState;

function renderStep(overrides: Partial<React.ComponentProps<typeof LauncherPostInstallStep>> = {}) {
  const props: React.ComponentProps<typeof LauncherPostInstallStep> = {
    appSemver: "1.0",
    dashboard,
    savingsDashboard: dashboard,
    lifetimeDataDays: 2,
    lifetimeDataDaysLabel: "Last 2 tracked days",
    onMouseDown: vi.fn(),
    onBack: vi.fn(),
    onGetStarted: vi.fn(),
    ...overrides,
  };
  return { ...render(<LauncherPostInstallStep {...props} />), props };
}

describe("LauncherPostInstallStep", () => {
  it("renders returning-user lifetime metrics and wires navigation", async () => {
    const user = userEvent.setup();
    const { props } = renderStep();
    expect(screen.getByText("$12")).toBeVisible();
    expect(screen.getByText("12K")).toBeVisible();
    expect(screen.getByText("Across 2 tracked days")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Back" }));
    await user.click(screen.getByRole("button", { name: "Get started" }));
    expect(props.onBack).toHaveBeenCalledOnce();
    expect(props.onGetStarted).toHaveBeenCalledOnce();
  });

  it("uses first-run guidance instead of misleading historical metrics", () => {
    renderStep({ dashboard: { ...dashboard, launchExperience: "first_run" } });
    expect(screen.getByText(/Use Test setup to send a first prompt/)).toBeVisible();
    expect(screen.queryByText("Savings all-time")).not.toBeInTheDocument();
  });

  it("describes zero-day history without pluralizing a tracked period", () => {
    renderStep({ lifetimeDataDays: 0 });
    expect(screen.getByText("Across all recorded usage")).toBeVisible();
  });
});
