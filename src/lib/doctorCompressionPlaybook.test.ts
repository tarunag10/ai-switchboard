import { describe, expect, it } from "vitest";

import {
  buildCompressionPlaybookSummary,
  COMPRESSION_PLAYBOOK_ORDER,
  compressionPlaybookShareText,
} from "./doctorCompressionPlaybook";
import type { DoctorIssue } from "./types";

const runtimeIssue: DoctorIssue = {
  id: "headroom_runtime_unreachable",
  title: "Headroom runtime is not reachable",
  body: "Proxy is down.",
  severity: "error",
  repairAction: "repair_runtime",
};

const rtkIssue: DoctorIssue = {
  id: "rtk_not_active",
  title: "RTK is not active",
  body: "RTK is off.",
  severity: "warning",
  repairAction: "repair_rtk_runtime",
};

const repoIssue: DoctorIssue = {
  id: "repo_intelligence_stale",
  title: "Repo Intelligence index is stale",
  body: "Re-index required.",
  severity: "warning",
  repairAction: "clear_repo_intelligence_index",
};

describe("doctorCompressionPlaybook", () => {
  it("keeps playbook stage order stable", () => {
    expect(COMPRESSION_PLAYBOOK_ORDER).toEqual([
      "runtime",
      "routing",
      "rtk",
      "cache",
      "repo-index",
      "mcp",
    ]);
  });

  it("groups open issues into ordered stages without inventing repairs", () => {
    const summary = buildCompressionPlaybookSummary({
      issues: [repoIssue, runtimeIssue, rtkIssue],
    });

    expect(summary.hasOpenCompressionIssues).toBe(true);
    expect(summary.stages.map((entry) => entry.stage.id)).toEqual(
      COMPRESSION_PLAYBOOK_ORDER,
    );
    expect(summary.stages[0]?.openIssues[0]?.id).toBe(runtimeIssue.id);
    expect(summary.stages[2]?.openIssues[0]?.id).toBe(rtkIssue.id);
    expect(summary.stages[4]?.openIssues[0]?.id).toBe(repoIssue.id);
    expect(summary.stages[0]?.nextRepairLabel).toBe("Restart Headroom");
  });

  it("surfaces cache eligibility without a new repair action", () => {
    const summary = buildCompressionPlaybookSummary({
      issues: [],
      exactCacheRecommended: true,
      semanticCacheEnabled: false,
    });

    expect(summary.stages.find((entry) => entry.stage.id === "cache")?.openIssues)
      .toHaveLength(1);
    expect(
      summary.stages.find((entry) => entry.stage.id === "cache")?.nextRepairAction,
    ).toBeNull();
  });

  it("formats a shareable playbook summary", () => {
    const text = compressionPlaybookShareText(
      buildCompressionPlaybookSummary({ issues: [runtimeIssue, rtkIssue] }),
    );

    expect(text).toContain("Runtime (1)");
    expect(text).toContain("Next repair: Restart Headroom");
    expect(text).toContain("RTK (1)");
  });
});
