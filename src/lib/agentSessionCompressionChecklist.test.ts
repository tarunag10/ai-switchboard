import { describe, expect, it } from "vitest";

import {
  agentSessionChecklistStatusLabel,
  buildAgentSessionCompressionChecklist,
} from "./agentSessionCompressionChecklist";
import {
  buildRepoIntelligenceSummary,
  getRepoIndexFreshness,
} from "./repoIntelligence";

describe("agentSessionCompressionChecklist", () => {
  it("blocks copy guidance when the index is missing", () => {
    const checklist = buildAgentSessionCompressionChecklist({
      agentId: "codex",
      packEstimatedTokens: 2_000,
      tokenBudget: 24_000,
      switchboardMode: "full",
      indexFreshness: getRepoIndexFreshness({}),
    });

    expect(checklist.blocked).toBe(true);
    expect(checklist.items[0]?.doctorLink).toBe(true);
    expect(agentSessionChecklistStatusLabel(checklist.items[0]!.status)).toBe(
      "Blocked",
    );
  });

  it("warns when the selected pack exceeds the budget", () => {
    const summary = buildRepoIntelligenceSummary([
      { path: "src/App.tsx", bytes: 4000 },
    ]);
    const checklist = buildAgentSessionCompressionChecklist({
      agentId: "cursor",
      packEstimatedTokens: 12_000,
      tokenBudget: 8_000,
      switchboardMode: "full",
      indexFreshness: getRepoIndexFreshness({
        indexedAt: summary.indexedAt,
        indexerVersion: summary.indexerVersion,
        indexMetadata: summary.indexMetadata,
        graph: summary.graph,
      }),
    });

    expect(checklist.blocked).toBe(true);
    expect(checklist.canCopyWithAcknowledgment).toBe(true);
  });
});
