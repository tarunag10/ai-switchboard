import { render, screen, within, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RollbackCenter } from "./RollbackCenter";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

function ownerCard(owner: string) {
  const label = screen.getByText(owner, { selector: "strong" });
  const card = label.closest(".rollback-center-card__item");
  if (!card) throw new Error(`Missing rollback card for ${owner}`);
  return within(card as HTMLElement);
}

describe("RollbackCenter native workflows", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.useRealTimers();
  });

  it("executes undo-all only after exact confirmation and reports the result", async () => {
    const user = userEvent.setup();
    invokeMock
      .mockResolvedValueOnce({ status: "ready", confirmationPhrase: "UNDO ALL", ready: [{ recordId: "codex-routing" }], blocked: [] })
      .mockResolvedValueOnce({ executed: [{ recordId: "codex-routing" }], blocked: [] });
    render(<RollbackCenter />);

    await user.click(screen.getByRole("button", { name: "Preview native undo-all" }));
    expect(invokeMock).toHaveBeenNthCalledWith(1, "preview_managed_rollback_undo_all");
    await user.type(await screen.findByPlaceholderText("UNDO ALL"), "UNDO ALL");
    await user.click(screen.getByRole("button", { name: "Execute native undo-all" }));

    expect(invokeMock).toHaveBeenNthCalledWith(2, "execute_managed_rollback_undo_all", { confirmationPhrase: "UNDO ALL" });
    expect(await screen.findByText(/Executed 1; left 0 blocked/)).toBeInTheDocument();
    expect(screen.getByText("Undo-all executed 1 native row.")).toBeInTheDocument();
  });

  it("renders undo-all preview and execution failures without issuing extra actions", async () => {
    const user = userEvent.setup();
    invokeMock.mockRejectedValueOnce(new Error("preview offline"));
    const view = render(<RollbackCenter />);
    await user.click(screen.getByRole("button", { name: "Preview native undo-all" }));
    expect(await screen.findByText("preview offline")).toBeInTheDocument();

    view.unmount();
    invokeMock
      .mockReset()
      .mockResolvedValueOnce({ status: "ready", confirmationPhrase: "UNDO ALL", ready: [], blocked: [] })
      .mockRejectedValueOnce(new Error("execution refused"));
    render(<RollbackCenter />);
    await user.click(screen.getByRole("button", { name: "Preview native undo-all" }));
    await user.type(await screen.findByPlaceholderText("UNDO ALL"), "UNDO ALL");
    await user.click(screen.getByRole("button", { name: "Execute native undo-all" }));
    expect(await screen.findByText("execution refused")).toBeInTheDocument();
  });

  it("previews and executes managed config apply with exact payloads", async () => {
    const user = userEvent.setup();
    invokeMock.mockImplementation((command: string) => {
      if (command === "preview_managed_config_apply") return Promise.resolve({
        status: "ready", confirmationPhrase: "APPLY OPENCODE", targetPath: "/tmp/opencode.json",
        backupPath: "/tmp/opencode.backup", rollbackPreview: "Restore backup", blockedReason: null,
      });
      if (command === "execute_managed_config_apply") return Promise.resolve({ changed: true, backupPath: "/tmp/opencode.backup" });
      if (command === "preview_managed_rollback") return Promise.resolve({ status: "blocked", confirmationPhrase: "ROLLBACK", targetPath: "/tmp/opencode.json", backupPath: null, markerPresent: true, blockedReason: "No backup" });
      return Promise.resolve(undefined);
    });
    render(<RollbackCenter />);
    const card = ownerCard("OpenCode routing");
    await user.click(card.getByRole("button", { name: "Preview safe apply" }));
    expect(invokeMock).toHaveBeenCalledWith("preview_managed_config_apply", { recordId: "opencode-routing" });
    await user.type(await card.findByPlaceholderText("APPLY OPENCODE"), "APPLY OPENCODE");
    await user.click(card.getByRole("button", { name: "Apply OpenCode routing" }));
    expect(invokeMock).toHaveBeenCalledWith("execute_managed_config_apply", { recordId: "opencode-routing", confirmationPhrase: "APPLY OPENCODE" });
    expect(await card.findByText(/Applied: changed; backup: \/tmp\/opencode.backup/)).toBeInTheDocument();
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("preview_managed_rollback", { recordId: "opencode-routing" }));
  });

  it("previews and executes standard and dedicated cleanup rollbacks", async () => {
    const user = userEvent.setup();
    invokeMock.mockImplementation((command: string, payload: any) => {
      if (command.startsWith("preview_")) return Promise.resolve({
        status: "ready", confirmationPhrase: payload?.recordId === "managed-storage" ? "CLEAN" : "ROLLBACK",
        targetPath: "/tmp/target", backupPath: "/tmp/backup", markerPresent: true, backupExists: true, blockedReason: null,
      });
      return Promise.resolve({ restoredFrom: "/tmp/backup", safetyBackupPath: "/tmp/safety" });
    });
    render(<RollbackCenter />);

    const standard = ownerCard("OpenCode routing");
    await user.click(standard.getByRole("button", { name: "Preview native rollback" }));
    await user.type(await standard.findByPlaceholderText("ROLLBACK"), "ROLLBACK");
    await user.click(standard.getByRole("button", { name: "Execute rollback for OpenCode routing" }));
    expect(invokeMock).toHaveBeenCalledWith("execute_managed_rollback", {
      recordId: "opencode-routing", backupPath: "/tmp/backup", confirmationPhrase: "ROLLBACK",
    });

    const cleanup = ownerCard("AI Switchboard runtime");
    await user.click(cleanup.getByRole("button", { name: "Preview native rollback" }));
    await user.type(await cleanup.findByPlaceholderText("CLEAN"), "CLEAN");
    await user.click(cleanup.getByRole("button", { name: /Execute rollback/ }));
    expect(invokeMock).toHaveBeenCalledWith("execute_dedicated_cleanup_rollback", {
      recordId: "managed-storage", confirmationPhrase: "CLEAN",
    });
  });
});
