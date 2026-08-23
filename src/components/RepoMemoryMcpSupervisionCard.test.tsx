import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { RepoMemoryMcpSupervisionCard } from "./RepoMemoryMcpSupervisionCard";

const runtime = (overrides: Record<string, unknown> = {}) => ({
  repoMemoryMcpConfigured: false,
  repoMemoryMcpActive: false,
  repoMemoryMcpSupervisionStatus: "not_configured",
  repoMemoryMcpRelaunchSurvivalStatus: "not_applicable",
  repoMemoryMcpSupervisionScope: "app_session",
  repoMemoryMcpService: null,
  ...overrides,
});

describe("RepoMemoryMcpSupervisionCard lifecycle controls", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(runtime());
  });

  it("exposes Prepare MCP and refreshes supervision after preparation", async () => {
    const user = userEvent.setup();
    const prepare = vi.fn(async () => true);
    render(<RepoMemoryMcpSupervisionCard prepareRepoMemoryMcp={prepare} />);

    const button = await screen.findByRole("button", { name: "Prepare MCP" });
    await user.click(button);

    await waitFor(() => expect(prepare).toHaveBeenCalledOnce());
    expect(await screen.findByText(/lifecycle action completed/)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("get_runtime_status");
  });

  it("uses Stop MCP for an active configured service", async () => {
    const user = userEvent.setup();
    const stopOrStart = vi.fn(async () => true);
    invoke.mockResolvedValue(runtime({
      repoMemoryMcpConfigured: true,
      repoMemoryMcpActive: true,
      repoMemoryMcpSupervisionStatus: "verified_active",
    }));
    render(<RepoMemoryMcpSupervisionCard setRepoMemoryMcpActive={stopOrStart} />);

    await user.click(await screen.findByRole("button", { name: "Stop MCP" }));
    await waitFor(() => expect(stopOrStart).toHaveBeenCalledWith(false));
  });

  it("does not offer lifecycle actions without wired callbacks", async () => {
    render(<RepoMemoryMcpSupervisionCard />);
    expect(await screen.findByRole("button", { name: "Prepare MCP" })).toBeDisabled();
  });
});
