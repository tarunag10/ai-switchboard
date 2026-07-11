import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { GatewayProfilesCard } from "./GatewayProfilesCard";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

describe("GatewayProfilesCard", () => {
  it("keeps remote disclosure and Doctor evidence behind an explicit details action", async () => {
    render(<GatewayProfilesCard onCopyGuidance={vi.fn()} />);
    const user = userEvent.setup();

    expect(screen.getByText("Semantic Cache")).toBeVisible();
    expect(screen.getByText("Cloudflare AI Gateway")).toBeVisible();
    expect(screen.queryByText(/requests pass through Cloudflare/i)).not.toBeVisible();

    await user.click(screen.getAllByRole("button", { name: "View privacy & Doctor" })[2]);

    expect(screen.getByText(/requests pass through Cloudflare/i)).toBeVisible();
    expect(screen.getAllByText("Required evidence")[2]).toBeVisible();
    expect(screen.getAllByText("Doctor checks")[2]).toBeVisible();
    expect(screen.getByText(/Switchboard writes nothing/i)).toBeVisible();
  });

  it("copies a manual guide without applying configuration", async () => {
    const onCopyGuidance = vi.fn();
    render(<GatewayProfilesCard onCopyGuidance={onCopyGuidance} />);
    const user = userEvent.setup();

    await user.click(screen.getAllByRole("button", { name: "Copy setup & Doctor guide" })[0]);

    expect(onCopyGuidance).toHaveBeenCalledWith(
      expect.stringContaining("Semantic Cache"),
      "Semantic Cache setup and Doctor guide",
    );
  });

  it("records only a local lifecycle receipt and exposes a non-secret config preview", async () => {
    const onCopyGuidance = vi.fn();
    render(<GatewayProfilesCard onCopyGuidance={onCopyGuidance} />);
    const user = userEvent.setup();

    await user.click(screen.getAllByRole("button", { name: "Enable local profile" })[0]);
    await user.click(screen.getAllByRole("button", { name: "View privacy & Doctor" })[0]);
    expect(screen.getAllByText(/local Switchboard receipt only/i)[0]).toBeVisible();

    await user.click(screen.getAllByRole("button", { name: "Copy config preview" })[0]);
    expect(onCopyGuidance).toHaveBeenLastCalledWith(
      expect.stringContaining("not applied"),
      "Semantic Cache config preview",
    );
  });

  it("runs redacted readiness only after an explicit action and never calls it on render", async () => {
    invokeMock.mockResolvedValue({
      profileId: "litellm-local-cache", configuration: [], credentials: [],
      connectivity: { attempted: false, status: "not-run", detail: "No connectivity preflight was run." }, live: false, guidance: "Advisory only.",
    });
    render(<GatewayProfilesCard onCopyGuidance={vi.fn()} />);
    const user = userEvent.setup();
    expect(invokeMock).not.toHaveBeenCalled();
    await user.click(screen.getAllByRole("button", { name: "View privacy & Doctor" })[0]);
    await user.click(screen.getByRole("button", { name: "Check redacted readiness" }));
    expect(invokeMock).toHaveBeenCalledWith("get_gateway_readiness", {
      profileId: "litellm-local-cache", runLocalConnectivity: false,
    });
    expect(await screen.findByText(/Not verified\. Advisory only/)).toBeVisible();
  });
});
