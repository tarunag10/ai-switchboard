import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RuntimeUpgradeOverlay } from "./RuntimeUpgradeOverlay";
import type { RuntimeUpgradeFailure, RuntimeUpgradeProgress } from "../lib/types";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(() => Promise.resolve()),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const idle: RuntimeUpgradeProgress = {
  running: false,
  complete: false,
  failed: false,
  currentStep: "idle",
  message: "",
  overallPercent: 0,
};

const failure: RuntimeUpgradeFailure = {
  appVersion: "1.0.0",
  targetHeadroomVersion: "2.0.0",
  fallbackHeadroomVersion: "1.9.0",
  failurePhase: "boot_validation",
  attempts: 3,
  firstAttemptAt: "2026-01-01T00:00:00Z",
  lastAttemptAt: "2026-01-01T00:01:00Z",
  errorMessage: "boot failed",
  errorHint: null,
  rollbackRestored: true,
};

function renderOverlay(overrides: Partial<React.ComponentProps<typeof RuntimeUpgradeOverlay>> = {}) {
  return render(<RuntimeUpgradeOverlay
    runtimeUpgradeProgress={idle}
    upgradeFailure={null}
    proxyReachable={false}
    supportIssuesUrl="https://example.test/issues"
    maxUpgradeAutoRetries={3}
    {...overrides}
  />);
}

describe("RuntimeUpgradeOverlay", () => {
  beforeEach(() => invokeMock.mockClear());

  it("shows modal progress only while an upgrade is actively incomplete", () => {
    const { container } = renderOverlay({
      runtimeUpgradeProgress: { ...idle, running: true, currentStep: "Installing", message: "Almost there", overallPercent: 42, fromVersion: "1.0", toVersion: "2.0" },
    });
    expect(screen.getByRole("dialog")).toHaveTextContent("Finishing Headroom engine update to 2.0");
    expect(screen.getByText("Almost there")).toBeVisible();
    expect(container.querySelector<HTMLElement>(".install-progress__bar-fill")).toHaveStyle({ width: "42%" });
  });

  it("wires recovery, issue reporting, and safe dismissal commands", async () => {
    const user = userEvent.setup();
    renderOverlay({ upgradeFailure: failure, proxyReachable: true });

    expect(screen.getByRole("alert")).toHaveTextContent("won't auto-retry");
    await user.click(screen.getByRole("button", { name: "Retry now" }));
    await user.click(screen.getByRole("button", { name: "Retry with full rebuild" }));
    await user.click(screen.getByRole("button", { name: "Report issue" }));
    await user.click(screen.getByRole("button", { name: "Dismiss" }));

    expect(invokeMock).toHaveBeenNthCalledWith(1, "retry_runtime_upgrade");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "retry_runtime_upgrade_with_rebuild");
    expect(invokeMock).toHaveBeenNthCalledWith(3, "open_external_link", expect.objectContaining({ url: expect.stringContaining("https://example.test/issues") }));
    expect(invokeMock).toHaveBeenNthCalledWith(4, "dismiss_runtime_upgrade_failure");
  });

  it("keeps dismissal hidden until rollback and proxy health are both proven", () => {
    renderOverlay({ upgradeFailure: { ...failure, rollbackRestored: false }, proxyReachable: true });
    expect(screen.queryByRole("button", { name: "Dismiss" })).not.toBeInTheDocument();
  });
});
