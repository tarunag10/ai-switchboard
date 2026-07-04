import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { TermsGate } from "./TermsGate";

function renderGate(
  overrides: Partial<React.ComponentProps<typeof TermsGate>> = {},
) {
  const onAccepted = overrides.onAccepted ?? vi.fn();
  const props: React.ComponentProps<typeof TermsGate> = {
    requiredVersion: 3,
    onAccepted,
    ...overrides,
  };
  render(<TermsGate {...props} />);
  return { onAccepted };
}

describe("TermsGate", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it("disables Accept until consent box is checked", async () => {
    renderGate();
    const accept = screen.getByRole("button", { name: "Accept & Continue" });
    expect(accept).toBeDisabled();

    await userEvent.click(screen.getByRole("checkbox"));
    expect(accept).toBeEnabled();
  });

  it("does not invoke accept_terms while box is unchecked", async () => {
    renderGate();
    await userEvent.click(
      screen.getByRole("button", { name: "Accept & Continue" }),
    );
    expect(invokeMock).not.toHaveBeenCalledWith(
      "accept_terms",
      expect.anything(),
    );
  });

  it("persists acceptance with the required version and calls onAccepted", async () => {
    const { onAccepted } = renderGate({ requiredVersion: 5 });
    await userEvent.click(screen.getByRole("checkbox"));
    await userEvent.click(
      screen.getByRole("button", { name: "Accept & Continue" }),
    );

    expect(invokeMock).toHaveBeenCalledWith("accept_terms", { version: 5 });
    await vi.waitFor(() => expect(onAccepted).toHaveBeenCalledTimes(1));
  });

  it("does not double-submit while acceptance is pending", async () => {
    let resolveAccept: () => void = () => {};
    invokeMock.mockImplementationOnce(() => {
      return new Promise<void>((resolve) => {
        resolveAccept = resolve;
      });
    });
    const { onAccepted } = renderGate();
    await userEvent.click(screen.getByRole("checkbox"));

    const accept = screen.getByRole("button", { name: "Accept & Continue" });
    await userEvent.click(accept);

    const saving = screen.getByRole("button", { name: "Saving..." });
    expect(saving).toBeDisabled();
    await userEvent.click(saving);
    const acceptCalls = invokeMock.mock.calls.filter(
      ([cmd]) => cmd === "accept_terms",
    );
    expect(acceptCalls).toHaveLength(1);

    resolveAccept();
    await vi.waitFor(() => expect(onAccepted).toHaveBeenCalledTimes(1));
  });

  it("re-enables the button so the user can retry when persistence fails", async () => {
    invokeMock.mockRejectedValueOnce(new Error("keychain write failed"));
    const { onAccepted } = renderGate();
    await userEvent.click(screen.getByRole("checkbox"));
    await userEvent.click(
      screen.getByRole("button", { name: "Accept & Continue" }),
    );

    expect(onAccepted).not.toHaveBeenCalled();
    await vi.waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Accept & Continue" }),
      ).toBeEnabled(),
    );
  });

  it("renders bundled legal notices without upstream links", () => {
    renderGate();

    expect(screen.getByLabelText("Legal notices")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", {
        level: 1,
        name: "AI Switchboard Terms of Use",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", {
        name: "AI Switchboard Privacy Notice",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Local-only mode should avoid/i),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText(/Terms of Use and Privacy Notice/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });
});
