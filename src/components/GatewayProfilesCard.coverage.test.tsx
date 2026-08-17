import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { GatewayProfilesCard } from "./GatewayProfilesCard";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

describe("GatewayProfilesCard additional lifecycle branches", () => {
  beforeEach(() => { localStorage.clear(); invokeMock.mockReset(); });

  it("runs opt-in connectivity, copies the env template, and records reviewed evidence", async () => {
    const user = userEvent.setup();
    const onCopyGuidance = vi.fn();
    invokeMock.mockResolvedValue({ profileId: "litellm-local-cache", configuration: [], credentials: [], connectivity: { attempted: true, status: "reachable", detail: "loopback ready" }, live: false, guidance: "Advisory" });
    render(<GatewayProfilesCard onCopyGuidance={onCopyGuidance} />);
    await user.click(screen.getAllByRole("button", { name: "View privacy & Doctor" })[0]);
    await user.click(screen.getByRole("button", { name: "Run opt-in local proxy preflight" }));
    expect(invokeMock).toHaveBeenCalledWith("get_gateway_readiness", { profileId: "litellm-local-cache", runLocalConnectivity: true });
    await user.click(screen.getByRole("button", { name: "Download env template" }));
    expect(onCopyGuidance).toHaveBeenCalledWith(expect.stringContaining("LITELLM"), "LiteLLM env template");
    const check = screen.getAllByRole("checkbox")[0];
    await user.click(check);
    expect(check).toBeChecked();
    expect(screen.getByText(/evidence-reviewed/)).toBeVisible();
    await user.click(check);
    expect(check).not.toBeChecked();
  });

  it("surfaces structured and fallback readiness failures", async () => {
    const user = userEvent.setup();
    invokeMock.mockRejectedValueOnce(new Error("doctor unavailable"));
    const { unmount } = render(<GatewayProfilesCard onCopyGuidance={vi.fn()} />);
    await user.click(screen.getAllByRole("button", { name: "View privacy & Doctor" })[0]);
    await user.click(screen.getByRole("button", { name: "Check redacted readiness" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("doctor unavailable");
    unmount();
    invokeMock.mockRejectedValueOnce({ unavailable: true });
    render(<GatewayProfilesCard onCopyGuidance={vi.fn()} />);
    await user.click(screen.getAllByRole("button", { name: "View privacy & Doctor" })[0]);
    await user.click(screen.getByRole("button", { name: "Check redacted readiness" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Readiness check could not run.");
  });
});
