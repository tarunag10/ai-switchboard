import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { mockDashboard } from "../lib/mockData";
import TrayApp from "./TrayApp";

const invokeMock = vi.fn<(command: string, args?: unknown) => Promise<unknown>>();
const tauriWindowMock = vi.hoisted(() => ({
  label: "main",
  hide: vi.fn().mockResolvedValue(undefined),
  onFocusChanged: vi.fn().mockResolvedValue(vi.fn()),
  startDragging: vi.fn().mockResolvedValue(undefined),
}));
let dashboardResponse = {
  ...mockDashboard,
  bootstrapComplete: true,
  launchExperience: "returning" as const,
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (command: string, args?: unknown) => invokeMock(command, args),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => tauriWindowMock,
}));

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(false),
  requestPermission: vi.fn().mockResolvedValue("denied"),
  sendNotification: vi.fn(),
}));

describe("TrayApp integrated shell", () => {
  beforeEach(() => {
    window.localStorage.clear();
    delete (window as Window & { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
    tauriWindowMock.label = "main";
    tauriWindowMock.hide.mockClear();
    tauriWindowMock.onFocusChanged.mockClear();
    tauriWindowMock.startDragging.mockClear();
    invokeMock.mockReset();
    dashboardResponse = {
      ...mockDashboard,
      bootstrapComplete: true,
      launchExperience: "returning",
    };
    invokeMock.mockImplementation(async (command) => {
      switch (command) {
        case "get_dashboard_state":
          return dashboardResponse;
        case "get_bootstrap_progress":
          return {
            currentStep: null,
            overallPercent: 100,
            message: "Ready",
            running: false,
            complete: true,
            failed: false,
            error: null,
            logs: [],
          };
        case "get_client_connectors":
        case "get_savings_attribution_events":
          return [];
        case "load_release_readiness_report":
          return null;
        case "accept_terms":
          return true;
        default:
          throw new Error(`Unavailable in browser smoke: ${command}`);
      }
    });
  });

  it("boots through the real application controller and mounts every sidebar route", async () => {
    render(<TrayApp />);

    const navigation = await screen.findByRole("navigation", {
      name: "AI Switchboard navigation",
    });
    expect(navigation).toBeInTheDocument();

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("get_dashboard_state", undefined);
      expect(invokeMock).toHaveBeenCalledWith("get_bootstrap_progress", undefined);
      expect(invokeMock).toHaveBeenCalledWith("get_runtime_status", undefined);
      expect(invokeMock).toHaveBeenCalledWith("get_switchboard_state", undefined);
      expect(invokeMock).toHaveBeenCalledWith("get_doctor_report", undefined);
      expect(invokeMock).toHaveBeenCalledWith("get_managed_footprint", undefined);
      expect(invokeMock).toHaveBeenCalledWith("get_client_connectors", undefined);
    });

    const destinations = [
      "Optimize",
      "Repo Intelligence",
      "Routing",
      "Workbench",
      "Savings",
      "Doctor",
      "Token X-Ray",
      "Daily Briefing",
      "Agent Memory",
      "Event Log",
      "Repo Map",
      "Addons",
      "Overview",
    ];

    for (const label of destinations) {
      const button = within(navigation).getByRole("button", { name: label });
      fireEvent.click(button);
      expect(button).toHaveClass("is-active");
      if (label === "Workbench") {
        expect(
          screen.getByRole("heading", { level: 1, name: "Workbench" }),
        ).toBeVisible();
      }
    }

    const settingsButton = screen.getByRole("button", { name: "Settings" });
    fireEvent.click(settingsButton);
    expect(settingsButton).toHaveClass("is-active");
  }, 30_000);

  it("opens the existing Addons harness replay panel from Workbench readiness", async () => {
    render(<TrayApp />);

    const navigation = await screen.findByRole("navigation", {
      name: "AI Switchboard navigation",
    });
    fireEvent.click(within(navigation).getByRole("button", { name: "Workbench" }));

    expect(
      screen.getByRole("heading", { level: 1, name: "Workbench" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Open harness replay" }));

    expect(within(navigation).getByRole("button", { name: "Addons" })).toHaveClass("is-active");
    const replayHeadings = screen.getAllByRole("heading", {
      level: 2,
      name: "Redacted harness replay",
    });
    expect(replayHeadings).toHaveLength(1);
    expect(replayHeadings[0]).toBeVisible();
  }, 30_000);

  it("blocks the assembled app on updated legal terms and resumes after persisted acceptance", async () => {
    dashboardResponse = {
      ...dashboardResponse,
      requiredTermsVersion: 3,
      acceptedTermsVersion: 2,
    };

    render(<TrayApp />);

    expect(
      await screen.findByRole("dialog", { name: "AI Switchboard Terms of Use" }),
    ).toBeInTheDocument();
    const accept = screen.getByRole("button", { name: "Accept & Continue" });
    expect(accept).toBeDisabled();

    fireEvent.click(
      screen.getByRole("checkbox", {
        name: /I have read and accept the AI Switchboard Terms of Use/i,
      }),
    );
    fireEvent.click(accept);

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("accept_terms", { version: 3 }),
    );
    expect(
      await screen.findByRole("navigation", { name: "AI Switchboard navigation" }),
    ).toBeInTheDocument();
  });

  it("renders the resumed native launcher and requests animated hide from Get started", async () => {
    tauriWindowMock.label = "launcher";
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: { metadata: { currentWindow: { label: "launcher" } } },
    });

    render(<TrayApp />);

    expect(
      await screen.findByRole("heading", {
        name: /AI Switchboard is ready/i,
      }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Get started" }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("hide_launcher_animated", undefined),
    );
  });
});
