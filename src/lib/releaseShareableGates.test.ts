import { describe, expect, it } from "vitest";

import { releaseShareableGates } from "./releaseReadiness";

describe("shareable DMG gates", () => {
  it("summarizes the gates shown in Settings", () => {
    expect(releaseShareableGates.map((gate) => gate.id)).toEqual([
      "environment-clear",
      "backend-validation",
      "signed-notarized",
      "installed-smoke",
    ]);

    const gateCopy = releaseShareableGates
      .map((gate) => `${gate.label} ${gate.detail}`)
      .join(" ");

    expect(gateCopy).toContain("release:report");
    expect(gateCopy).toContain("cargo");
    expect(gateCopy).toContain("notarization");
    expect(gateCopy).toContain("/Applications/Mac AI Switchboard.app");
  });
});
