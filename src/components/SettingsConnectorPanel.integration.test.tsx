import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsConnectorPanel } from "./SettingsConnectorPanel";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

const readiness = { headline: "Ready", detail: "Connector evidence", detectedCount: 2, manualOnlyCount: 1, notDetectedCount: 0, safeTodayCount: 1, automationGateCount: 1 };
const codex = { clientId: "codex", name: "Codex", supportStatus: "managed", setupPhase: "managed", installed: true, enabled: true, verified: false, setupVerification: null, lastConfiguredAt: null } as any;
const cursor = { clientId: "cursor", name: "Cursor", supportStatus: "planned", setupPhase: "manual", installed: true, enabled: false, verified: false, setupVerification: null, lastConfiguredAt: null } as any;

function renderPanel(overrides: Record<string, unknown> = {}) {
  const p = { connectors: [codex, cursor], plannedConnectorReadiness: readiness, plannedConnectorCopyNotice: null, connectorsBusy: false, connectorsError: null,
    openConnectorHelpId: null, setOpenConnectorHelpId: vi.fn(), toggleConnector: vi.fn(), copyPlannedConnectorCommand: vi.fn(), ...overrides } as any;
  return { p, ...render(<SettingsConnectorPanel {...p} />) };
}

describe("SettingsConnectorPanel alternate connector branches", () => {
  beforeEach(() => invokeMock.mockReset());

  it("copies both global plans and disables an enabled unverified connector", async () => {
    const user = userEvent.setup();
    const { p } = renderPanel({ connectors: [codex] });
    await user.click(screen.getByRole("button", { name: /Copy checks/ }));
    await user.click(screen.getByRole("button", { name: /Copy config plans/ }));
    await user.click(screen.getByRole("button", { name: "Disable" }));
    expect(p.copyPlannedConnectorCommand).toHaveBeenNthCalledWith(1, expect.stringContaining("connector"), "Connector checklist");
    expect(p.copyPlannedConnectorCommand).toHaveBeenNthCalledWith(2, expect.any(String), "Connector config plans");
    expect(p.toggleConnector).toHaveBeenCalledWith(expect.objectContaining({ clientId: "codex" }), false);
    expect(screen.getByText(/Restart Codex to start routing/)).toBeInTheDocument();
  });

  it("loads Cursor native schema evidence and copies its setup/config plans", async () => {
    const user = userEvent.setup();
    invokeMock.mockResolvedValue({ schemaVersion: "1", schemaSupported: false, protectedFieldsPreserved: true, backupRoundTripPassed: true, offCleanupPassed: true, reasons: ["schema not promoted"] });
    const { p } = renderPanel({ connectors: [cursor], openConnectorHelpId: "cursor" });
    expect(invokeMock).toHaveBeenCalledWith("get_cursor_native_schema_assessment");
    await waitFor(() => expect(screen.getByText(/Schema assessment:/)).toBeInTheDocument());
    const item = screen.getByText("Cursor", { selector: "h3" }).closest("article");
    if (!item) throw new Error("Cursor item missing");
    const card = within(item);
    await user.click(card.getByRole("button", { name: "Copy Cursor setup check command" }));
    await user.click(card.getByRole("button", { name: "Copy Cursor config creation plan" }));
    expect(p.copyPlannedConnectorCommand).toHaveBeenCalledWith(expect.any(String), "Cursor");
    expect(p.copyPlannedConnectorCommand).toHaveBeenCalledWith(expect.any(String), "Cursor config plan");
  });

});
