import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { RepoMapView } from "./RepoMapView";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({
    repoPath: "/Users/tarunagarwal/Developer/Codex-Repos/mac-ai-switchboard",
    exists: true,
    isDirectory: true,
    hasPackageJson: true,
    hasCargoManifest: true,
    tools: [],
  }),
}));

function getDisclosureButton(controls: string) {
  return screen
    .getAllByRole("button")
    .find((button) => button.getAttribute("aria-controls") === controls);
}

describe("RepoMapView progressive disclosure", () => {
  it("keeps health cards visible while graph diagnostics are collapsed", async () => {
    const user = userEvent.setup();
    render(
      <RepoMapView
        onOpenDoctor={vi.fn()}
        onOpenRepoIntelligence={vi.fn()}
      />,
    );

    expect(screen.getByText("Graphify graph")).toBeInTheDocument();
    expect(screen.queryByText("Tool Status")).not.toBeInTheDocument();
    expect(screen.queryByText("Tool Checks")).not.toBeInTheDocument();

    const diagnosticsButton = getDisclosureButton("repo-map-graph-diagnostics");
    expect(diagnosticsButton).toHaveAttribute("aria-expanded", "false");
    expect(diagnosticsButton).toHaveTextContent("Learn more");

    await user.click(diagnosticsButton!);
    expect(diagnosticsButton).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Tool Status")).toBeInTheDocument();
    expect(screen.getByText("Tool Checks")).toBeInTheDocument();
  });
});
