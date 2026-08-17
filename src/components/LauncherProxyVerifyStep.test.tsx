import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { LauncherProxyVerifyStep } from "./LauncherProxyVerifyStep";
import type { ProxyVerificationRow } from "../lib/proxyVerification";

const waiting: ProxyVerificationRow = {
  clientId: "codex",
  name: "Codex",
  state: "waiting",
  message: "Waiting for a prompt",
  oneClickSupported: true,
};

function renderStep(overrides: Partial<React.ComponentProps<typeof LauncherProxyVerifyStep>> = {}) {
  const props: React.ComponentProps<typeof LauncherProxyVerifyStep> = {
    appSemver: "1.0",
    proxyVerificationRows: [waiting],
    proxyVerificationHint: null,
    connectorSmokeBusyId: null,
    onMouseDown: vi.fn(),
    onBack: vi.fn(),
    onContinue: vi.fn(),
    runAllSupportedConnectorSmokeTests: vi.fn(),
    runConnectorSmokeTest: vi.fn(),
    ...overrides,
  };
  return { ...render(<LauncherProxyVerifyStep {...props} />), props };
}

describe("LauncherProxyVerifyStep", () => {
  it("runs all and per-client smoke tests and navigation callbacks", async () => {
    const user = userEvent.setup();
    const { props } = renderStep();
    await user.click(screen.getByRole("button", { name: "Send all test prompts" }));
    await user.click(screen.getByRole("button", { name: "Send test prompt" }));
    await user.click(screen.getByRole("button", { name: "Back" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(props.runAllSupportedConnectorSmokeTests).toHaveBeenCalledOnce();
    expect(props.runConnectorSmokeTest).toHaveBeenCalledWith(waiting);
    expect(props.onBack).toHaveBeenCalledOnce();
    expect(props.onContinue).toHaveBeenCalledOnce();
  });

  it("locks test controls while busy and marks completed clients verified", () => {
    renderStep({
      connectorSmokeBusyId: "codex",
      proxyVerificationHint: "Restart Codex first.",
      proxyVerificationRows: [waiting, { ...waiting, clientId: "claude", name: "Claude Code", state: "verified" }],
    });
    expect(screen.getByRole("button", { name: "Sending test prompts..." })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Sending..." })).toBeDisabled();
    expect(screen.getByText("verified")).toBeVisible();
    expect(screen.getByText("Restart Codex first.")).toBeVisible();
  });

  it("explains an empty selection and omits unsupported automation", () => {
    renderStep({ proxyVerificationRows: [] });
    expect(screen.getByText(/No tools are enabled yet/)).toBeVisible();
    expect(screen.queryByRole("button", { name: /test prompt/i })).not.toBeInTheDocument();
  });
});
