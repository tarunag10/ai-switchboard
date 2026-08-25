import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

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

const cursorConnector: ClientConnectorStatus = {
  clientId: "cursor",
  name: "Cursor",
  supportStatus: "planned",
  setupPhase: "managed",
  installed: true,
  enabled: false,
  verified: false,
  setupVerification: null,
  lastConfiguredAt: null,
};

const blockedCursorAssessment = {
  schemaId: "cursor-native-provider-schema",
  supported: false,
  reason:
    "Cursor documents provider API keys in Settings > Models, but does not document a stable on-disk provider/model/base-url schema for safe third-party writes.",
  docsUrl: "https://cursor.com/help/models-and-usage/api-keys",
  surfacesDetected: 2,
  evidence: ["Cursor native schema not allowlisted."],
};

const allowlistedCursorAssessment = {
  schemaId: "cursor-native-provider-schema",
  supported: true,
  reason: "Cursor native schema is allowlisted.",
  docsUrl: "https://cursor.com/help/models-and-usage/api-keys",
  surfacesDetected: 2,
  evidence: ["Cursor native schema allowlisted."],
};

function renderPanel(
  connectors: ClientConnectorStatus[] = [{ ...codexConnector }],
  openConnectorHelpId: string | null = null,
) {
  const toggleConnector = vi.fn().mockResolvedValue(undefined);
  const copyPlannedConnectorCommand = vi.fn().mockResolvedValue(undefined);
  const setOpenConnectorHelpId = vi.fn();

  const { container, unmount } = render(
    <SettingsConnectorPanel
      connectors={connectors}
      connectorsBusy={false}
      connectorsError={null}
      verifyConnectors={vi.fn()}
      copyPlannedConnectorCommand={copyPlannedConnectorCommand}
      openConnectorHelpId={openConnectorHelpId}
      plannedConnectorCopyNotice={null}
      plannedConnectorReadiness={readiness}
      setOpenConnectorHelpId={setOpenConnectorHelpId}
      toggleConnector={toggleConnector}
    />
  );

  return {
    container,
    unmount,
    copyPlannedConnectorCommand,
    setOpenConnectorHelpId,
    toggleConnector,
  };
}

describe("SettingsConnectorPanel", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("renders cursor native-schema status on the closed card and keeps sidecar availability visible", async () => {
    invokeMock.mockResolvedValue(blockedCursorAssessment);

    const { container } = renderPanel([{ ...codexConnector }, { ...cursorConnector }]);

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
    expect(screen.getByText("Sidecar available · native gated")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Enable sidecar" })).toBeEnabled();

    const item = screen.getByText("Cursor", { selector: "h3" }).closest("article");
    expect(item).toBeTruthy();
    const card = within(item!);
    expect(card.getByText(/Native schema:/)).toBeInTheDocument();
    expect(card.getByText(/Sidecar routing and Repo Intelligence packs remain available\./)).toBeInTheDocument();
    expect(card.queryByText(/settings\.json|globalStorage|credentials|account/i)).not.toBeInTheDocument();

    expect(container.querySelector(".connector-item__native-schema")).not.toBeNull();
  });

  it("shows allowlisted Cursor schema status when the public assessment allows native writes", async () => {
    invokeMock.mockResolvedValue(allowlistedCursorAssessment);

    renderPanel([{ ...cursorConnector }]);

    await waitFor(() =>
      expect(
        screen.getByText("Cursor native schema is allowlisted."),
      ).toBeInTheDocument(),
    );
    expect(screen.getByText("Sidecar available · native gated")).toBeInTheDocument();
  });

  it("falls back to the blocked Cursor schema status when the public assessment is missing", async () => {
    invokeMock.mockResolvedValue(null);

    renderPanel([{ ...cursorConnector }]);

    await waitFor(() =>
      expect(
        screen.getByText(
          /Cursor native provider writes remain blocked until a documented on-disk schema and full lifecycle proof exist\./,
        ),
      ).toBeInTheDocument(),
    );
    expect(
      screen.getByText(/Sidecar routing remains available\./),
    ).toBeInTheDocument();
  });

  it("renders readiness metrics and connector controls", async () => {
    const user = userEvent.setup();
    const {
      container,
      copyPlannedConnectorCommand,
      setOpenConnectorHelpId,
      toggleConnector,
    } =
      renderPanel([{ ...codexConnector }]);

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
    expect(helpButton).toHaveAttribute(
      "aria-controls",
      "connector-setup-details-codex",
    );
    expect(screen.getByRole("button", { name: "Enable" })).toBeEnabled();
    await user.click(helpButton!);
    expect(setOpenConnectorHelpId).toHaveBeenCalled();

    await user.click(screen.getByRole("switch", { name: /codex/i }));
    expect(toggleConnector).toHaveBeenCalledWith(
      expect.objectContaining({ clientId: "codex" }),
      true
    );
  });

  it("keeps technical connector evidence behind setup-details disclosure", () => {
    const { unmount } = renderPanel();

    expect(
      screen.getByRole("button", {
        name: "Show setup details for Codex",
      }),
    ).toHaveAttribute("aria-expanded", "false");
    expect(
      screen.queryByText(/Headroom writes a managed provider block/i),
    ).not.toBeInTheDocument();

    unmount();
    renderPanel([{ ...codexConnector }], "codex");

    expect(
      screen.getByRole("button", {
        name: "Hide setup details for Codex",
      }),
    ).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByText(/Headroom writes a managed provider block/i),
    ).toBeInTheDocument();
  });

  it("keeps gated connector paths and safety reasons collapsed by default", () => {
    const gatedGrok = {
      clientId: "grok_cli",
      name: "Grok / xAI",
      installed: false,
      supportStatus: "planned" as const,
    };
    const { unmount } = renderPanel([
      { ...codexConnector },
      gatedGrok as ClientConnectorStatus,
    ]);

    expect(screen.queryByText(/~\/\.grok\/config\.toml/i)).not.toBeInTheDocument();

    unmount();
    renderPanel([{ ...codexConnector }, gatedGrok as ClientConnectorStatus], "grok_cli");

    expect(screen.getAllByText(/~\/\.grok\/config\.toml/i).length).toBeGreaterThan(0);
  });

  it("disables connector toggles while connector state is busy", () => {
    render(
      <SettingsConnectorPanel
        connectors={[codexConnector]}
        connectorsBusy={true}
        connectorsError="Could not refresh connectors."
        verifyConnectors={vi.fn()}
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
