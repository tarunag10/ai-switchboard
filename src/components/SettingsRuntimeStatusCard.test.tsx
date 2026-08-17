import { createRef } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SettingsRuntimeStatusCard } from "./SettingsRuntimeStatusCard";
import type { RuntimeStatus } from "../lib/types";

const runtime = {
  running: true,
  proxyReachable: false,
  mcpConfigured: undefined,
  kompressEnabled: true,
} as unknown as RuntimeStatus;

function renderCard(overrides: Partial<React.ComponentProps<typeof SettingsRuntimeStatusCard>> = {}) {
  const props: React.ComponentProps<typeof SettingsRuntimeStatusCard> = {
    appSemver: "1.2.3",
    appUpdateConfig: { enabled: true, currentVersion: "1.2.3", endpointCount: 1, betaChannelEnabled: true },
    appUpdateBusy: false,
    appUpdateInstallBusy: false,
    appUpdateStatusCopy: "You're up to date.",
    checkForAppUpdate: vi.fn(),
    headroomVersion: "0.9",
    headroomLifetimeSavingsPct: 22.2,
    runtimeStatus: runtime,
    kompressWarming: false,
    runtimeActionError: null,
    runtimeLabel: "Headroom",
    showHeadroomDetails: false,
    headroomLogLines: [],
    headroomLogRef: createRef<HTMLPreElement>(),
    onOpenHeadroomDashboard: vi.fn(),
    onToggleHeadroomDetails: vi.fn(),
    ...overrides,
  };
  return { ...render(<SettingsRuntimeStatusCard {...props} />), props };
}

describe("SettingsRuntimeStatusCard", () => {
  it("exposes status meaning and wires updates, dashboard, and log controls", async () => {
    const user = userEvent.setup();
    const { props } = renderCard();
    expect(screen.getByText("beta channel")).toBeVisible();
    expect(screen.getByRole("status", { name: "Runtime: Running" })).toBeVisible();
    expect(screen.getByRole("status", { name: "MCP: Unknown" })).toHaveAttribute("title", "MCP status unknown");
    await user.click(screen.getByRole("button", { name: "Check for updates" }));
    await user.click(screen.getByRole("button", { name: /Proxy: Offline, 6767/ }));
    await user.click(screen.getByRole("button", { name: "Show runtime logs" }));
    expect(props.checkForAppUpdate).toHaveBeenCalledOnce();
    expect(props.onOpenHeadroomDashboard).toHaveBeenCalledOnce();
    expect(props.onToggleHeadroomDetails).toHaveBeenCalledOnce();
  });

  it("renders warming, error, and log branches without losing accessibility", () => {
    const { container } = renderCard({ kompressWarming: true, runtimeActionError: "Runtime stopped", showHeadroomDetails: true, headroomLogLines: ["line one", "line two"] });
    expect(screen.getByRole("status", { name: "Kompress: Unknown, warming up" })).toBeVisible();
    expect(screen.getByRole("alert")).toHaveTextContent("Runtime stopped");
    expect(container.querySelector("pre.runtime-log")).toHaveTextContent("line one line two");
    expect(screen.getByRole("button", { name: "Hide runtime logs" })).toBeVisible();
  });

  it("blocks update checks during check or install work", () => {
    const { rerender, props } = renderCard({ appUpdateBusy: true });
    expect(screen.getByRole("button", { name: "Checking…" })).toBeDisabled();
    rerender(<SettingsRuntimeStatusCard {...props} appUpdateBusy={false} appUpdateInstallBusy />);
    expect(screen.getByRole("button", { name: "Check for updates" })).toBeDisabled();
  });
});
