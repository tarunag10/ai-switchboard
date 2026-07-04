import { describe, expect, it } from "vitest";

import {
  buildDoctorTimelinePreview,
  buildUpgradeIssueUrl,
  sampleManagedBlock,
} from "./appSupport";
import type { RuntimeUpgradeFailure } from "./types";

describe("app support helpers", () => {
  it("builds a visible sample managed block from a rollback record", () => {
    expect(
      sampleManagedBlock({
        id: "codex-provider",
        kind: "client_config",
        owner: "Codex",
        text: "Codex provider",
        paths: ["~/.codex/config.toml"],
        markerId: "headroom:codex_cli",
        backupPath: null,
        lastVerifiedLabel: "Verified",
        rollback: "Remove managed block.",
      }),
    ).toContain("# >>> headroom:codex_cli >>>");
  });

  it("combines doctor and managed-change timeline events with stable time", () => {
    const events = buildDoctorTimelinePreview(
      {
        status: "ok",
        summary: "All good",
        issues: [],
      },
      "Repair complete.",
      "2026-07-05T00:00:00.000Z",
    );

    expect(events.some((event) => event.id === "latest-repair-success")).toBe(
      true,
    );
    expect(events.some((event) => event.kind === "rollback")).toBe(
      true,
    );
  });

  it("encodes runtime upgrade failure details into a support issue URL", () => {
    const failure: RuntimeUpgradeFailure = {
      appVersion: "1.2.3",
      targetHeadroomVersion: "0.27.0",
      fallbackHeadroomVersion: "0.26.0",
      failurePhase: "boot_validation",
      attempts: 2,
      firstAttemptAt: "2026-07-05T00:00:00Z",
      lastAttemptAt: "2026-07-05T00:01:00Z",
      errorMessage: "readyz failed",
      rollbackRestored: true,
    };

    const url = buildUpgradeIssueUrl("https://github.com/acme/app/issues", failure);

    expect(url).toContain("https://github.com/acme/app/issues/new?");
    expect(decodeURIComponent(url)).toContain("Target Headroom: 0.27.0");
    expect(decodeURIComponent(url)).toContain("Rollback restored: yes");
  });
});
