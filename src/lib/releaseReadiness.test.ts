import { describe, expect, it } from "vitest";

import {
  formatReleaseReadinessCommandCopy,
  formatReleaseReadinessNextAction,
  formatReleaseReadinessReportSnapshot,
  formatReleaseReadinessSourceLabel,
  releaseReadinessCommand,
  releaseReadinessEvidenceSummary,
  releaseReadinessGroups,
  releaseReadinessItemCount,
  releaseReadinessNextAction,
  releaseReadinessRowsFromReport,
  releaseReadinessStatusCounts,
  releaseReadinessStatusRows,
} from "./releaseReadiness";

describe("release readiness checklist", () => {
  it("points users at durable release report command", () => {
    expect(releaseReadinessCommand).toBe("npm run release:ready");
  });

  it("formats a safe release command copy when no report is loaded", () => {
    const copy = formatReleaseReadinessCommandCopy();

    expect(copy).toContain("Mac AI Switchboard release readiness command");
    expect(copy).toContain("Refresh report: npm run release:ready");
    expect(copy).toContain(
      "Strict public-release gate: npm run release:ready -- --strict",
    );
    expect(copy).toContain("dist/release-readiness-report.json");
    expect(copy).toContain("npm run build:mac:local-install");
    expect(copy).toContain(
      "local unsigned/ad-hoc install evidence never replaces signed DMG install",
    );
  });

  it("labels release report source without treating defaults as proof", () => {
    expect(
      formatReleaseReadinessSourceLabel("dist/release-readiness-report.json"),
    ).toBe("Loaded report: dist/release-readiness-report.json");
    expect(formatReleaseReadinessSourceLabel(null)).toContain(
      "checklist defaults are guidance, not release proof",
    );
    expect(formatReleaseReadinessSourceLabel(undefined)).toContain(
      "Run npm run release:ready",
    );
  });

  it("covers environment, signing, static preflight, and installed-app smoke gates", () => {
    expect(releaseReadinessGroups.map((group) => group.id)).toEqual([
      "environment",
      "signing",
      "smoke",
    ]);
    expect(releaseReadinessItemCount()).toBe(10);

    const allCopy = releaseReadinessGroups
      .flatMap((group) => group.items)
      .map((item) => `${item.label} ${item.detail}`)
      .join(" ");

    expect(allCopy).toMatch(/cargo|rustup/i);
    expect(allCopy).toMatch(/Developer ID/i);
    expect(allCopy).toMatch(/notarization|App Store Connect/i);
    expect(allCopy).toMatch(/signed DMG/i);
    expect(allCopy).toMatch(/notarized DMG/i);
    expect(allCopy).toMatch(/smoke:preflight/i);
    expect(allCopy).toMatch(/smoke-preflight-summary\.md/i);
    expect(allCopy).toMatch(/npm run smoke:installed/i);
    expect(allCopy).toMatch(/installed-smoke-summary\.md/i);
    expect(allCopy).toMatch(/managed connector evidence/i);
    expect(allCopy).toMatch(/planned connector automation gates/i);
    expect(allCopy).toMatch(/automation gates/i);
    expect(allCopy).toMatch(/Repo Intelligence recipes/i);
    expect(allCopy).toMatch(/per-tool agent handoffs/i);
    expect(allCopy).toMatch(/connector readiness payload/i);
    expect(allCopy).toMatch(/beta-smoke-test\.md/i);
  });

  it("keeps release blockers tied to runnable next-action commands", () => {
    const commands = releaseReadinessGroups
      .flatMap((group) => group.items)
      .map((item) => item.command);

    expect(commands.every(Boolean)).toBe(true);
    expect(commands).toContain(
      "rustup --version && cargo --version && rustup target add aarch64-apple-darwin x86_64-apple-darwin",
    );
    expect(commands).toContain("npm run smoke:preflight");
    expect(commands).toContain("npm run build:mac:dmg");
    expect(commands).toContain("npm run release:ready -- --strict");
  });

  it("keeps checklist entries concrete enough for release handoff", () => {
    for (const group of releaseReadinessGroups) {
      expect(group.title.length).toBeGreaterThan(4);
      expect(group.items.length).toBeGreaterThanOrEqual(3);
      for (const item of group.items) {
        expect(item.label.length).toBeGreaterThan(5);
        expect(item.detail.length).toBeGreaterThan(40);
        expect(item.command?.length).toBeGreaterThan(10);
      }
    }
  });

  it("summarizes release dashboard status rows for every roadmap surface", () => {
    expect(releaseReadinessStatusRows.map((row) => row.id)).toEqual([
      "frontend-build",
      "desktop-tests",
      "local-dmg",
      "installed-smoke",
      "signing-env",
      "notarization-env",
      "updater-config",
      "connector-config-plan",
      "final-gate",
    ]);

    expect(releaseReadinessStatusCounts()).toEqual({
      ready: 1,
      blocked: 7,
      "local-only": 1,
    });

    const copy = releaseReadinessStatusRows
      .map((row) => `${row.label} ${row.statusLabel} ${row.source} ${row.detail}`)
      .join(" ");

    expect(copy).toContain("npm run build");
    expect(copy).toContain("npm run fmt:desktop && npm run test:desktop");
    expect(copy).toContain("npm run build:mac:local-install");
    expect(copy).toContain("npm run smoke:installed -- --confirm");
    expect(copy).toContain("Developer ID");
    expect(copy).toContain("notarization credentials");
    expect(copy).toContain("HEADROOM_UPDATER_PUBLIC_KEY");
    expect(copy).toMatch(/planned connector smoke evidence/i);
    expect(copy).toMatch(/gated config creation plan/i);
    expect(copy).toMatch(/connector readiness payload/i);
    expect(copy).toContain("npm run release:ready -- --strict");
    expect(copy).toContain("local install evidence");
    expect(copy).toContain("does not prove signed release readiness");
    expect(copy).toContain("release blockers when missing, not app failures");
  });

  it("summarizes release evidence coverage separately from local-only proof", () => {
    const defaults = releaseReadinessEvidenceSummary();
    expect(defaults).toMatchObject({
      totalRows: 9,
      readyRows: 1,
      blockedRows: 7,
      localOnlyRows: 1,
      publicGateReady: false,
      reportLoaded: false,
    });
    expect(defaults.copy).toContain("no release report loaded");

    const report = {
      status: "ready",
      backendValidation: { ready: true },
      staticSmokePreflight: {
        ready: true,
        evidenceReady: true,
        requiredEvidence: [
          "Planned connector config creation plan",
          "Connector readiness payload in agent handoffs",
        ],
      },
      installedSmoke: {
        installedAppPresent: true,
        evidenceReady: true,
        missingEvidence: [],
      },
      shareableDmgGate: {
        ready: true,
        signedAndNotarized: true,
        updaterFeedReady: true,
      },
    };
    const rows = releaseReadinessRowsFromReport(report);
    const loaded = releaseReadinessEvidenceSummary(rows, report);

    expect(loaded).toMatchObject({
      readyRows: 8,
      blockedRows: 0,
      localOnlyRows: 1,
      publicGateReady: true,
      reportLoaded: true,
    });
    expect(loaded.copy).toContain("public gate ready");
  });

  it("selects the next blocked release action from scripted rows", () => {
    const action = releaseReadinessNextAction();

    expect(action).toEqual({
      rowId: "desktop-tests",
      label: "Desktop tests",
      command: "npm run fmt:desktop && npm run test:desktop",
      detail:
        "Rust formatting and desktop tests must run locally or in CI before public release.",
    });
    expect(formatReleaseReadinessNextAction(action)).toContain(
      "Next release action: Desktop tests",
    );
    expect(formatReleaseReadinessNextAction(action)).toContain(
      "Command: npm run fmt:desktop && npm run test:desktop",
    );
  });

  it("reports no blocked next action for a ready public release report", () => {
    const report = {
      status: "ready",
      backendValidation: { ready: true },
      staticSmokePreflight: {
        ready: true,
        evidenceReady: true,
        requiredEvidence: [
          "Planned connector config creation plan",
          "Connector readiness payload in agent handoffs",
        ],
      },
      installedSmoke: {
        installedAppPresent: true,
        evidenceReady: true,
        missingEvidence: [],
      },
      shareableDmgGate: {
        ready: true,
        signedAndNotarized: true,
        updaterFeedReady: true,
      },
    };
    const action = releaseReadinessNextAction(
      releaseReadinessRowsFromReport(report),
    );

    expect(action).toBeNull();
    expect(formatReleaseReadinessNextAction(action)).toContain(
      "no blocked scripted rows",
    );
  });

  it("derives ready release dashboard rows from release report JSON", () => {
    const rows = releaseReadinessRowsFromReport({
      status: "ready",
      backendValidation: { ready: true },
      staticSmokePreflight: {
        ready: true,
        requiredEvidence: [
          "Planned connector config creation plan",
          "Connector readiness payload in agent handoffs",
        ],
      },
      installedSmoke: {
        installedAppPresent: true,
        evidenceReady: true,
        missingEvidence: [],
      },
      shareableDmgGate: {
        ready: true,
        signedAndNotarized: true,
        updaterFeedReady: true,
        staticSmokePreflightReady: true,
        installedAppSmokeReady: true,
        message: "Shareable DMG is ready.",
      },
      releaseEnv: {
        ok: true,
        blockers: [],
        warnings: [],
      },
    });

    expect(releaseReadinessStatusCounts(rows)).toEqual({
      ready: 8,
      blocked: 0,
      "local-only": 1,
    });
    expect(rows.find((row) => row.id === "connector-config-plan")?.detail).toContain(
      "includes the planned connector config creation plan and connector readiness payload",
    );
    expect(rows.find((row) => row.id === "final-gate")?.detail).toBe(
      "Shareable DMG is ready.",
    );
  });

  it("derives blocked and missing-artifact rows from release report JSON", () => {
    const rows = releaseReadinessRowsFromReport({
      status: "blocked",
      backendValidation: { ready: false },
      staticSmokePreflight: {
        ready: false,
        requiredEvidence: [],
        missingEvidence: ["Savings calculator copyable ledger"],
        evidenceReady: false,
      },
      installedSmoke: {
        installedAppPresent: false,
        evidenceReady: false,
        missingEvidence: ["planned connector evidence", "installed smoke summary"],
      },
      shareableDmgGate: {
        ready: false,
        signedAndNotarized: false,
        updaterFeedReady: false,
        staticSmokePreflightReady: false,
        installedAppSmokeReady: false,
        message: "Shareable DMG is blocked.",
      },
      releaseEnv: {
        ok: false,
        blockers: [{ label: "missing environment: APPLE_SIGNING_IDENTITY" }],
        warnings: [],
      },
    });

    expect(releaseReadinessStatusCounts(rows)).toEqual({
      ready: 0,
      blocked: 8,
      "local-only": 1,
    });
    expect(rows.find((row) => row.id === "frontend-build")?.detail).toContain(
      "Savings calculator copyable ledger",
    );
    expect(rows.find((row) => row.id === "installed-smoke")?.detail).toContain(
      "planned connector evidence",
    );
    expect(rows.find((row) => row.id === "signing-env")?.detail).toContain(
      "APPLE_SIGNING_IDENTITY",
    );
    expect(rows.find((row) => row.id === "local-dmg")?.detail).toContain(
      "does not prove signed release readiness",
    );
    expect(rows.find((row) => row.id === "connector-config-plan")?.detail).toContain(
      "must include the planned connector config creation plan and connector readiness payload",
    );
  });

  it("does not mark connector config-plan evidence ready before smoke preflight is ready", () => {
    const report = {
      status: "blocked",
      backendValidation: { ready: true },
      staticSmokePreflight: {
        ready: false,
        requiredEvidence: [
          "Planned connector config creation plan",
          "Connector readiness payload in agent handoffs",
        ],
      },
      installedSmoke: {
        installedAppPresent: true,
        evidenceReady: false,
        missingEvidence: [],
      },
      shareableDmgGate: {
        ready: false,
        signedAndNotarized: true,
        updaterFeedReady: true,
        staticSmokePreflightReady: false,
        installedAppSmokeReady: false,
      },
      releaseEnv: {
        ok: true,
        blockers: [],
        warnings: [],
      },
    };
    const rows = releaseReadinessRowsFromReport(report);
    const snapshot = formatReleaseReadinessReportSnapshot(
      report,
      "dist/release-readiness-report.json",
    );

    const row = rows.find((item) => item.id === "connector-config-plan");
    expect(row?.tone).toBe("blocked");
    expect(row?.statusLabel).toBe("Blocked");
    expect(snapshot).toContain("Connector config plan evidence: no");
    expect(snapshot).toContain("Evidence summary:");
    expect(snapshot).toContain("Next release action:");
  });

  it("keeps frontend blocked when direct static evidence is missing", () => {
    const rows = releaseReadinessRowsFromReport({
      status: "blocked",
      backendValidation: { ready: true },
      staticSmokePreflight: {
        ready: true,
        requiredEvidence: [
          "Planned connector config creation plan",
          "Connector readiness payload in agent handoffs",
        ],
        missingEvidence: ["Savings calculator copyable ledger"],
        evidenceReady: false,
      },
      installedSmoke: {
        installedAppPresent: true,
        evidenceReady: true,
        missingEvidence: [],
      },
      shareableDmgGate: {
        ready: false,
        signedAndNotarized: true,
        updaterFeedReady: true,
        staticSmokePreflightReady: true,
        installedAppSmokeReady: true,
      },
      releaseEnv: {
        ok: true,
        blockers: [],
        warnings: [],
      },
    });

    const row = rows.find((item) => item.id === "frontend-build");
    expect(row?.tone).toBe("blocked");
    expect(row?.statusLabel).toBe("Blocked");
    expect(row?.detail).toContain("Savings calculator copyable ledger");
  });

  it("formats a copyable release report snapshot from report JSON", () => {
    const snapshot = formatReleaseReadinessReportSnapshot(
      {
        generatedAt: "2026-06-28T00:00:00.000Z",
        status: "blocked",
        backendValidation: { ready: true },
        staticSmokePreflight: {
          ready: true,
          requiredEvidence: [
            "Planned connector config creation plan",
            "Connector readiness payload in agent handoffs",
          ],
        },
        installedSmoke: {
          ready: false,
          installedAppPresent: true,
          evidenceReady: false,
          missingEvidence: ["Codex compression recovery"],
        },
        shareableDmgGate: {
          ready: false,
          signedAndNotarized: false,
          updaterFeedReady: false,
          staticSmokePreflightReady: true,
          installedAppSmokeReady: false,
          message: "Do not share a public DMG until every gate is clear.",
        },
        releaseEnv: {
          ok: false,
          blockers: [
            { label: "missing environment: APPLE_SIGNING_IDENTITY" },
            { label: "missing notarization credentials" },
            { label: "missing HEADROOM_UPDATER_PUBLIC_KEY" },
          ],
          warnings: [{ label: "missing HEADROOM_UPDATER_PUBLIC_KEY" }],
        },
      },
      "dist/release-readiness-report.json",
    );

    expect(snapshot).toContain("# Mac AI Switchboard Release Readiness");
    expect(snapshot).toContain("Source: dist/release-readiness-report.json");
    expect(snapshot).toContain("Status: blocked");
    expect(snapshot).toContain("## Commands");
    expect(snapshot).toContain("Refresh report: npm run release:ready");
    expect(snapshot).toContain(
      "Strict public-release gate: npm run release:ready -- --strict",
    );
    expect(snapshot).toContain(
      "Local-only install evidence: npm run build:mac:local-install",
    );
    expect(snapshot).toContain("Installed app present: yes");
    expect(snapshot).toContain("Connector config plan evidence: yes");
    expect(snapshot).toContain("Signed and notarized: no");
    expect(snapshot).toContain("## Evidence Boundary");
    expect(snapshot).toContain("Public signed/notarized release proof: not ready");
    expect(snapshot).toContain(
      "Local unsigned/ad-hoc install evidence: present, local-only",
    );
    expect(snapshot).toContain(
      "Local unsigned/ad-hoc evidence never replaces signed DMG install, notarization, updater feed, or installed smoke confirmation.",
    );
    expect(snapshot).toContain(
      "Sharing status: blocked until strict release gate is ready",
    );
    expect(snapshot).toContain("## Status Rows");
    expect(snapshot).toContain("## Planned Connector Readiness");
    expect(snapshot).toContain("Planned connectors: 9");
    expect(snapshot).toContain("Automation ready: 0");
    expect(snapshot).toContain("Backup Implemented (9)");
    expect(snapshot).toContain("Cursor: Guide, next gate Backup Implemented");
    expect(snapshot).toContain(
      "Full per-tool dossiers are available from Doctor's connector dossier copy action.",
    );
    expect(snapshot).toContain(
      "Local DMG: Installed locally (local-only) via npm run build:mac:local-install. A local installed app exists, but local evidence is separate from signed release readiness.",
    );
    expect(snapshot).toContain(
      "Final release gate: Blocked (blocked) via npm run release:ready -- --strict. Do not share a public DMG until every gate is clear.",
    );
    expect(snapshot).toContain("missing environment: APPLE_SIGNING_IDENTITY");
    expect(snapshot).toContain(
      "Signing blockers: missing environment: APPLE_SIGNING_IDENTITY",
    );
    expect(snapshot).toContain(
      "Notarization blockers: missing notarization credentials",
    );
    expect(snapshot).toContain(
      "Updater blockers: missing HEADROOM_UPDATER_PUBLIC_KEY",
    );
    expect(snapshot).toContain(
      "Missing signing, notarization, or updater secrets are release blockers, not app failures.",
    );
    expect(snapshot).toContain("Codex compression recovery");
    expect(snapshot).toContain(
      "Do not share a public DMG until every gate is clear.",
    );
  });

  it("marks copied release snapshots shareable only when strict gates are ready", () => {
    const snapshot = formatReleaseReadinessReportSnapshot(
      {
        generatedAt: "2026-06-28T00:00:00.000Z",
        status: "ready",
        backendValidation: { ready: true },
        staticSmokePreflight: {
          ready: true,
          evidenceReady: true,
          requiredEvidence: [
            "Planned connector config creation plan",
            "Connector readiness payload in agent handoffs",
          ],
          missingEvidence: [],
        },
        installedSmoke: {
          ready: true,
          installedAppPresent: true,
          evidenceReady: true,
          missingEvidence: [],
        },
        shareableDmgGate: {
          ready: true,
          signedAndNotarized: true,
          updaterFeedReady: true,
          staticSmokePreflightReady: true,
          installedAppSmokeReady: true,
        },
        releaseEnv: {
          ok: true,
          blockers: [],
          warnings: [],
        },
      },
      "dist/release-readiness-report.json",
    );

    expect(snapshot).toContain("Public signed/notarized release proof: ready");
    expect(snapshot).toContain(
      "Local unsigned/ad-hoc install evidence: present, local-only",
    );
    expect(snapshot).toContain("Sharing status: shareable");
    expect(snapshot).toContain("Shareable DMG ready: yes");
  });
});
