import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { LauncherClientSetupStep } from "./LauncherClientSetupStep";
import type { ClientConnectorStatus } from "../lib/types";

const codex: ClientConnectorStatus = {
  clientId: "codex",
  name: "Codex",
  installed: true,
  enabled: false,
  verified: false,
};
const missing: ClientConnectorStatus = {
  clientId: "claude",
  name: "Claude Code",
  installed: false,
  enabled: false,
  verified: false,
};

function renderStep(overrides: Partial<React.ComponentProps<typeof LauncherClientSetupStep>> = {}) {
  const props: React.ComponentProps<typeof LauncherClientSetupStep> = {
    appSemver: "1.0",
    connectors: [codex, missing],
    connectorsBusy: false,
    connectorsError: null,
    openConnectorHelpId: null,
    openConnectorWarningId: null,
    setOpenConnectorHelpId: vi.fn(),
    setOpenConnectorWarningId: vi.fn(),
    setLauncherStage: vi.fn(),
    toggleConnector: vi.fn(() => Promise.resolve()),
    onMouseDown: vi.fn(),
    onContinue: vi.fn(),
    ...overrides,
  };
  return { ...render(<LauncherClientSetupStep {...props} />), props };
}

describe("LauncherClientSetupStep", () => {
  it("toggles detected clients with the next state and returns to install", async () => {
    const user = userEvent.setup();
    const { props } = renderStep();
    const toggle = screen.getByRole("switch", { name: "Enable Codex connector" });
    expect(toggle).toHaveAttribute("aria-checked", "false");
    await user.click(toggle);
    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(props.toggleConnector).toHaveBeenCalledWith(codex, true);
    expect(props.setLauncherStage).toHaveBeenCalledWith("install");
  });

  it("uses an accessible setup disclosure and shows restart/error feedback", async () => {
    const user = userEvent.setup();
    const { props } = renderStep({ connectors: [{ ...codex, enabled: true }], connectorsError: "Could not save config" });
    await user.click(screen.getByRole("button", { name: "Show setup details for Codex" }));
    const updater = vi.mocked(props.setOpenConnectorHelpId).mock.calls[0][0];
    expect(typeof updater).toBe("function");
    expect((updater as (value: string | null) => string | null)(null)).toBe("codex");
    expect(screen.getByText("Restart Codex to apply changes.")).toBeVisible();
    expect(screen.getByText("Could not save config")).toBeVisible();
  });

  it("requires one available selection but allows continuing when none are detected", async () => {
    const user = userEvent.setup();
    const { rerender, props } = renderStep({ connectors: [codex] });
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();

    rerender(<LauncherClientSetupStep {...props} connectors={[missing]} />);
    const continueButton = screen.getByRole("button", { name: "Continue" });
    expect(continueButton).toBeEnabled();
    await user.click(continueButton);
    expect(props.onContinue).toHaveBeenCalledOnce();
    expect(screen.getByText("Not detected on this machine")).toBeVisible();
  });
});
