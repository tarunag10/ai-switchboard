import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { PlannedConnectorReadinessSummary } from "../lib/dashboardHelpers";
import type { ClientConnectorStatus } from "../lib/types";
import { SettingsConnectorPanel } from "./SettingsConnectorPanel";

const readiness: PlannedConnectorReadinessSummary = {
  detectedCount: 1,
  manualOnlyCount: 0,
  notDetectedCount: 0,
  safeTodayCount: 1,
  plannedCapabilityCount: 1,
  automationGateCount: 0,
  detectedNames: ["Codex"],
  notDetectedNames: [],
  headline: "1 connector ready",
  detail: "Codex is detected and can be managed safely.",
};

const codexConnector: ClientConnectorStatus = {
  clientId: "codex",
  name: "Codex",
  supportStatus: "managed",
  setupPhase: "managed",
  installed: true,
  enabled: false,
  verified: true,
  setupVerification: null,
  lastConfiguredAt: null,
};

function renderPanel(overrides: Partial<ClientConnectorStatus> = {}) {
  const toggleConnector = vi.fn().mockResolvedValue(undefined);
  const copyPlannedConnectorCommand = vi.fn().mockResolvedValue(undefined);
  const setOpenConnectorHelpId = vi.fn();

  const { container } = render(
    <SettingsConnectorPanel
      connectors={[{ ...codexConnector, ...overrides }]}
      connectorsBusy={false}
      connectorsError={null}
      copyPlannedConnectorCommand={copyPlannedConnectorCommand}
      openConnectorHelpId={null}
      plannedConnectorCopyNotice={null}
      plannedConnectorReadiness={readiness}
      setOpenConnectorHelpId={setOpenConnectorHelpId}
      toggleConnector={toggleConnector}
    />
  );

  return {
    container,
    copyPlannedConnectorCommand,
    setOpenConnectorHelpId,
    toggleConnector,
  };
}

describe("SettingsConnectorPanel", () => {
  it("renders readiness metrics and connector controls", async () => {
    const user = userEvent.setup();
    const {
      container,
      copyPlannedConnectorCommand,
      setOpenConnectorHelpId,
      toggleConnector,
    } =
      renderPanel();

    expect(screen.getByText("Connector readiness")).toBeInTheDocument();
    expect(screen.getByText("1 connector ready")).toBeInTheDocument();
    const copyButton = container.querySelector<HTMLButtonElement>(
      ".connector-readiness__copy"
    );
    expect(copyButton).not.toBeNull();
    await user.click(copyButton!);
    expect(copyPlannedConnectorCommand).toHaveBeenCalled();

    const helpButton = container.querySelector<HTMLButtonElement>(".connector-help");
    expect(helpButton).not.toBeNull();
    await user.click(helpButton!);
    expect(setOpenConnectorHelpId).toHaveBeenCalled();

    await user.click(screen.getByRole("switch", { name: /codex/i }));
    expect(toggleConnector).toHaveBeenCalledWith(
      expect.objectContaining({ clientId: "codex" }),
      true
    );
  });

  it("disables connector toggles while connector state is busy", () => {
    render(
      <SettingsConnectorPanel
        connectors={[codexConnector]}
        connectorsBusy={true}
        connectorsError="Could not refresh connectors."
        copyPlannedConnectorCommand={vi.fn()}
        openConnectorHelpId={null}
        plannedConnectorCopyNotice="Copied Codex setup."
        plannedConnectorReadiness={readiness}
        setOpenConnectorHelpId={vi.fn()}
        toggleConnector={vi.fn()}
      />
    );

    expect(screen.getByText("Could not refresh connectors.")).toBeInTheDocument();
    expect(screen.getByText("Copied Codex setup.")).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: /codex/i })).toBeDisabled();
  });
});
