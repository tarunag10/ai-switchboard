import { describe, expect, it } from "vitest";

import {
  doctorIssueActionHint,
  doctorIssueActionKind,
  doctorIssueActionLabel,
  doctorIssueActionLabelForIssue,
} from "./doctorRepairCopy";

describe("doctor repair classification", () => {
  it("describes automatic versus manual issue handling", () => {
    expect(doctorIssueActionKind("repair_runtime")).toBe("automatic");
    expect(doctorIssueActionKind("verify_off_mode")).toBe("verification");
    expect(doctorIssueActionKind(null)).toBe("manual");
    expect(doctorIssueActionKind(undefined)).toBe("manual");
    expect(doctorIssueActionLabel("repair_runtime")).toBe("Auto repair");
    expect(doctorIssueActionLabel("verify_off_mode")).toBe("Verification");
    expect(doctorIssueActionLabel(null)).toBe("Approval needed");
    expect(doctorIssueActionHint(null)).toContain("No automatic repair");
  });

  it("labels gated connector manual issues distinctly", () => {
    expect(
      doctorIssueActionLabelForIssue({
        id: "planned_connectors_detected",
        title: "Gated connector readiness detected",
        body: "Connector setup remains gated.",
        severity: "warning",
        repairAction: null,
      }),
    ).toBe("Gated setup");
  });
});
