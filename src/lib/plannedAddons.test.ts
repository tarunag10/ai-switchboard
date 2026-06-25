import { describe, expect, it } from "vitest";

import { getPlannedAddon, plannedAddons } from "./plannedAddons";

describe("planned add-ons", () => {
  it("tracks repo intelligence as a planned local-first graph capability", () => {
    const repoIntelligence = getPlannedAddon("repo_intelligence");

    expect(repoIntelligence).toMatchObject({
      name: "Repo Intelligence",
      statusLabel: "Planned",
    });
    expect(repoIntelligence?.description).toContain("Future local repo graph");
    expect(repoIntelligence?.description).toContain("smaller, safer edits");
    expect(repoIntelligence?.bullets.join(" ")).toContain("Graphy-style");
    expect(repoIntelligence?.bullets.join(" ")).toContain("Local-first");
  });

  it("keeps planned add-on ids stable for UI rendering", () => {
    expect(plannedAddons.map((addon) => addon.id)).toEqual([
      "repo_intelligence",
    ]);
  });
});
