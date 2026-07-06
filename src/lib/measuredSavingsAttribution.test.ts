import { describe, expect, it, vi } from "vitest";

import {
  buildMeasuredAddonSavingsRequest,
  recordMeasuredAddonSavings,
} from "./measuredSavingsAttribution";

describe("measured savings attribution", () => {
  it("builds a measured add-on request with stable labels", () => {
    expect(
      buildMeasuredAddonSavingsRequest({
        source: "markitdown",
        baselineTokens: 3_200.9,
        optimizedTokens: 900.2,
        requestDelta: 3,
        detail: "Converted PDF to Markdown before attaching it.",
      }),
    ).toEqual({
      source: "markitdown",
      label: "MarkItDown",
      baselineTokens: 3_200,
      optimizedTokens: 900,
      requestDelta: 3,
      detail: "Converted PDF to Markdown before attaching it.",
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
    ).toMatchObject({
      source: "compact_chinese",
      label: "Compact Chinese private scratch profile",
      requestDelta: 1,
    });
  });

  it("does not build requests for empty or invalid deltas", () => {
    expect(
      buildMeasuredAddonSavingsRequest({
        source: "caveman",
        baselineTokens: 100,
        optimizedTokens: 100,
      }),
    ).toBeNull();
    expect(
      buildMeasuredAddonSavingsRequest({
        source: "ponytail",
        baselineTokens: 100,
        optimizedTokens: 50,
        requestDelta: 0,
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
        detail: "Measured narrower changed-file handoff.",
      },
      invoke,
    );

    expect(result).toEqual({
      recorded: true,
      tokensSaved: 880,
      requestDelta: 2,
    });
    expect(invoke).toHaveBeenCalledWith("record_measured_savings_attribution", {
      request: {
        source: "ponytail",
        label: "Ponytail",
        baselineTokens: 1_400,
        optimizedTokens: 520,
        requestDelta: 2,
        detail: "Measured narrower changed-file handoff.",
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
        },
        invoke,
      ),
    ).resolves.toEqual({
      recorded: false,
      tokensSaved: 0,
      requestDelta: 1,
      reason: "empty_delta",
    });

    await expect(
      recordMeasuredAddonSavings(
        {
          source: "caveman",
          baselineTokens: 120,
          optimizedTokens: 80,
          requestDelta: 0,
        },
        invoke,
      ),
    ).resolves.toEqual({
      recorded: false,
      tokensSaved: 0,
      requestDelta: 0,
      reason: "invalid_request_delta",
    });

    expect(invoke).not.toHaveBeenCalled();
  });
});
