import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { OptimizationView } from "./OptimizationView";

function renderOptimizationView() {
  return render(
    <OptimizationView
      activeView="optimization"
      setActiveView={vi.fn()}
      headroomLearnSupported={true}
      headroomLearnDisabledReason={null}
      headroomLearnPrereq={{
        claudeCliAvailable: true,
        codexCliAvailable: true,
        codexLoggedIn: true,
      }}
      headroomLearnStatus={{
        running: false,
        progressPercent: 0,
        summary: "Idle",
        outputTail: [],
      }}
      headroomLearnBusy={false}
      claudeLearnEnabled={false}
      codexLearnEnabled={false}
      claudeProjectsBusy={false}
      claudeProjects={[]}
      visibleClaudeProjects={[]}
      sortedClaudeProjects={[]}
      showAllClaudeProjects={false}
      setShowAllClaudeProjects={vi.fn()}
      handleRunHeadroomLearn={vi.fn()}
      copyLearnInstallCommand={vi.fn()}
      openLearnInstallDocsLink={vi.fn()}
      refreshHeadroomLearnPrereq={vi.fn()}
      learnInstallCopyNotice={null}
      optimizeAppliedByProject={null}
      setOptimizeAppliedRefreshTick={vi.fn()}
      claudeProjectsError={null}
      learnBlurb="Headroom learns from local coding history."
    />,
  );
}

describe("OptimizationView", () => {
  it("keeps setup instructions collapsed behind details", async () => {
    const user = userEvent.setup();
    renderOptimizationView();

    expect(screen.getByText("Learning setup")).toBeInTheDocument();
    expect(
      screen.getByText("Use enabled connectors to scan project or session history."),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/Enable Claude Code or Codex in Addons/i),
    ).not.toBeInTheDocument();

    const detailsButton = screen.getByRole("button", { name: "Details" });
    expect(detailsButton).toHaveAttribute("aria-expanded", "false");
    expect(detailsButton).toHaveAttribute(
      "aria-controls",
      "optimization-setup-details",
    );

    await user.click(detailsButton);

    expect(detailsButton).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByText(/Enable Claude Code or Codex in Addons/i),
    ).toBeInTheDocument();
  });
});
