import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SettingsReleaseReadinessCard } from "./SettingsReleaseReadinessCard";

function renderCard(overrides = {}) {
  const props = {
    copyReleaseReadinessReport: vi.fn(),
    formatLocalReleaseEvidenceSequenceCopy: vi.fn(() => "Run local evidence"),
    refreshReleaseReadinessReport: vi.fn(),
    releaseEvidenceBusyId: null,
    releaseEvidenceResult: null,
    releaseLocalEvidenceRows: [
      {
        id: "local-install",
        label: "Local install smoke",
        detail: "Install smoke summary is present.",
        statusLabel: "Ready",
        passed: true,
        command: "npm smoke:installed:local",
        summaryPath: "outputs/installed-smoke-summary.md",
      },
    ],
    releaseReadinessAction:
      "Run npm run release:ready -- --strict before sharing a DMG.",
    releaseReadinessCommandProp: "npm run release:ready -- --strict",
    releaseReadinessCopyNotice: null,
    releaseReadinessCounts: { ready: 1, blocked: 1, "local-only": 1 },
    releaseReadinessError: null,
    releaseReadinessEvidence: {
      copy: "Local evidence copy",
      reportLoaded: false,
      publicGateReady: false,
      readyRows: 1,
      blockedRows: 1,
      localOnlyRows: 1,
      totalRows: 3,
    },
    releaseReadinessRefreshing: false,
    releaseReadinessReport: null,
    releaseReadinessRows: [
      {
        id: "strict-release",
        label: "Strict release gate",
        detail: "Strict release readiness must be green.",
        source: "release:ready",
        statusLabel: "Blocked",
        tone: "blocked",
      },
    ],
    runLocalReleaseEvidenceSequence: vi.fn(),
    runReleaseEvidenceCommand: vi.fn(),
    ...overrides,
  };

  render(<SettingsReleaseReadinessCard {...props} />);
  return props;
}

describe("SettingsReleaseReadinessCard", () => {
  it("wires refresh, copy, local evidence, and per-check evidence actions", async () => {
    const user = userEvent.setup();
    const props = renderCard();

    await user.click(screen.getByRole("button", { name: /refresh report/i }));
    await user.click(
      screen.getByRole("button", { name: /run local evidence/i }),
    );
    await user.click(
      screen.getByRole("button", { name: /copy report command/i }),
    );
    await user.click(
      screen.getAllByRole("button", { name: /run evidence/i })[0],
    );

    expect(props.refreshReleaseReadinessReport).toHaveBeenCalledTimes(1);
    expect(props.runLocalReleaseEvidenceSequence).toHaveBeenCalledTimes(1);
    expect(props.copyReleaseReadinessReport).toHaveBeenCalledTimes(1);
    expect(props.runReleaseEvidenceCommand).toHaveBeenCalledTimes(1);
    expect(props.runReleaseEvidenceCommand).toHaveBeenCalledWith(
      expect.any(String),
    );
  });

  it("shows report snapshot copy label when a report is loaded", () => {
    renderCard({
      releaseReadinessReport: {
        reportPath: "dist/release-readiness-report.json",
        report: {} as never,
      },
      releaseReadinessEvidence: {
        copy: "Loaded report copy",
        reportLoaded: true,
        publicGateReady: false,
        readyRows: 1,
        blockedRows: 1,
        localOnlyRows: 1,
        totalRows: 3,
      },
    });

    expect(
      screen.getByRole("button", { name: /copy report snapshot/i }),
    ).toBeInTheDocument();
    const evidenceState = screen.getByLabelText("Release readiness evidence state");
    expect(
      within(evidenceState).getByText(/report loaded from the local checkout/i),
    ).toBeInTheDocument();
    expect(within(evidenceState).getByText("Loaded")).toBeInTheDocument();
    expect(
      within(evidenceState).getByText(/public release gate is still blocked/i),
    ).toBeInTheDocument();
    expect(within(evidenceState).getByText("Blocked")).toBeInTheDocument();
  });

  it("shows missing local evidence when no report is loaded", () => {
    renderCard({
      releaseReadinessReport: null,
      releaseReadinessEvidence: {
        copy: "No report copy",
        reportLoaded: false,
        publicGateReady: false,
        readyRows: 1,
        blockedRows: 1,
        localOnlyRows: 1,
        totalRows: 3,
      },
    });

    const evidenceState = screen.getByLabelText("Release readiness evidence state");
    expect(
      within(evidenceState).getByText(/no report loaded yet; local readiness proof is not available/i),
    ).toBeInTheDocument();
    expect(within(evidenceState).getByText("Missing")).toBeInTheDocument();
  });

  it("shows a ready public gate when the loaded summary represents it", () => {
    renderCard({
      releaseReadinessReport: {
        reportPath: "dist/release-readiness-report.json",
        report: { status: "ready" } as never,
      },
      releaseReadinessEvidence: {
        copy: "Ready report copy",
        reportLoaded: true,
        publicGateReady: true,
        readyRows: 3,
        blockedRows: 0,
        localOnlyRows: 1,
        totalRows: 4,
      },
    });

    const evidenceState = screen.getByLabelText("Release readiness evidence state");
    expect(
      within(evidenceState).getByText(/public release gate is ready in the loaded report/i),
    ).toBeInTheDocument();
    expect(within(evidenceState).getByText("Ready")).toBeInTheDocument();
  });

  it("disables checkout-only evidence actions for packaged-app payloads", () => {
    renderCard({
      releaseReadinessReport: {
        reportPath: "/Applications/AI Switchboard.app/dist/release-readiness-report.json",
        report: null,
        environment: {
          available: false,
          kind: "packaged",
          workspacePath: "/Applications/AI Switchboard.app",
          reason: "Run release evidence from a repository checkout.",
        },
      },
    });

    expect(
      screen.getByText(/Release evidence unavailable in this packaged app/i),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /refresh report/i })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: /run local evidence/i }),
    ).toBeDisabled();
    expect(screen.getAllByRole("button", { name: /run evidence/i })[0]).toBeDisabled();
  });
});
