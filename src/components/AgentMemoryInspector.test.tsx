import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentMemoryInspector } from "./AgentMemoryInspector";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

describe("AgentMemoryInspector", () => {
  beforeEach(() => { invokeMock.mockReset(); });
  it("renders a local source inventory and a safe copy action", async () => {
    invokeMock.mockResolvedValue({ repoPath: "/repo", sources: [{ id: "agents", agent: "codex", sourcePath: "/repo/AGENTS.md", scope: "repo", status: "duplicate", estimatedTokens: 500, duplicateTokens: 60, cacheableTokens: 440, secretScan: { status: "safe" }, previewAvailable: true }] });
    render(<AgentMemoryInspector hidden={false} />);
    expect(await screen.findByText(/1 source/i)).toBeInTheDocument();
    expect(screen.getByText("/repo/AGENTS.md")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /copy safe summary/i })).toBeInTheDocument();
  });
  it("blocks diff rendering when the preview secret scan is blocked", async () => {
    invokeMock.mockImplementation((command: string) => command === "get_agent_memory_snapshot" ? Promise.resolve({ repoPath: "/repo", sources: [{ id: "claude", agent: "claude", sourcePath: "/repo/CLAUDE.md", scope: "repo", status: "live", estimatedTokens: 30, secretScan: { status: "safe" }, previewAvailable: true }] }) : Promise.resolve({ agent: "claude", secretScan: { status: "blocked", reason: "API key" }, diff: "secret value" }));
    render(<AgentMemoryInspector hidden={false} />);
    await screen.findByText("/repo/CLAUDE.md");
    fireEvent.click(screen.getByRole("button", { name: /preview compaction/i }));
    expect(await screen.findByText(/Preview content is blocked/i)).toBeInTheDocument();
    expect(screen.queryByText("secret value")).not.toBeInTheDocument();
  });
  it("requires an explicit phrase before applying a safe managed preview", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_agent_memory_snapshot") return Promise.resolve({ repoPath: "/repo", sources: [{ id: "managed", agent: "codex", sourcePath: "/repo/.switchboard/AGENTS.md", scope: "repo", status: "duplicate", managedBySwitchboard: true, estimatedTokens: 30, secretScan: { status: "safe" }, previewAvailable: true }] });
      if (command === "preview_agent_memory_compaction") return Promise.resolve({ agent: "codex", secretScan: { status: "safe" }, summary: "Would remove duplicates.", applyEligible: true, confirmationPhrase: "APPLY AGENT MEMORY COMPACTION FOR CODEX" });
      return Promise.resolve([{ receiptId: "receipt-1", agent: "codex", backupPath: "/repo/.switchboard/backups/receipt-1", rollbackAvailable: true, rollbackConfirmationPhrase: "ROLLBACK AGENT MEMORY COMPACTION receipt-1" }]);
    });
    render(<AgentMemoryInspector hidden={false} />);
    await screen.findByText("/repo/.switchboard/AGENTS.md");
    fireEvent.click(screen.getByRole("button", { name: /preview compaction/i }));
    const apply = await screen.findByRole("button", { name: /apply compaction/i });
    expect(apply).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/apply memory confirmation/i), { target: { value: "APPLY AGENT MEMORY COMPACTION FOR CODEX" } });
    fireEvent.click(apply);
    expect((await screen.findAllByText(/receipt-1/i)).length).toBeGreaterThan(0);
    expect(invokeMock).toHaveBeenCalledWith("apply_agent_memory_compaction", expect.objectContaining({ confirmationPhrase: "APPLY AGENT MEMORY COMPACTION FOR CODEX" }));
  });
});
