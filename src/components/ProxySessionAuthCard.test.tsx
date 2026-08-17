import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ProxySessionAuthCard } from "./ProxySessionAuthCard";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const status = {
  available: true,
  enforce: false,
  validatedRequestCount: 7,
  rejectedRequestCount: 2,
};

describe("ProxySessionAuthCard", () => {
  beforeEach(() => invoke.mockReset());

  it("loads, enforces, and refreshes the session token policy", async () => {
    invoke.mockResolvedValue(status);
    render(<ProxySessionAuthCard />);
    expect(await screen.findByText("7")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Enforce session token" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_proxy_session_auth_enforce", { enforce: true }));
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(invoke.mock.calls.filter(([name]) => name === "get_proxy_session_auth_status")).toHaveLength(2));
  });

  it("renders backend failures and leaves unavailable enforcement disabled", async () => {
    invoke.mockRejectedValueOnce(new Error("auth unavailable")).mockResolvedValueOnce({ ...status, available: false });
    render(<ProxySessionAuthCard />);
    expect(await screen.findByRole("alert")).toHaveTextContent("auth unavailable");
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Enforce session token" })).toBeDisabled());
  });
});
