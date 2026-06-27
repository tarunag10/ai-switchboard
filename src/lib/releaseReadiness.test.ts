import { describe, expect, it } from "vitest";

import {
  releaseReadinessCommand,
  releaseReadinessGroups,
  releaseReadinessItemCount,
  releaseReadinessRowsFromReport,
  releaseReadinessStatusCounts,
  releaseReadinessStatusRows,
} from "./releaseReadiness";

describe("release readiness checklist", () => {
  it("points users at durable release report command", () => {
    expect(releaseReadinessCommand).toBe("npm run release:ready");
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
    expect(allCopy).toMatch(/planned connector evidence/i);
    expect(allCopy).toMatch(/automation gates/i);
    expect(allCopy).toMatch(/manual workflow/i);
    expect(allCopy).toMatch(/Repo Intelligence recipes/i);
    expect(allCopy).toMatch(/per-tool agent handoffs/i);
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
      "final-gate",
    ]);

    expect(releaseReadinessStatusCounts()).toEqual({
      ready: 1,
      blocked: 6,
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
    expect(copy).toContain("npm run release:ready -- --strict");
    expect(copy).toContain("local install evidence");
    expect(copy).toContain("does not prove signed release readiness");
    expect(copy).toContain("release blockers when missing, not app failures");
  });

  it("derives ready release dashboard rows from release report JSON", () => {
    const rows = releaseReadinessRowsFromReport({
      status: "ready",
      backendValidation: { ready: true },
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
      ready: 7,
      blocked: 0,
      "local-only": 1,
    });
    expect(rows.find((row) => row.id === "final-gate")?.detail).toBe(
      "Shareable DMG is ready.",
    );
  });

  it("derives blocked and missing-artifact rows from release report JSON", () => {
    const rows = releaseReadinessRowsFromReport({
      status: "blocked",
      backendValidation: { ready: false },
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
      blocked: 7,
      "local-only": 1,
    });
    expect(rows.find((row) => row.id === "installed-smoke")?.detail).toContain(
      "planned connector evidence",
    );
    expect(rows.find((row) => row.id === "signing-env")?.detail).toContain(
      "APPLE_SIGNING_IDENTITY",
    );
    expect(rows.find((row) => row.id === "local-dmg")?.detail).toContain(
      "does not prove signed release readiness",
    );
  });
});
