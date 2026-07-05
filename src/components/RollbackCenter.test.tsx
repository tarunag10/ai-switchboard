import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RollbackCenter } from "./RollbackCenter";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("RollbackCenter", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({
      status: "ready",
      confirmationPhrase: "CONFIRM",
      ready: [],
      blocked: [],
    });
  });

  it("renders rollback inventory and managed record controls", () => {
    render(<RollbackCenter />);

    expect(
      screen.getByRole("heading", { name: "Rollback Center" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /copy inventory/i }),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/Codex/i).length).toBeGreaterThan(0);
    expect(
      screen.getAllByRole("button", { name: /preview/i }).length,
    ).toBeGreaterThan(0);
  });

  it("previews undo-all rollback with the native command", async () => {
    const user = userEvent.setup();
    render(<RollbackCenter />);

    await user.click(findButton(/preview/i, /undo-all/i));

    expect(invokeMock).toHaveBeenCalledWith("preview_managed_rollback_undo_all");
  });

  it("keeps undo-all execution disabled until confirmation matches", async () => {
    const user = userEvent.setup();
    render(<RollbackCenter />);

    await user.click(findButton(/preview/i, /undo-all/i));

    const executeButton = (await screen.findAllByRole("button")).find((button) =>
      /execute/i.test(button.textContent ?? ""),
    );
    expect(executeButton).toBeDefined();
    if (!executeButton) {
      throw new Error("Execute button not found");
    }
    expect(executeButton).toBeDisabled();

    await user.type(screen.getByPlaceholderText("CONFIRM"), "NOPE");
    expect(executeButton).toBeDisabled();

    await user.clear(screen.getByPlaceholderText("CONFIRM"));
    await user.type(screen.getByPlaceholderText("CONFIRM"), "CONFIRM");
    expect(executeButton).not.toBeDisabled();
  });
});

function findButton(...patterns: RegExp[]) {
  const button = screen.getAllByRole("button").find((candidate) => {
    const text = candidate.textContent ?? "";
    return patterns.every((pattern) => pattern.test(text));
  });
  if (!button) {
    throw new Error(`Button not found for ${patterns.map(String).join(", ")}`);
  }
  return button;
}
