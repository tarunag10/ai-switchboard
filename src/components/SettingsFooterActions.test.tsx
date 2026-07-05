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

    await user.click(screen.getByRole("button", { name: "Support" }));
    await user.click(
      screen.getByRole("button", { name: /quit mac ai switchboard/i }),
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

    await user.click(screen.getByRole("button", { name: "Support" }));

    expect(screen.getByText("Link opener unavailable")).toBeInTheDocument();
  });

  it("shows quit failures instead of silently swallowing them", async () => {
    const user = userEvent.setup();
    invokeMock.mockRejectedValueOnce(new Error("Quit command unavailable"));

    render(<SettingsFooterActions supportUrl="https://example.test/support" />);

    await user.click(
      screen.getByRole("button", { name: /quit mac ai switchboard/i }),
    );

    expect(screen.getByText("Quit command unavailable")).toBeInTheDocument();
  });
});
