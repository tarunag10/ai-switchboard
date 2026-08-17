import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsHeadroomAdvancedCard } from "./SettingsHeadroomAdvancedCard";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

describe("SettingsHeadroomAdvancedCard", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ version: 1, ccSwitchReconcile: false });
  });

  it("loads settings and persists opt-in without restarting", async () => {
    const user = userEvent.setup();
    render(<SettingsHeadroomAdvancedCard />);
    const checkbox = await screen.findByRole("checkbox");
    await user.click(checkbox);

    expect(invokeMock).toHaveBeenNthCalledWith(1, "get_headroom_advanced_settings");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "set_headroom_advanced_settings", {
      settings: { version: 1, ccSwitchReconcile: true },
      restartHeadroom: false,
    });
    expect(await screen.findByText(/saved\. Restart Headroom/)).toBeVisible();
  });

  it("saves the loaded state and requests a runtime restart", async () => {
    const user = userEvent.setup();
    render(<SettingsHeadroomAdvancedCard />);
    const button = await screen.findByRole("button", { name: "Save and restart Headroom" });
    await user.click(button);
    expect(invokeMock).toHaveBeenLastCalledWith("set_headroom_advanced_settings", {
      settings: { version: 1, ccSwitchReconcile: false },
      restartHeadroom: true,
    });
    expect(await screen.findByText(/runtime restarted/)).toBeVisible();
  });

  it("surfaces initial load failures and restores controls", async () => {
    invokeMock.mockRejectedValueOnce(new Error("storage unavailable"));
    render(<SettingsHeadroomAdvancedCard />);
    expect(await screen.findByRole("alert")).toHaveTextContent("storage unavailable");
    await waitFor(() => expect(screen.getByRole("checkbox")).toBeEnabled());
  });
});
