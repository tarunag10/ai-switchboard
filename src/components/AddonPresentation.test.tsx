import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { AddonClientChips } from "./AddonClientChips";
import { AddonHealthStrip } from "./AddonHealthStrip";
import { ConnectorLogo } from "./ConnectorLogo";
import type { ClientConnectorStatus } from "../lib/types";

describe("add-on presentation components", () => {
  it("omits empty client chips and exposes aggregated client status accessibly", () => {
    const { container, rerender } = render(<AddonClientChips connectors={[]} />);
    expect(container).toBeEmptyDOMElement();

    const connectors: ClientConnectorStatus[] = [
      { clientId: "codex", name: "Codex", installed: true, enabled: true, verified: true },
      { clientId: "claude_code", name: "Claude Code", installed: true, enabled: true, verified: false },
    ];
    rerender(<AddonClientChips connectors={connectors} />);

    expect(screen.getByText("Codex")).toBeVisible();
    expect(screen.getByText("Claude Code")).toBeVisible();
    expect(container.querySelectorAll(".callout-banner__chip-dot")).toHaveLength(2);
  });

  it("renders health evidence, an accessible trend, and scaled sparkline points", () => {
    const { container } = render(
      <AddonHealthStrip
        cards={[{
          id: "rtk",
          name: "RTK",
          statusLabel: "Healthy",
          tone: "healthy",
          detail: "Compression is active.",
          evidence: ["100 commands observed"],
          nextAction: "No action needed",
          trend: {
            label: "Commands",
            value: "100",
            detail: "Last two days",
            points: [{ label: "Yesterday", value: 0 }, { label: "Today", value: 100 }],
          },
        }]}
      />,
    );

    expect(screen.getByLabelText("Add-on health")).toBeVisible();
    expect(screen.getByLabelText("RTK health trend")).toBeVisible();
    expect(screen.getByText("100 commands observed")).toBeVisible();
    const bars = container.querySelectorAll<HTMLElement>(".addons__health-sparkline span");
    expect(bars[0]).toHaveStyle({ height: "12%" });
    expect(bars[1]).toHaveStyle({ height: "100%" });
  });

  it("renders the connector glyph independently of client id", () => {
    const { container } = render(<ConnectorLogo clientId="future-client" />);
    expect(container.querySelector("svg.client-logo__glyph")).toBeInTheDocument();
  });
});
