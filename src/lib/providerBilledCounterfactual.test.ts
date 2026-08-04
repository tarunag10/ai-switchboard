import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  recordProviderBilledCounterfactual,
  validateProviderBilledCounterfactual,
} from "./providerBilledCounterfactual";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

describe("validateProviderBilledCounterfactual", () => {
  it("accepts a complete provider-billed pair", () => {
    expect(
      validateProviderBilledCounterfactual({
        provider: "codex",
        baselineTokens: 12_000,
        optimizedTokens: 4_200,
        baselineEvidence: "Codex /wham/usage before",
        optimizedEvidence: "Codex /wham/usage after",
      }).valid,
    ).toBe(true);
  });

  it("rejects pairs without independent evidence", () => {
    expect(
      validateProviderBilledCounterfactual({
        provider: "claude",
        baselineTokens: 1_000,
        optimizedTokens: 800,
        baselineEvidence: "",
        optimizedEvidence: "after",
      }).reason,
    ).toBe("missing_baseline_evidence");
  });
});

describe("recordProviderBilledCounterfactual", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  it("records measured savings when validation passes", async () => {
    const result = await recordProviderBilledCounterfactual({
      provider: "headroom_stats",
      baselineTokens: 3_000,
      optimizedTokens: 1_800,
      baselineEvidence: "Headroom /stats before",
      optimizedEvidence: "Headroom /stats after",
      requestDelta: 2,
    });

    expect(result.recorded).toBe(true);
    expect(result.tokensSaved).toBe(1_200);
    expect(invoke).toHaveBeenCalledWith("record_provider_billed_counterfactual", {
      request: expect.objectContaining({
        baselineTokens: 3_000,
        optimizedTokens: 1_800,
        requestDelta: 2,
      }),
    });
  });
});
