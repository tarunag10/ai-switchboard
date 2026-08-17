import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ActivityFeed } from "./ActivityFeed";
import type { ActivityFeedResponse } from "../lib/types";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const feed: ActivityFeedResponse = {
  proxyReachable: true,
  tiles: { transformation: null, record: null, rtkToday: null, learningsMilestone: null, weeklyRecap: null, trainSuggestion: null },
};

describe("ActivityFeed message logging controls", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ fullMessageLogging: false, fullMessageLoggingExpiresAt: null, messageLogRetentionHours: 24 });
  });

  it("requires explicit consent and enables the selected standard duration", async () => {
    const user = userEvent.setup();
    render(<ActivityFeed feed={feed} error={null} />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_message_logging_settings"));
    await user.click(screen.getByRole("button", { name: "Enable full message logging" }));
    expect(screen.getByRole("button", { name: "Enable temporarily" })).toBeDisabled();
    await user.click(screen.getByRole("checkbox"));
    invokeMock.mockResolvedValueOnce({ fullMessageLogging: true, fullMessageLoggingExpiresAt: null, messageLogRetentionHours: 24 });
    await user.click(screen.getByRole("button", { name: "Enable temporarily" }));
    expect(invokeMock).toHaveBeenLastCalledWith("enable_full_message_logging", { hours: 1 });
    expect(await screen.findByText("Full message logging enabled for 1 hour(s).")).toBeVisible();
  });

  it("uses the explicit expiry payload for the 15-minute option", async () => {
    const user = userEvent.setup();
    render(<ActivityFeed feed={feed} error={null} />);
    await user.click(screen.getByRole("button", { name: "Enable full message logging" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Expiration" }), "0.25");
    await user.click(screen.getByRole("checkbox"));
    await user.click(screen.getByRole("button", { name: "Enable temporarily" }));
    expect(invokeMock).toHaveBeenLastCalledWith("set_message_logging_settings", {
      settings: expect.objectContaining({ fullMessageLogging: true, messageLogRetentionHours: 24, fullMessageLoggingExpiresAt: expect.any(String) }),
    });
    expect(await screen.findByText("Full message logging enabled for 15 minutes.")).toBeVisible();
  });

  it("disables active logging and handles both purge result branches", async () => {
    const user = userEvent.setup();
    invokeMock.mockResolvedValueOnce({ fullMessageLogging: true, fullMessageLoggingExpiresAt: null, messageLogRetentionHours: 24 });
    render(<ActivityFeed feed={feed} error={null} />);
    const disable = await screen.findByRole("button", { name: "Disable full message logging" });
    invokeMock.mockResolvedValueOnce({ fullMessageLogging: false, fullMessageLoggingExpiresAt: null, messageLogRetentionHours: 24 });
    await user.click(disable);
    expect(invokeMock).toHaveBeenLastCalledWith("disable_full_message_logging");
    expect(await screen.findByText("Full message logging disabled.")).toBeVisible();

    invokeMock.mockResolvedValueOnce({ purged: true, removedPaths: ["facts.json"] });
    await user.click(screen.getByRole("button", { name: "Purge message logs" }));
    expect(await screen.findByText("Message logs purged: facts.json.")).toBeVisible();
    invokeMock.mockResolvedValueOnce({ purged: false, removedPaths: [] });
    await user.click(screen.getByRole("button", { name: "Purge message logs" }));
    expect(await screen.findByText("No persisted message logs found to purge.")).toBeVisible();
  });

  it("closes the warning without invoking a mutation", async () => {
    const user = userEvent.setup();
    render(<ActivityFeed feed={feed} error={null} />);
    await user.click(screen.getByRole("button", { name: "Enable full message logging" }));
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });
});
