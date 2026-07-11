import { buildSafeMemorySummary, canApplyAgentMemoryCompaction, normalizeAgentMemoryPreview, normalizeAgentMemoryReceipt, normalizeAgentMemorySnapshot } from "./agentMemory";
import { describe, expect, it } from "vitest";

describe("agent memory contracts", () => {
  it("normalizes snake-case backend snapshots", () => {
    const snapshot = normalizeAgentMemorySnapshot({ repo_path: "/repo", memory_sources: [{ source_path: "/repo/AGENTS.md", agent: "codex", scope: "repo", status: "live", estimated_tokens: 120, duplicate_tokens: 20, cacheable_tokens: 90, secret_scan: { status: "clear" } }] });
    expect(snapshot.sources[0]).toMatchObject({ sourcePath: "/repo/AGENTS.md", estimatedTokens: 120, secretScan: { status: "safe" } });
  });

  it("does not place paths or memory contents in a safe summary", () => {
    const snapshot = normalizeAgentMemorySnapshot({ sources: [{ sourcePath: "/private/AGENTS.md", agent: "codex", scope: "repo", status: "live", estimatedTokens: 25, secretScan: { status: "safe" } }] });
    const summary = buildSafeMemorySummary(snapshot);
    expect(summary).toContain("codex");
    expect(summary).not.toContain("/private/AGENTS.md");
  });

  it("retains a blocked secret scan for the UI safety gate", () => {
    const preview = normalizeAgentMemoryPreview({ agent: "claude", diff: "do not show", secret_scan: { status: "blocked", reason: "credential" } });
    expect(preview.secretScan).toEqual(expect.objectContaining({ status: "blocked", reason: "credential" }));
  });

  it("normalizes the content-free structural preview emitted by the backend", () => {
    const preview = normalizeAgentMemoryPreview({ agent: "codex", blockedBySecrets: false, sources: [{ sourcePath: "/repo/AGENTS.md", beforeTokens: 90, afterTokens: 50, estimatedTokensSaved: 40, diffSummary: ["Would remove 2 repeated instruction line(s)."] }] });
    expect(preview).toMatchObject({ sourcePath: "/repo/AGENTS.md", beforeTokens: 90, afterTokens: 50, duplicateTokensRemoved: 40 });
    expect(preview.diff).toContain("Would remove");
  });

  it("normalizes a content-free managed-change receipt", () => {
    const receipt = normalizeAgentMemoryReceipt({ receipt_id: "receipt-1", agent: "codex", backup_path: "/private/backup", rollback_available: true });
    expect(receipt).toMatchObject({ receiptId: "receipt-1", rollbackAvailable: true });
  });

  it("permits apply only for safe Switchboard-managed sources", () => {
    const source = normalizeAgentMemorySnapshot({ sources: [{ managedBySwitchboard: true, previewAvailable: true, status: "duplicate", secretScan: { status: "safe" } }] }).sources[0];
    const preview = normalizeAgentMemoryPreview({ secretScan: { status: "safe" }, applyEligible: true, confirmationPhrase: "APPLY AGENT MEMORY COMPACTION FOR CODEX" });
    expect(canApplyAgentMemoryCompaction(source, preview)).toBe(true);
    expect(canApplyAgentMemoryCompaction({ ...source, managedBySwitchboard: false }, preview)).toBe(false);
  });
});
