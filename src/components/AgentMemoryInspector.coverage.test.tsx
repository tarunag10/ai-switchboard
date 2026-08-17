import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentMemoryInspector } from "./AgentMemoryInspector";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const snapshot = { repoPath: "/repo", sources: [{ id: "agents", agent: "codex", sourcePath: "/repo/AGENTS.md", scope: "repo", status: "duplicate", estimatedTokens: 10, secretScan: { status: "safe" }, previewAvailable: true }] };

describe("AgentMemoryInspector alternate handlers", () => {
  beforeEach(() => invokeMock.mockReset());

  it("copies a content-free summary and refreshes an edited repository", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    invokeMock.mockResolvedValue(snapshot);
    render(<AgentMemoryInspector hidden={false} />);
    await screen.findByText("/repo/AGENTS.md");
    await user.click(screen.getByRole("button", { name: "Copy safe summary" }));
    expect(writeText).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: "Copied" })).toBeVisible();
    await user.type(screen.getByRole("textbox", { name: "Repository path" }), "/next");
    await user.click(screen.getByRole("button", { name: "Refresh" }));
    expect(invokeMock).toHaveBeenLastCalledWith("get_agent_memory_snapshot", { repoPath: "/next" });
  });

  it("surfaces snapshot and preview fallback errors", async () => {
    invokeMock.mockRejectedValueOnce({ unavailable: true });
    const { unmount } = render(<AgentMemoryInspector hidden={false} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Agent Memory is unavailable.");
    unmount();

    invokeMock.mockResolvedValueOnce(snapshot).mockRejectedValueOnce({ unavailable: true });
    render(<AgentMemoryInspector hidden={false} />);
    await screen.findByText("/repo/AGENTS.md");
    await userEvent.click(screen.getByRole("button", { name: /preview compaction/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Compaction preview is unavailable.");
  });
});
