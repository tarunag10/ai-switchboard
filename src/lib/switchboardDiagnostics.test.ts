import { describe, expect, it } from "vitest";

import { switchboardModeDiagnostic } from "./switchboardDiagnostics";

describe("switchboardModeDiagnostic", () => {
  it("explains requested and effective mode mismatches", () => {
    expect(switchboardModeDiagnostic("full", "rtk", true)).toEqual({
      requestedLabel: "Full optimization",
      effectiveLabel: "RTK only",
      attentionCopy:
        "Active now: RTK only. Connect a supported client or repair Headroom routing in Doctor.",
    });
  });

  it("does not show attention copy when backend says mode is healthy", () => {
    expect(switchboardModeDiagnostic("full", "full", false)).toEqual({
      requestedLabel: "Full optimization",
      effectiveLabel: "Full optimization",
      attentionCopy: "",
    });
  });

  it("falls back to requested mode when effective mode is absent", () => {
    expect(switchboardModeDiagnostic("rtk", undefined, true)).toEqual({
      requestedLabel: "RTK only",
      effectiveLabel: "RTK only",
      attentionCopy: "Active now: RTK only. Run Doctor to repair.",
    });
  });
});
