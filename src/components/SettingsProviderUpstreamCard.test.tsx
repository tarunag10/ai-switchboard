import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsProviderUpstreamCard } from "./SettingsProviderUpstreamCard";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const state = {
  version: 1,
  openai: { enabled: false, url: "" },
  anthropic: { enabled: false, url: "" },
};

describe("SettingsProviderUpstreamCard", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (command === "test_provider_upstream_profile") {
        return Promise.resolve({ ok: true, detail: "Models endpoint responded", statusCode: 200 });
      }
      return Promise.resolve(state);
    });
    vi.spyOn(window, "confirm").mockReturnValue(true);
  });

  it("loads, edits, and tests an OpenAI-compatible endpoint with exact payload", async () => {
    const user = userEvent.setup();
    render(<SettingsProviderUpstreamCard />);
    const checkbox = await screen.findByRole("checkbox", { name: /Route OpenAI-compatible/ });
    await user.click(checkbox);
    const url = screen.getByRole("textbox", { name: "OpenAI-compatible upstream URL" });
    await user.type(url, "https://api.deepseek.com/v1");
    await user.click(screen.getAllByRole("button", { name: "Test connection" })[0]);

    expect(invokeMock).toHaveBeenCalledWith("test_provider_upstream_profile", {
      provider: "openai",
      url: "https://api.deepseek.com/v1",
    });
    expect(await screen.findByRole("status")).toHaveTextContent("Reachable");
    expect(screen.getByText(/HTTP 200/)).toBeVisible();
  });

  it("saves the complete state using the user's restart decision", async () => {
    const user = userEvent.setup();
    render(<SettingsProviderUpstreamCard />);
    await screen.findByRole("button", { name: "Save overrides" });
    vi.mocked(window.confirm).mockReturnValue(false);
    await user.click(screen.getByRole("button", { name: "Save overrides" }));
    expect(invokeMock).toHaveBeenLastCalledWith("set_provider_upstream_profiles", {
      state,
      restartHeadroom: false,
    });
    expect(await screen.findByText(/Restart Headroom before routing/)).toBeVisible();
  });

  it("requires confirmation before clearing and resets through the backend", async () => {
    const user = userEvent.setup();
    render(<SettingsProviderUpstreamCard />);
    await screen.findByRole("button", { name: "Clear overrides" });
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    await user.click(screen.getByRole("button", { name: "Clear overrides" }));
    expect(invokeMock).not.toHaveBeenCalledWith("clear_provider_upstream_profiles_command", expect.anything());

    vi.mocked(window.confirm).mockReturnValueOnce(true);
    await user.click(screen.getByRole("button", { name: "Clear overrides" }));
    expect(invokeMock).toHaveBeenCalledWith("clear_provider_upstream_profiles_command", { restartHeadroom: true });
    expect(await screen.findByText("Upstream overrides cleared.")).toBeVisible();
  });

  it("reports backend load failures", async () => {
    invokeMock.mockRejectedValueOnce(new Error("profile corrupt"));
    render(<SettingsProviderUpstreamCard />);
    expect(await screen.findByRole("alert")).toHaveTextContent("profile corrupt");
    await waitFor(() => expect(screen.queryByText("Loading upstream profile…")).not.toBeInTheDocument());
  });
});
