import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsFooterActions } from "./SettingsFooterActions";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("SettingsFooterActions", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it("opens support and quits through Tauri commands", async () => {
    const user = userEvent.setup();

    render(<SettingsFooterActions supportUrl="https://example.test/support" />);

    await user.click(screen.getByRole("button", { name: "Contact us" }));
    await user.click(
      screen.getByRole("button", { name: "Quit AI Switchboard for Mac" }),
    );

    expect(invokeMock).toHaveBeenNthCalledWith(1, "open_external_link", {
      url: "https://example.test/support",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "quit_headroom");
  });

  it("shows command failures instead of silently swallowing them", async () => {
    const user = userEvent.setup();
    invokeMock.mockRejectedValueOnce(new Error("Link opener unavailable"));

    render(<SettingsFooterActions supportUrl="https://example.test/support" />);

    await user.click(screen.getByRole("button", { name: "Contact us" }));

    expect(screen.getByText("Link opener unavailable")).toBeInTheDocument();
  });

  it("shows quit failures instead of silently swallowing them", async () => {
    const user = userEvent.setup();
    invokeMock.mockRejectedValueOnce(new Error("Quit command unavailable"));

    render(<SettingsFooterActions supportUrl="https://example.test/support" />);

    await user.click(
      screen.getByRole("button", { name: "Quit AI Switchboard for Mac" }),
    );

    expect(screen.getByText("Quit command unavailable")).toBeInTheDocument();
  });

  it("uses current branding when Tauri rejects with a non-Error value", async () => {
    const user = userEvent.setup();
    invokeMock.mockRejectedValueOnce("quit unavailable");

    render(<SettingsFooterActions supportUrl="https://example.test/support" />);

    await user.click(
      screen.getByRole("button", { name: "Quit AI Switchboard for Mac" }),
    );

    expect(
      screen.getByText("Could not quit AI Switchboard for Mac."),
    ).toBeInTheDocument();
  });
});
