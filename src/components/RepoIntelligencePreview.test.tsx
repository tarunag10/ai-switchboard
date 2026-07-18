import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { RepoIntelligencePreview } from "./RepoIntelligencePreview";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

function getDisclosureButton(controls: string) {
  return screen
    .getAllByRole("button")
    .find((button) => button.getAttribute("aria-controls") === controls);
}

describe("RepoIntelligencePreview progressive disclosure", () => {
  it("hides verification and mode reasoning copy until the user expands it", async () => {
    const user = userEvent.setup();
    render(<RepoIntelligencePreview />);

    expect(
      screen.queryByText(/Doctor still verifies runtime and connector health/i),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText(/No optimization dependency is currently healthy/i),
    ).not.toBeInTheDocument();

    const detailsButton = getDisclosureButton(
      "repo-intelligence-verification-details",
    );
    expect(detailsButton).toHaveAttribute("aria-expanded", "false");
    expect(detailsButton).toHaveTextContent("Details");

    await user.click(detailsButton!);
    expect(detailsButton).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByText(/Doctor still verifies runtime and connector health/i),
    ).toBeInTheDocument();

    const reasoningButton = getDisclosureButton(
      "repo-intelligence-mode-reasoning",
    );
    expect(reasoningButton).toHaveAttribute("aria-expanded", "false");

    await user.click(reasoningButton!);
    expect(reasoningButton).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByText(/No optimization dependency is currently healthy/i),
    ).toBeInTheDocument();
  });

  it("keeps graph summary visible while graph diagnostics are collapsed", async () => {
    const user = userEvent.setup();
    render(<RepoIntelligencePreview />);

    expect(screen.getByText("Top directories")).toBeInTheDocument();
    expect(screen.queryByText("Agent graph signal")).not.toBeInTheDocument();

    const graphButton = getDisclosureButton(
      "repo-intelligence-graph-diagnostics",
    );
    expect(graphButton).toHaveAttribute("aria-expanded", "false");
    expect(graphButton).toHaveTextContent("Learn more");

    await user.click(graphButton!);
    expect(graphButton).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Agent graph signal")).toBeInTheDocument();
  });

  it("labels chonkify as a blocked evidence preview and keeps native packs", async () => {
    const user = userEvent.setup();
    render(<RepoIntelligencePreview />);

    await user.selectOptions(screen.getByRole("combobox", { name: "Repo pack compression mode" }), "chonkify");
    expect(screen.getByText(/Chonkify is selected for evidence preview only/i)).toBeInTheDocument();
    expect(screen.getByText(/license metadata is NOASSERTION/i)).toBeInTheDocument();
  });
});
