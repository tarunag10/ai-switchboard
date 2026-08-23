import { describe, expect, it } from "vitest";

import {
  canActivateChonkifyRepoPack,
  canActivateSwitchboardPackCompaction,
  evaluateSwitchboardPackCompactionGate,
  wrongOmissionRatePct,
} from "./chonkifyPromotionGate";

describe("Switchboard Pack Compaction promotion gate", () => {
  it("passes the shipped Switchboard-native provenance fixture", () => {
    const verdict = evaluateSwitchboardPackCompactionGate();
    expect(verdict.verdict).toBe("repo_pack_eligible");
    expect(canActivateSwitchboardPackCompaction()).toBe(true);
    expect(canActivateChonkifyRepoPack()).toBe(true);
  });

  it("blocks when wrong-omission fixtures exceed the gate", () => {
    const verdict = evaluateSwitchboardPackCompactionGate(
      {
        schemaVersion: 1,
        license: "MIT",
        implementationId: "switchboard-pack-compaction",
        implementationOwner: "ai-switchboard",
        upstreamCodeEmbedded: false,
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

  it("blocks evidence that claims upstream Chonkify as the native implementation", () => {
    const verdict = evaluateSwitchboardPackCompactionGate({
      schemaVersion: 1,
      license: "MIT",
      implementationId: "switchboard-pack-compaction",
      implementationOwner: "ai-switchboard",
      upstreamCodeEmbedded: true,
      requiredSignals: ["license_verified"],
      wrongOmissionFixturesPath: "fixtures/chonkify-wrong-omission-fixtures.json",
      maxWrongOmissionRatePct: 0,
    });
    expect(verdict.verdict).toBe("blocked");
    expect(verdict.reasons).toContain(
      "upstream Chonkify code must not be claimed by the Switchboard-native compactor",
    );
  });
});
