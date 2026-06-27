import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args)
}));

import { TermsGate } from "./TermsGate";

function renderGate(overrides: Partial<React.ComponentProps<typeof TermsGate>> = {}) {
  const onAccepted = overrides.onAccepted ?? vi.fn();
  const props: React.ComponentProps<typeof TermsGate> = {
    requiredVersion: 3,
    onAccepted,
    ...overrides
  };
  render(<TermsGate {...props} />);
  return { onAccepted };
}

describe("TermsGate", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it("disables Accept until the consent box is checked", async () => {
    renderGate();
    const accept = screen.getByRole("button", { name: "Accept & Continue" });
    expect(accept).toBeDisabled();

    await userEvent.click(screen.getByRole("checkbox"));
    expect(accept).toBeEnabled();
  });

  it("does not invoke accept_terms while the box is unchecked", async () => {
    renderGate();
    // The button is disabled, but assert the handler itself is a no-op even if
    // a click slips through, so the guard isn't purely CSS.
    await userEvent.click(screen.getByRole("button", { name: "Accept & Continue" }));
    expect(invokeMock).not.toHaveBeenCalledWith("accept_terms", expect.anything());
  });

  it("persists acceptance with the required version and calls onAccepted", async () => {
    const { onAccepted } = renderGate({ requiredVersion: 5 });
    await userEvent.click(screen.getByRole("checkbox"));
    await userEvent.click(screen.getByRole("button", { name: "Accept & Continue" }));

    expect(invokeMock).toHaveBeenCalledWith("accept_terms", { version: 5 });
    expect(onAccepted).toHaveBeenCalledTimes(1);
  });

  it("renders bundled Mac AI Switchboard terms without opening an external URL", () => {
    renderGate();
    expect(
      screen.getByRole("heading", { name: "Mac AI Switchboard Terms of Use" })
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Terms of Use summary")).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith(
      "open_external_link",
      expect.anything()
    );
  });

  it("shows a saving state and blocks repeat clicks while persistence is in flight", async () => {
    let resolveAccept: () => void = () => {};
    invokeMock.mockImplementation((command: string) => {
      if (command === "accept_terms") {
        return new Promise<void>((resolve) => {
          resolveAccept = resolve;
        });
      }
      return Promise.resolve();
    });
    const { onAccepted } = renderGate();
    await userEvent.click(screen.getByRole("checkbox"));

    const accept = screen.getByRole("button", { name: "Accept & Continue" });
    await userEvent.click(accept);

    const saving = screen.getByRole("button", { name: "Saving…" });
    expect(saving).toBeDisabled();
    await userEvent.click(saving);
    const acceptCalls = invokeMock.mock.calls.filter(([cmd]) => cmd === "accept_terms");
    expect(acceptCalls).toHaveLength(1);

    resolveAccept();
    await vi.waitFor(() => expect(onAccepted).toHaveBeenCalledTimes(1));
  });

  it("re-enables the button so the user can retry when persistence fails", async () => {
    invokeMock.mockRejectedValueOnce(new Error("keychain write failed"));
    const { onAccepted } = renderGate();
    await userEvent.click(screen.getByRole("checkbox"));
    await userEvent.click(screen.getByRole("button", { name: "Accept & Continue" }));

    expect(onAccepted).not.toHaveBeenCalled();
    await vi.waitFor(() =>
      expect(screen.getByRole("button", { name: "Accept & Continue" })).toBeEnabled()
    );
  });
});
