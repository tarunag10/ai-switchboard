import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SettingsReleaseReadinessCard } from "./SettingsReleaseReadinessCard";

function renderCard(overrides = {}) {
  const props = {
    copyReleaseReadinessReport: vi.fn(),
    formatLocalReleaseEvidenceSequenceCopy: vi.fn(() => "Run local evidence"),
    refreshReleaseReadinessReport: vi.fn(),
    releaseEvidenceBusyId: null,
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
    releaseReadinessEvidence: { copy: "Local evidence copy" },
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
    });

    expect(
      screen.getByRole("button", { name: /copy report snapshot/i }),
    ).toBeInTheDocument();
  });
});
