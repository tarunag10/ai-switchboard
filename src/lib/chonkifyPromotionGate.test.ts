import { describe, expect, it } from "vitest";

import {
  canActivateChonkifyRepoPack,
  evaluateChonkifyPromotionGate,
  wrongOmissionRatePct,
} from "./chonkifyPromotionGate";

describe("chonkifyPromotionGate", () => {
  it("passes the shipped MIT provenance fixture", () => {
    const verdict = evaluateChonkifyPromotionGate();
    expect(verdict.verdict).toBe("repo_pack_eligible");
    expect(canActivateChonkifyRepoPack()).toBe(true);
  });

  it("blocks when wrong-omission fixtures exceed the gate", () => {
    const verdict = evaluateChonkifyPromotionGate(
      {
        schemaVersion: 1,
        license: "MIT",
        requiredSignals: ["license_verified"],
        wrongOmissionFixturesPath: "fixtures/chonkify-wrong-omission-fixtures.json",
        maxWrongOmissionRatePct: 0,
      },
      {
        fixtures: [
          {
            name: "bad",
            relevantFacts: ["fact-a", "fact-b"],
            wrongOmissions: ["fact-a"],
          },
        ],
      },
    );
    expect(verdict.verdict).toBe("blocked");
    expect(
      wrongOmissionRatePct({
        name: "bad",
        relevantFacts: ["fact-a", "fact-b"],
        wrongOmissions: ["fact-a"],
      }),
    ).toBe(50);
  });
});
