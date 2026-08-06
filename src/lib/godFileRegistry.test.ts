import { describe, expect, it } from "vitest";

import {
  describeGodFileRegistry,
  findWatchlistRegistryEntry,
  godFileRegistry,
  godFileRegistryPaths,
  godFileLineCeiling,
  trackedOversizeRegistryPaths,
  splitModuleRegistryPaths,
  validateGodFileRegistry,
  watchlistRegistryPaths,
} from "./godFileRegistry";

describe("godFileRegistry", () => {
  it("has no remaining god files after P3.3 splits", () => {
    expect(godFileRegistryPaths()).toEqual([]);
  });

  it("validates registry shape", () => {
    expect(validateGodFileRegistry()).toEqual([]);
  });

  it("tracks watchlist frontier files", () => {
    expect(watchlistRegistryPaths()).toContain("src/app/TrayApp.tsx");
    expect(watchlistRegistryPaths()).toContain(
      "src-tauri/src/client_setup_apply.rs",
    );
    expect(watchlistRegistryPaths()).not.toContain(
      "src/components/OptimizationDashboard.tsx",
    );
    expect(trackedOversizeRegistryPaths()).toHaveLength(2);
  });

  it("tracks split modules from the god-file program", () => {
    expect(splitModuleRegistryPaths()).toEqual(
      expect.arrayContaining([
        "src-tauri/src/client_adapters.rs",
        "src/app/TrayApp.tsx",
        "src/components/TrayAppShell.tsx",
        "src/components/SettingsView.tsx",
        "src/components/OptimizationCompressionOverview.tsx",
        "src-tauri/src/client_codex_setup.rs",
        "src/styles/tokens.css",
        "src/styles.css",
      ]),
    );
  });

  it("computes growth ceilings for watchlist entries", () => {
    const entry = findWatchlistRegistryEntry("src/app/TrayApp.tsx");
    expect(entry).not.toBeNull();
    expect(godFileLineCeiling(entry!)).toBe(
      entry!.baselineLines + entry!.maxGrowthLines,
    );
  });

  it("describes the registry for maintainability copy", () => {
    expect(describeGodFileRegistry()).toMatch(/TrayApp\.tsx/);
    expect(describeGodFileRegistry()).toMatch(/split/);
  });
});
