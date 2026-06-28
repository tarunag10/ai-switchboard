import { describe, expect, it } from "vitest";

import {
  buildManagedChangeTimelineEvents,
  canRepairIssue,
  doctorIssueGuidance,
  plannedConnectorDoctorGuidance,
  doctorRepairHint,
  doctorRepairLabel,
  doctorTimelineKindLabel,
  formatDoctorTimelineShareText,
  formatDoctorReportShareText,
  sortDoctorTimelineEvents,
} from "./doctorRepairCopy";
import { managedChangeRecords } from "./managedChanges";

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

  it("guides corrupt Repo Intelligence storage recovery", () => {
    expect(
      doctorIssueGuidance({
        id: "repo_intelligence_storage_corrupt",
        title: "Repo Intelligence index cannot be read",
        body: "The saved Repo Intelligence index could not be parsed.",
        severity: "warning",
        repairAction: null,
      }),
    ).toContain("Clear the unreadable Repo Intelligence index");
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
    expect(text).toContain("next automation gate is backup implemented");
    expect(text).toContain("Automation gated");
  });

  it("explains why planned connectors stay manual in Doctor", () => {
    const guidance = plannedConnectorDoctorGuidance();

    expect(guidance).toContain("detection evidence");
    expect(guidance).toContain("readiness stages");
    expect(guidance).toContain("safety badges");
    expect(guidance).toContain("next automation gate is backup implemented");
    expect(guidance).toContain("Manual only");
    expect(guidance).toContain("Automation gated");
    expect(guidance).toContain("Unsupported account/model");
    expect(guidance).toContain("backup, verify, rollback, and Off mode cleanup");
  });

  it("labels and sorts Doctor timeline events newest first", () => {
    const events = sortDoctorTimelineEvents([
      {
        id: "older",
        kind: "backup",
        title: "Created Codex backup",
        body: "Backed up managed config before repair.",
        occurredAt: "2026-06-27T09:00:00.000Z",
        status: "ok",
        actor: "doctor",
        target: "~/.codex/config.toml",
      },
      {
        id: "newer",
        kind: "failed_repair",
        title: "RTK repair failed",
        body: "RTK binary was unavailable.",
        occurredAt: "2026-06-27T10:00:00.000Z",
        status: "error",
        actor: "doctor",
        target: "rtk",
      },
    ]);

    expect(doctorTimelineKindLabel("failed_repair")).toBe("Failed repair");
    expect(doctorTimelineKindLabel("index_refresh")).toBe("Index refresh");
    expect(events.map((event) => event.id)).toEqual(["newer", "older"]);
  });

  it("formats Doctor timeline events for support sharing", () => {
    const text = formatDoctorTimelineShareText([
      {
        id: "repair-1",
        kind: "repair",
        title: "Repaired RTK integration",
        body: "Restored shell hook wiring.",
        occurredAt: "2026-06-27T10:00:00.000Z",
        status: "ok",
        actor: "doctor",
        target: "~/.zshrc",
      },
      {
        id: "rollback-1",
        kind: "rollback",
        title: "Rolled back Codex routing",
        body: "Removed managed provider block.",
        occurredAt: "2026-06-27T10:05:00.000Z",
        status: "warning",
        actor: "user",
        target: "~/.codex/config.toml",
      },
    ]);

    expect(text).toContain("Mac AI Switchboard Doctor timeline");
    expect(text).toContain("Events: 2");
    expect(text).toContain("1. Rolled back Codex routing");
    expect(text).toContain("Kind: Rollback");
    expect(text).toContain("Actor: user");
    expect(text).toContain("Target: ~/.codex/config.toml");
  });

  it("builds scrubbed timeline events from managed rollback records", () => {
    const events = buildManagedChangeTimelineEvents(
      managedChangeRecords,
      "2026-06-27T10:00:00.000Z",
    );
    const codex = events.find(
      (event) => event.id === "managed-change-codex-routing",
    );

    expect(events).toHaveLength(managedChangeRecords.length);
    expect(codex).toMatchObject({
      kind: "backup",
      title: "Codex routing rollback coverage",
      status: "warning",
      actor: "switchboard",
      target: "3 managed paths",
    });
    expect(codex?.body).toContain("headroom:codex_cli");
    expect(codex?.body).toContain("Backup: next to edited client config");
    expect(codex?.target).not.toContain("~/.codex/config.toml");

    const repoIndex = events.find(
      (event) => event.id === "managed-change-repo-intelligence",
    );
    expect(repoIndex).toMatchObject({
      kind: "rollback",
      target: "1 managed path",
    });
  });

  it("formats an empty Doctor timeline", () => {
    expect(formatDoctorTimelineShareText([])).toContain(
      "No Doctor timeline events recorded.",
    );
  });
});
