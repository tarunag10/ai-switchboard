import { describe, expect, it } from "vitest";

import {
  buildManagedChangeTimelineEvents,
  canRepairIssue,
  doctorIssueGuidance,
  formatPlannedConnectorDoctorDossiers,
  buildDoctorReportTimelineEvents,
  plannedConnectorDoctorGuidance,
  doctorRepairHint,
  doctorRepairLabel,
  doctorTimelineKindLabel,
  formatDoctorTimelineShareText,
  formatDoctorReportShareText,
  plannedConnectorDoctorPreviewRows,
  formatVerifyOffModeShareText,
  repoIntelligenceDoctorAvailabilityGates,
  repoIntelligenceDoctorApiContract,
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

  it("formats a focused Verify Off report when routing evidence remains", () => {
    const text = formatVerifyOffModeShareText({
      status: "warning",
      summary: "Off mode requested, but routing is still visible.",
      issues: [
        {
          id: "off_mode_not_clean",
          title: "Off mode still has active routing evidence",
          body: "Headroom engine is still reachable and 2 clients are enabled.",
          severity: "warning",
          repairAction: "verify_off_mode",
        },
      ],
    });

    expect(text).toContain("Mac AI Switchboard Verify Off report");
    expect(text).toContain("Status: active routing evidence found");
    expect(text).toContain("Checks: active engine, enabled clients, RTK routing evidence");
    expect(text).toContain("Headroom engine is still reachable");
    expect(text).toContain("Doctor will re-check active engine");
  });

  it("formats a clean Verify Off report from Doctor state", () => {
    const text = formatVerifyOffModeShareText({
      status: "ok",
      summary: "Switchboard looks ready.",
      issues: [],
    });

    expect(text).toContain("Status: clean");
    expect(text).toContain(
      "Evidence: no Off mode routing issue is present in the current Doctor report.",
    );
    expect(text).toContain("Stay in Off mode");
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

  it("guides moved Repo Intelligence path recovery", () => {
    expect(
      doctorIssueGuidance({
        id: "repo_intelligence_repo_moved",
        title: "Repo Intelligence index no longer matches this folder",
        body: "The saved Repo Intelligence file map no longer matches files under the saved path.",
        severity: "warning",
        repairAction: "clear_repo_intelligence_index",
      }),
    ).toContain("Re-index the current local repo path");
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
    expect(text).toContain("Planned connector config readiness dossiers");
    expect(text).toContain("## Gemini CLI");
    expect(text).toContain("Config surface:");
    expect(text).toContain("Blocked automation gates:");
    expect(text).toContain(
      "Backup Implemented: No planned connector can write config until exact backup coverage exists.",
    );
    expect(text).toContain("Gated config-creation steps:");
    expect(text).toContain("Show dry-run diff");
    expect(text).toContain("Required evidence:");
    expect(text).toContain("Repo Intelligence local API contract");
    expect(text).toContain("get_repo_manifest");
    expect(text).toContain("get_repo_pack");
    expect(text).toContain("get_agent_handoff");
    expect(text).toContain("planned connector config readiness");
    expect(text).toContain("next gate");
    expect(text).toContain("evidence requirements");
    expect(text).toContain("config path strategy");
    expect(text).toContain("account caveat");
    expect(text).toContain("rollback strategy");
    expect(text).toContain("get_index_freshness");
    expect(text).toContain("API availability");
    expect(text).toContain("graph availability");
    expect(text).toContain("indexer/parser versions");
    expect(text).toContain("indexed/skipped counts");
    expect(text).toContain("missing/stale index state");
    expect(text).toContain("clear_repo_index");
    expect(text).toContain("Availability gates");
    expect(text).toContain("never mutates the user repo");
  });

  it("summarizes the read-only Repo Intelligence API contract for Doctor", () => {
    const contract = repoIntelligenceDoctorApiContract();

    expect(contract).toContain("read-only by default");
    expect(contract).toContain("secret-like paths excluded");
    expect(contract).toContain("generated/vendor paths skipped");
    expect(contract).toContain("outputs bounded by pack/token budgets");
    expect(contract).toContain("parser version reported");
    expect(contract).toContain("graph availability reported");
    expect(contract).toContain("indexed/skipped counts");
    expect(contract).toContain("missing, stale, corrupt, or moved repo indexes");
    expect(contract).toContain("planned connector config readiness");
    expect(contract).toContain("config path strategy");
    expect(contract).toContain("rollback strategy");
  });

  it("summarizes Repo Intelligence Doctor availability gates", () => {
    const gates = repoIntelligenceDoctorAvailabilityGates();

    expect(gates).toContain("get_index_freshness is the trust gate");
    expect(gates).toContain("copy actions stay blocked");
    expect(gates).toContain("stale state visible");
    expect(gates).toContain(
      "clear_repo_index removes only Switchboard managed index metadata",
    );
    expect(gates).toContain("Moved repo path");
    expect(gates).toContain("indexer/parser versions");
    expect(gates).toContain("indexed/skipped counts");
    expect(gates).toContain("secret exclusion");
    expect(gates).toContain("read-only safety");
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

  it("formats per-tool planned connector readiness dossiers", () => {
    const dossiers = formatPlannedConnectorDoctorDossiers();

    expect(dossiers).toContain("Planned connector config readiness dossiers");
    expect(dossiers).toContain("## OpenCode");
    expect(dossiers).toContain("Connector ID: opencode");
    expect(dossiers).toContain("Next blocked gate: Backup Implemented");
    expect(dossiers).toContain("Automation enabled: no");
    expect(dossiers).toContain("Blocked automation gates:");
    expect(dossiers).toContain(
      "Apply Implemented: Automatic setup is disabled until a reversible apply path exists.",
    );
    expect(dossiers).toContain(
      "Off Cleanup Implemented: Off mode cleanup must remove managed routing before automation is enabled.",
    );
    expect(dossiers).toContain("Detect config surface");
    expect(dossiers).toContain("Clean up in Off mode");
    expect(dossiers).toContain("Post-rollback diff proving unrelated user settings are unchanged.");
    expect(dossiers).toContain("## Amazon Q Developer CLI");
    expect(dossiers).toContain("AWS profile");
  });

  it("builds compact planned connector preview rows for Doctor", () => {
    const rows = plannedConnectorDoctorPreviewRows();

    expect(rows).toHaveLength(11);
    expect(rows[0]).toEqual(
      expect.objectContaining({
        id: "gemini_cli",
        name: "Gemini CLI",
        setupPhase: "Guide",
        nextBlockedGate: "Backup Implemented",
        automationEnabled: false,
      }),
    );
    expect(rows.map((row) => row.name)).toContain("OpenCode");
    expect(rows.every((row) => row.configSurface.length > 20)).toBe(true);
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
        body: "Removed managed provider block from /Users/tarunagarwal/.codex/config.toml.",
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
    expect(text).toContain("Target: [home-path]");
    expect(text).toContain("Body: Removed managed provider block from [user-path]");
    expect(text).toContain("Repo Intelligence Doctor availability gates");
    expect(text).toContain("get_index_freshness is the trust gate");
    expect(text).toContain("Missing index");
    expect(text).toContain("Stale index");
    expect(text).toContain("Corrupt index");
    expect(text).toContain("Moved repo path");
    expect(text).toContain(
      "clear_repo_index removes only Switchboard managed index metadata",
    );
    expect(text).toContain("API availability");
    expect(text).toContain("read-only safety");
    expect(text).not.toContain("~/.codex/config.toml");
    expect(text).not.toContain("/Users/tarunagarwal");
  });

  it("scrubs secrets from Doctor timeline support sharing", () => {
    const text = formatDoctorTimelineShareText([
      {
        id: "secret-issue",
        kind: "failed_repair",
        title: "Provider config failed",
        body:
          "OPENAI_API_KEY=sk-proj_abc123456789012345 leaked near github_pat_1234567890abcdef and /Users/tarunagarwal/.config/tool.",
        occurredAt: "2026-06-27T10:10:00.000Z",
        status: "error",
        actor: "doctor",
        target: "xai_1234567890abcdef",
      },
    ]);

    expect(text).toContain("OPENAI_API_KEY=[secret]");
    expect(text).toContain("Target: [secret]");
    expect(text).toContain("[user-path]");
    expect(text).not.toContain("sk-proj_abc123456789012345");
    expect(text).not.toContain("github_pat_1234567890abcdef");
    expect(text).not.toContain("xai_1234567890abcdef");
    expect(text).not.toContain("/Users/tarunagarwal");
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
    expect(codex?.body).toContain("Dry-run diff available");
    expect(codex?.body).toContain("apply requires explicit confirmation");
    expect(codex?.body).toContain(
      "Apply gate: target, backup path, marker, rollback plan, and Off-mode cleanup boundary must be confirmed first.",
    );
    expect(codex?.body).toContain("Per-change rollback: available");
    expect(codex?.target).not.toContain("~/.codex/config.toml");

    const repoIndex = events.find(
      (event) => event.id === "managed-change-repo-intelligence",
    );
    expect(repoIndex).toMatchObject({
      kind: "rollback",
      target: "1 managed path",
    });
    expect(repoIndex?.body).toContain(
      "No config diff is required for this managed footprint.",
    );
    expect(repoIndex?.body).toContain(
      "Apply gate: not applicable because this footprint is removed through cleanup inventory.",
    );
  });

  it("builds issue-level Doctor timeline events", () => {
    const events = buildDoctorReportTimelineEvents(
      {
        status: "error",
        summary: "Doctor found a blocking issue.",
        issues: [
          {
            id: "codex_setup_broken",
            title: "Codex setup broken",
            body: "Provider block is missing.",
            severity: "error",
            repairAction: "repair_codex_setup",
          },
          {
            id: "planned_connectors_detected",
            title: "Planned coding tools detected",
            body: "Automation is still gated.",
            severity: "warning",
            repairAction: null,
          },
          {
            id: "repo_intelligence_stale",
            title: "Repo Intelligence stale",
            body: "Saved index is stale.",
            severity: "warning",
            repairAction: "clear_repo_intelligence_index",
          },
        ],
      },
      "Repair finished.",
      "2026-06-27T10:00:00.000Z",
    );

    expect(events.map((event) => event.id).sort()).toEqual([
      "doctor-issue-codex_setup_broken",
      "doctor-issue-planned_connectors_detected",
      "doctor-issue-repo_intelligence_stale",
      "latest-repair-success",
      "latest-report",
    ]);
    expect(
      events.find((event) => event.id === "doctor-issue-codex_setup_broken"),
    ).toMatchObject({
      kind: "failed_repair",
      status: "error",
      target: "Repair Codex",
    });
    expect(
      events.find(
        (event) => event.id === "doctor-issue-planned_connectors_detected",
      ),
    ).toMatchObject({
      kind: "connector_setup",
      target: "manual follow-up",
    });
    expect(
      events.find((event) => event.id === "doctor-issue-repo_intelligence_stale"),
    ).toMatchObject({
      kind: "index_refresh",
      target: "Clear index",
    });
    expect(events.find((event) => event.id === "latest-repair-success")).toMatchObject({
      kind: "repair",
      status: "ok",
      target: "automatic repair",
    });
  });

  it("formats an empty Doctor timeline", () => {
    const text = formatDoctorTimelineShareText([]);

    expect(text).toContain("No Doctor timeline events recorded.");
    expect(text).toContain("Repo Intelligence Doctor availability gates");
    expect(text).toContain("get_index_freshness is the trust gate");
  });
});
