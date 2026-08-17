import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AddonCard } from "./AddonCard";
import type { ClientConnectorStatus } from "../lib/types";

const connector: ClientConnectorStatus = {
  clientId: "codex",
  name: "Codex",
  installed: true,
  enabled: true,
  verified: true,
};

function renderCard(overrides: Partial<React.ComponentProps<typeof AddonCard>> = {}) {
  const props: React.ComponentProps<typeof AddonCard> = {
    name: "RTK",
    version: "1.2.3",
    installed: false,
    enabled: false,
    description: "Compresses command output.",
    copy: { whatItDoes: "Keeps terminal context lean." },
    infoOpen: false,
    onToggleInfo: vi.fn(),
    busy: false,
    busyLabel: null,
    resultMessage: null,
    onDismissResult: vi.fn(),
    sourceUrl: "https://example.test/rtk",
    onOpenSource: vi.fn(),
    connectors: [connector],
    showClients: true,
    actionsDisabled: false,
    onInstall: vi.fn(),
    onToggleEnabled: vi.fn(),
    onUninstall: vi.fn(),
    ...overrides,
  };
  return { ...render(<AddonCard {...props} />), props };
}

describe("AddonCard", () => {
  it("wires info, source, install, and client status controls", async () => {
    const user = userEvent.setup();
    const { props } = renderCard();

    const info = screen.getByRole("button", { name: "What RTK does" });
    expect(info).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByText("Codex")).toBeVisible();

    await user.click(info);
    await user.click(screen.getByRole("button", { name: "https://example.test/rtk" }));
    await user.click(screen.getByRole("button", { name: "Install" }));

    expect(props.onToggleInfo).toHaveBeenCalledOnce();
    expect(props.onOpenSource).toHaveBeenCalledOnce();
    expect(props.onInstall).toHaveBeenCalledOnce();
  });

  it("shows installed actions, formatted versions, and result dismissal", async () => {
    const user = userEvent.setup();
    const { props } = renderCard({
      installed: true,
      enabled: true,
      infoOpen: true,
      resultMessage: "Installed successfully.",
    });

    expect(screen.getByText("v1.2.3")).toBeVisible();
    expect(screen.getByText("Enabled")).toBeVisible();
    expect(screen.getByText("Keeps terminal context lean.")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Disable" }));
    await user.click(screen.getByRole("button", { name: "Uninstall" }));
    await user.click(screen.getByRole("button", { name: "Dismiss" }));

    expect(props.onToggleEnabled).toHaveBeenCalledOnce();
    expect(props.onUninstall).toHaveBeenCalledOnce();
    expect(props.onDismissResult).toHaveBeenCalledOnce();
  });

  it("prioritizes busy progress and disables every mutation", () => {
    renderCard({ installed: true, busy: true, busyLabel: "Updating…", resultMessage: "Old result", actionsDisabled: true });

    expect(screen.getByText("Updating…")).toBeVisible();
    expect(screen.queryByText("Old result")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Enable" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Uninstall" })).toBeDisabled();
  });
});
