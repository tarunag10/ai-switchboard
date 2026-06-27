import { describe, expect, it } from "vitest";

import {
  canRepairIssue,
  doctorIssueGuidance,
  doctorRepairHint,
  doctorRepairLabel,
  formatDoctorReportShareText,
} from "./doctorRepairCopy";

describe("doctor repair copy", () => {
  it.each([
    ["repair_runtime", "Restart Headroom"],
    ["reset_codex_bypass", "Reset Codex"],
    ["repair_codex_setup", "Repair Codex"],
    ["repair_client_setups", "Repair clients"],
    ["repair_rtk_integrations", "Repair RTK"],
    ["repair_rtk_runtime", "Install RTK"],
    ["clear_repo_intelligence_index", "Clear index"],
    ["verify_off_mode", "Verify Off"],
    ["unknown", "Repair"],
  ])("labels %s", (action, label) => {
    expect(doctorRepairLabel(action)).toBe(label);
  });

  it("uses Codex-specific hints for Codex repair actions", () => {
    expect(doctorRepairHint("reset_codex_bypass")).toContain(
      "Compact the Codex conversation",
    );
    expect(doctorRepairHint("repair_codex_setup")).toContain(
      "Codex-supported ChatGPT model",
    );
  });

  it("describes runtime RTK and Repo Intelligence repair actions", () => {
    expect(doctorRepairHint("repair_runtime")).toContain(
      "refreshes switchboard status",
    );
    expect(doctorRepairHint("repair_rtk_integrations")).toContain(
      "RTK PATH and hook",
    );
    expect(doctorRepairHint("repair_rtk_runtime")).toContain(
      "local shell-output compression",
    );
    expect(doctorRepairHint("clear_repo_intelligence_index")).toContain(
      "saved Repo Intelligence summary",
    );
    expect(doctorRepairHint("verify_off_mode")).toContain(
      "without changing local routing",
    );
  });

  it("detects repairable issues by action presence", () => {
    expect(canRepairIssue("repair_runtime")).toBe(true);
    expect(canRepairIssue("clear_repo_intelligence_index")).toBe(true);
    expect(canRepairIssue("verify_off_mode")).toBe(true);
    expect(canRepairIssue("")).toBe(false);
    expect(canRepairIssue(null)).toBe(false);
    expect(canRepairIssue(undefined)).toBe(false);
  });

  it("guides Off mode verification without promising repair", () => {
    expect(
      doctorIssueGuidance({
        id: "off_mode_not_clean",
        title: "Off mode still has active routing evidence",
        body: "Off mode requested, but Headroom engine is still reachable.",
        severity: "warning",
        repairAction: "verify_off_mode",
      }),
    ).toContain("Doctor will re-check active engine");
  });

  it("guides manual degraded mode issues without repair action", () => {
    expect(
      doctorIssueGuidance({
        id: "switchboard_mode_degraded",
        title: "Requested optimization is degraded",
        body: "Full optimization is requested, but RTK only is active.",
        severity: "warning",
        repairAction: null,
      }),
    ).toContain("Requested mode and active mode differ");
    expect(
      doctorIssueGuidance({
        id: "switchboard_mode_degraded",
        title: "Requested optimization is degraded",
        body: "Full optimization is requested, but RTK only is active.",
        severity: "warning",
        repairAction: null,
      }),
    ).toContain("re-run Doctor until requested mode becomes active");
  });

  it("formats healthy Doctor report for sharing", () => {
    expect(
      formatDoctorReportShareText({
        status: "ok",
        summary: "No issues.",
        issues: [],
      }),
    ).toContain("No Doctor issues found.");
  });

  it("formats mixed automatic and manual Doctor report for sharing", () => {
    const text = formatDoctorReportShareText({
      status: "warning",
      summary: "Mixed setup required.",
      issues: [
        {
          id: "rtk_not_active",
          title: "RTK is not active",
          body: "Repair will install RTK.",
          severity: "warning",
          repairAction: "repair_rtk_runtime",
        },
        {
          id: "planned_connectors_detected",
          title: "Planned coding tools detected",
          body: "Gemini CLI detected.",
          severity: "warning",
          repairAction: null,
        },
      ],
    });

    expect(text).toContain("Mac AI Switchboard Doctor report");
    expect(text).toContain("Status: warning");
    expect(text).toContain("Action: automatic / Install RTK");
    expect(text).toContain("Action: manual / Manual step");
    expect(text).toContain("Use RTK-only mode or Repo Intelligence packs");
  });
});
