import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { SavingsChartTooltip } from "./SavingsChartTooltip";
import type { SavingsChartDatum } from "../lib/dashboardHelpers";

const aggregatePoint: SavingsChartDatum = {
  bucketKey: "today",
  bucketLabel: "Today",
  estimatedSavingsUsd: 2.5,
  actualCostUsd: 1.25,
  estimatedTokensSaved: 2_000,
  totalTokensSent: 1_000,
  totalCostBeforeOptimization: 3.75,
  totalTokensBeforeOptimization: 3_000,
};

describe("SavingsChartTooltip", () => {
  it("renders nothing unless the chart supplies an active data point", () => {
    const { container, rerender } = render(<SavingsChartTooltip active={false} chartMode="usd" />);
    expect(container).toBeEmptyDOMElement();
    rerender(<SavingsChartTooltip active payload={[]} chartMode="usd" />);
    expect(container).toBeEmptyDOMElement();
  });

  it("renders aggregate dollar and token fallbacks", () => {
    const { rerender } = render(
      <SavingsChartTooltip active payload={[{ payload: aggregatePoint }]} chartMode="usd" />,
    );
    expect(screen.getByText("Dollars")).toBeVisible();
    expect(screen.getByText(/Saved \$2\.50/)).toBeVisible();

    rerender(<SavingsChartTooltip active payload={[{ payload: aggregatePoint }]} chartMode="tokens" />);
    expect(screen.getByText("Tokens")).toBeVisible();
    expect(screen.getByText("Saved 2K tokens")).toBeVisible();
    expect(screen.getByText("Spent 1K tokens")).toBeVisible();
  });

  it("uses provider attribution instead of repeating aggregate totals", () => {
    render(
      <SavingsChartTooltip
        active
        chartMode="usd"
        payload={[{ payload: {
          ...aggregatePoint,
          byProvider: [{
            provider: "openai",
            estimatedSavingsUsd: 1.5,
            actualCostUsd: 0.5,
            estimatedTokensSaved: 1_200,
            totalTokensSent: 600,
          }],
        } }]}
      />,
    );
    expect(screen.getByText("Codex")).toBeVisible();
    expect(screen.getByText(/Saved \$1\.50/)).toBeVisible();
    expect(screen.queryByText("Dollars")).not.toBeInTheDocument();
  });
});
