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
  const p = { connectors: [codex, cursor], plannedConnectorReadiness: readiness, plannedConnectorCopyNotice: null, connectorsBusy: false, connectorsError: null, verifyConnectors: vi.fn(),
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
    await user.click(screen.getByRole("button", { name: "Verify now" }));
    await user.click(screen.getByRole("button", { name: "Disable" }));
    expect(p.copyPlannedConnectorCommand).toHaveBeenNthCalledWith(1, expect.stringContaining("connector"), "Connector checklist");
    expect(p.copyPlannedConnectorCommand).toHaveBeenNthCalledWith(2, expect.any(String), "Connector config plans");
    expect(p.toggleConnector).toHaveBeenCalledWith(expect.objectContaining({ clientId: "codex" }), false);
    expect(p.verifyConnectors).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/Restart Codex to start routing/)).toBeInTheDocument();
  });

  it("shows Cursor native schema status on the closed card from the public assessment payload", async () => {
    invokeMock.mockResolvedValue({
      schemaId: "cursor-native-provider-schema",
      supported: false,
      reason:
        "Cursor documents provider API keys in Settings > Models, but does not document a stable on-disk provider/model/base-url schema for safe third-party writes.",
      docsUrl: "https://cursor.com/help/models-and-usage/api-keys",
      surfacesDetected: 2,
      evidence: ["Cursor native settings surface: none detected yet."],
    });
    renderPanel({ connectors: [cursor] });

    expect(invokeMock).toHaveBeenCalledWith("get_cursor_native_schema_assessment");
    await waitFor(() =>
      expect(
        screen.getByText(
          /Cursor native provider writes remain blocked until a documented on-disk schema and full lifecycle proof exist\./,
        ),
      ).toBeInTheDocument(),
    );
    expect(
      screen.getByText(/Sidecar routing and Repo Intelligence packs remain available\./),
    ).toBeInTheDocument();
    const item = screen.getByText("Cursor", { selector: "h3" }).closest("article");
    if (!item) throw new Error("Cursor item missing");
    const card = within(item);
    expect(card.getByText("Sidecar available · native gated")).toBeInTheDocument();
    expect(card.getByRole("button", { name: "Enable sidecar" })).toBeEnabled();
  });

  it("exposes Cursor's safe sidecar toggle while keeping native routing gated", async () => {
    const user = userEvent.setup();
    invokeMock.mockResolvedValue({
      schemaId: "cursor-native-provider-schema",
      supported: false,
      reason:
        "Cursor documents provider API keys in Settings > Models, but does not document a stable on-disk provider/model/base-url schema for safe third-party writes.",
      docsUrl: "https://cursor.com/help/models-and-usage/api-keys",
      surfacesDetected: 2,
      evidence: ["Cursor native settings surface: none detected yet."],
    });
    const { p } = renderPanel({ connectors: [cursor] });
    const item = screen.getByText("Cursor", { selector: "h3" }).closest("article");
    if (!item) throw new Error("Cursor item missing");
    const card = within(item);

    expect(card.getByText("Sidecar available · native gated")).toBeInTheDocument();
    expect(card.getByRole("button", { name: "Enable sidecar" })).toBeEnabled();
    await user.click(card.getByRole("button", { name: "Enable sidecar" }));

    expect(p.toggleConnector).toHaveBeenCalledWith(
      expect.objectContaining({ clientId: "cursor" }),
      true,
    );
    expect(card.getByText("Sidecar available · native gated")).toBeInTheDocument();
  });

});
