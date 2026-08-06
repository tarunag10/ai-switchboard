import { describe, expect, it } from "vitest";

import {
  describeGodFileRegistry,
  findGodFileRegistryEntry,
  godFileLineCeiling,
  godFileRegistry,
  godFileRegistryPaths,
  trackedOversizeRegistryPaths,
  validateGodFileRegistry,
  watchlistRegistryPaths,
} from "./godFileRegistry";

describe("godFileRegistry", () => {
  it("tracks the three known god files", () => {
    expect(godFileRegistryPaths()).toEqual([
      "src-tauri/src/client_adapters.rs",
      "src/App.tsx",
      "src/styles.css",
    ]);
  });

  it("validates registry shape", () => {
    expect(validateGodFileRegistry()).toEqual([]);
  });

  it("tracks watchlist frontier files", () => {
    expect(watchlistRegistryPaths()).toContain(
      "src/components/OptimizationDashboard.tsx",
    );
    expect(trackedOversizeRegistryPaths()).toHaveLength(4);
  });

  it("computes growth ceilings per entry", () => {
    const entry = findGodFileRegistryEntry("src/App.tsx");
    expect(entry).not.toBeNull();
    expect(godFileLineCeiling(entry!)).toBe(entry!.baselineLines + entry!.maxGrowthLines);
  });

  it("describes the registry for maintainability copy", () => {
    expect(describeGodFileRegistry()).toMatch(/client_adapters\.rs/);
    expect(describeGodFileRegistry()).toMatch(/P3\.3/);
  });
});
