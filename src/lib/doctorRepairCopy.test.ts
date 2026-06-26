import { describe, expect, it } from "vitest";

import {
  canRepairIssue,
  doctorIssueGuidance,
  doctorRepairHint,
  doctorRepairLabel,
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
  });

  it("detects repairable issues by action presence", () => {
    expect(canRepairIssue("repair_runtime")).toBe(true);
    expect(canRepairIssue("clear_repo_intelligence_index")).toBe(true);
    expect(canRepairIssue("")).toBe(false);
    expect(canRepairIssue(null)).toBe(false);
    expect(canRepairIssue(undefined)).toBe(false);
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
});
