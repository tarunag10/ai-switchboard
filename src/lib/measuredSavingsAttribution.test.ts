import { describe, expect, it, vi } from "vitest";

import {
  buildMeasuredAddonSavingsRequest,
  recordMeasuredAddonSavings,
  validateAddonMeasurement,
} from "./measuredSavingsAttribution";

const evidence = {
  baseline: "Captured the unoptimized handoff token count from the local client.",
  optimized: "Captured the optimized handoff token count from the same local client.",
};

describe("measured savings attribution", () => {
  it("builds a measured add-on request with stable labels", () => {
    expect(
      buildMeasuredAddonSavingsRequest({
        source: "markitdown",
        baselineTokens: 3_200.9,
        optimizedTokens: 900.2,
        requestDelta: 3,
        measurementEvidence: evidence,
        detail: "Converted PDF to Markdown before attaching it.",
      }),
    ).toEqual({
      source: "markitdown",
      label: "MarkItDown",
      baselineTokens: 3_200,
      optimizedTokens: 900,
      requestDelta: 3,
      detail:
        "Converted PDF to Markdown before attaching it. Baseline evidence: Captured the unoptimized handoff token count from the local client.. Optimized evidence: Captured the optimized handoff token count from the same local client..",
    });
  });

  it("uses explicit labels for scoped Caveman profiles", () => {
    expect(
      buildMeasuredAddonSavingsRequest({
        source: "compact_chinese",
        label: "Compact Chinese private scratch profile",
        baselineTokens: 900,
        optimizedTokens: 420,
      }),
    ).toBeNull();
  });

  it("refuses claims without independent before and after evidence", () => {
    expect(
      validateAddonMeasurement({
        source: "caveman",
        baselineTokens: 300,
        optimizedTokens: 120,
      }),
    ).toEqual({
      valid: false,
      confidence: "estimated",
      reason: "missing_baseline_evidence",
    });
    expect(
      validateAddonMeasurement({
        source: "ponytail",
        baselineTokens: 300,
        optimizedTokens: 120,
        measurementEvidence: { baseline: "local counter" },
      }),
    ).toEqual({
      valid: false,
      confidence: "estimated",
      reason: "missing_optimized_evidence",
    });
  });

  it("refuses invalid counters and keeps their confidence estimated", () => {
    expect(
      validateAddonMeasurement({
        source: "markitdown",
        baselineTokens: Number.NaN,
        optimizedTokens: 120,
        measurementEvidence: evidence,
      }),
    ).toMatchObject({
      valid: false,
      confidence: "estimated",
      reason: "invalid_baseline_tokens",
    });
    expect(
      validateAddonMeasurement({
        source: "markitdown",
        baselineTokens: -1,
        optimizedTokens: 120,
        measurementEvidence: evidence,
      }),
    ).toMatchObject({
      valid: false,
      confidence: "estimated",
      reason: "invalid_baseline_tokens",
    });
    expect(
      validateAddonMeasurement({
        source: "compact_chinese",
        baselineTokens: 300,
        optimizedTokens: 120,
        measurementEvidence: evidence,
      }),
    ).toMatchObject({
      valid: false,
      confidence: "estimated",
      reason: "unsupported_source",
    });
  });

  it("does not build requests for empty or invalid deltas", () => {
    expect(
      buildMeasuredAddonSavingsRequest({
        source: "caveman",
        baselineTokens: 100,
        optimizedTokens: 100,
        measurementEvidence: evidence,
      }),
    ).toBeNull();
    expect(
      buildMeasuredAddonSavingsRequest({
        source: "ponytail",
        baselineTokens: 100,
        optimizedTokens: 50,
        requestDelta: 0,
        measurementEvidence: evidence,
      }),
    ).toBeNull();
  });

  it("records measured add-on savings through the Tauri command", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);

    const result = await recordMeasuredAddonSavings(
      {
        source: "ponytail",
        baselineTokens: 1_400,
        optimizedTokens: 520,
        requestDelta: 2,
        measurementEvidence: evidence,
        detail: "Measured narrower changed-file handoff.",
      },
      invoke,
    );

    expect(result).toEqual({
      recorded: true,
      tokensSaved: 880,
      requestDelta: 2,
      confidence: "measured",
    });
    expect(invoke).toHaveBeenCalledWith("record_measured_savings_attribution", {
      request: {
        source: "ponytail",
        label: "Ponytail",
        baselineTokens: 1_400,
        optimizedTokens: 520,
        requestDelta: 2,
        detail:
          "Measured narrower changed-file handoff. Baseline evidence: Captured the unoptimized handoff token count from the local client.. Optimized evidence: Captured the optimized handoff token count from the same local client..",
      },
    });
  });

  it("skips non-positive runtime/session deltas without invoking Tauri", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);

    await expect(
      recordMeasuredAddonSavings(
        {
          source: "caveman",
          baselineTokens: 80,
          optimizedTokens: 120,
          measurementEvidence: evidence,
        },
        invoke,
      ),
    ).resolves.toEqual({
      recorded: false,
      tokensSaved: 0,
      requestDelta: 1,
      confidence: "estimated",
      reason: "empty_delta",
    });

    await expect(
      recordMeasuredAddonSavings(
        {
          source: "caveman",
          baselineTokens: 120,
          optimizedTokens: 80,
          requestDelta: 0,
          measurementEvidence: evidence,
        },
        invoke,
      ),
    ).resolves.toEqual({
      recorded: false,
      tokensSaved: 0,
      requestDelta: 0,
      confidence: "estimated",
      reason: "invalid_request_delta",
    });

    expect(invoke).not.toHaveBeenCalled();
  });
});
