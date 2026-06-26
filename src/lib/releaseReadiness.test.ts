import { describe, expect, it } from "vitest";

import {
  releaseReadinessCommand,
  releaseReadinessGroups,
  releaseReadinessItemCount,
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
});
