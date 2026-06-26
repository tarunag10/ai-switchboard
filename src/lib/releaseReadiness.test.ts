import { describe, expect, it } from "vitest";

import {
  releaseReadinessCommand,
  releaseReadinessGroups,
  releaseReadinessItemCount,
} from "./releaseReadiness";

describe("release readiness checklist", () => {
  it("points users at the durable release report command", () => {
    expect(releaseReadinessCommand).toBe("npm run release:report");
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
    expect(allCopy).toMatch(/Repo Intelligence recipes/i);
expect(allCopy).toMatch(/per-tool agent handoffs/i);
    expect(allCopy).toMatch(/beta-smoke-test\.md/i);
  });

  it("keeps checklist entries concrete enough for a release handoff", () => {
    for (const group of releaseReadinessGroups) {
      expect(group.title.length).toBeGreaterThan(4);
      expect(group.items.length).toBeGreaterThanOrEqual(3);

      for (const item of group.items) {
        expect(item.label.length).toBeGreaterThan(5);
        expect(item.detail.length).toBeGreaterThan(40);
      }
    }
  });
});
