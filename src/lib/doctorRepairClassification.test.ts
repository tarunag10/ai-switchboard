import { describe, expect, it } from "vitest";

import {
  doctorIssueActionHint,
  doctorIssueActionKind,
  doctorIssueActionLabel,
} from "./doctorRepairCopy";

describe("doctor repair classification", () => {
  it("describes automatic versus manual issue handling", () => {
    expect(doctorIssueActionKind("repair_runtime")).toBe("automatic");
    expect(doctorIssueActionKind(null)).toBe("manual");
    expect(doctorIssueActionKind(undefined)).toBe("manual");
    expect(doctorIssueActionLabel("repair_runtime")).toBe("Auto repair");
    expect(doctorIssueActionLabel(null)).toBe("Manual step");
    expect(doctorIssueActionHint(null)).toContain("No automatic repair");
  });
});
