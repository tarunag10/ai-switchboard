import {
  applyAgentMemoryCompaction,
  buildSafeMemorySummary,
  canApplyAgentMemoryCompaction,
  formatMemoryTokens,
  getAgentMemorySnapshot,
  normalizeAgentMemoryPreview,
  normalizeAgentMemoryReceipt,
  normalizeAgentMemorySnapshot,
  previewAgentMemoryCompaction,
  rollbackAgentMemoryCompaction,
} from "./agentMemory";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("agent memory contracts", () => {
  beforeEach(() => invokeMock.mockReset());
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

  it("normalizes malformed sources and secret scan aliases safely", () => {
    const snapshot = normalizeAgentMemorySnapshot({
      generated_at: 42,
      repo_path: 42,
      sources: [
        {
          id: 7,
          agent: "invalid",
          scope: "invalid",
          status: "applied",
          estimatedTokens: Number.POSITIVE_INFINITY,
          modifiedAt: 5,
          secretScan: { state: "unsafe", detail: "token", matches: ["api", ""] },
          recommendedAction: 7,
          rollbackConfirmationPhrase: "ROLLBACK",
        },
        null,
      ],
    });
    expect(snapshot).toMatchObject({ generatedAt: null, repoPath: null });
    expect(snapshot.sources[0]).toMatchObject({
      id: "7",
      agent: "shared",
      scope: "unknown",
      status: "missing",
      estimatedTokens: null,
      modifiedAt: null,
      secretScan: { status: "blocked", reason: "token", categories: ["api"] },
      rollbackAvailable: true,
    });
    expect(snapshot.sources[1]).toMatchObject({
      id: "memory-source-1",
      sourcePath: "Path unavailable",
    });
  });

  it("aggregates preview sources and applies blocked secret warnings", () => {
    const preview = normalizeAgentMemoryPreview({
      blocked_by_secrets: true,
      warnings: ["credential found", ""],
      sources: [
        {
          source_path: "/repo/MEMORY.md",
          before_tokens: 10,
          after_tokens: 7,
          estimated_tokens_saved: 3,
          diff_summary: ["remove duplicate", ""],
        },
        { before_tokens: "bad", after_tokens: 2 },
      ],
      confirmation_phrase: "CONFIRM",
      apply_eligible: true,
      apply_blocked_reason: "secret",
    });
    expect(preview).toMatchObject({
      sourcePath: "/repo/MEMORY.md",
      beforeTokens: 10,
      afterTokens: 9,
      duplicateTokensRemoved: 3,
      diff: "remove duplicate",
      summary: "credential found",
      confirmationPhrase: "CONFIRM",
      applyEligible: true,
      applyBlockedReason: "secret",
      secretScan: { status: "blocked", reason: "credential found" },
    });
    expect(normalizeAgentMemoryPreview(null)).toMatchObject({
      beforeTokens: null,
      afterTokens: null,
      duplicateTokensRemoved: null,
      sourcePath: null,
    });
  });

  it("normalizes receipt target aliases and summary fallback", () => {
    expect(
      normalizeAgentMemoryReceipt({
        id: 5,
        agent: "invalid",
        target_path: "/repo/AGENTS.md",
        backup_path: 9,
        applied_at: 10,
        status: "applied",
        rollback_confirmation_phrase: "UNDO",
      }),
    ).toEqual({
      receiptId: "5",
      agent: "shared",
      sourcePath: "/repo/AGENTS.md",
      backupPath: null,
      appliedAt: null,
      rollbackAvailable: false,
      summary: "Memory compaction applied.",
      rollbackConfirmationPhrase: "UNDO",
    });
  });

  it.each([
    ["managedBySwitchboard", false],
    ["previewAvailable", false],
    ["status", "blocked"],
    ["status", "user-managed"],
  ])("rejects apply when source %s is unsafe", (key, value) => {
    const source = normalizeAgentMemorySnapshot({
      sources: [
        {
          managedBySwitchboard: true,
          previewAvailable: true,
          status: "duplicate",
          secretScan: { status: "safe" },
          [key]: value,
        },
      ],
    }).sources[0];
    const preview = normalizeAgentMemoryPreview({
      secretScan: { status: "safe" },
      applyEligible: true,
      confirmationPhrase: "CONFIRM",
    });
    expect(canApplyAgentMemoryCompaction(source, preview)).toBe(false);
  });

  it("rejects absent, blocked, ineligible, and unconfirmed previews", () => {
    const source = normalizeAgentMemorySnapshot({
      sources: [{ managedBySwitchboard: true, previewAvailable: true, status: "live", secretScan: { status: "safe" } }],
    }).sources[0];
    expect(canApplyAgentMemoryCompaction(source, null)).toBe(false);
    for (const raw of [
      { secretScan: { status: "blocked" }, applyEligible: true, confirmationPhrase: "x" },
      { secretScan: { status: "safe" }, applyEligible: false, confirmationPhrase: "x" },
      { secretScan: { status: "safe" }, applyEligible: true },
    ]) {
      expect(canApplyAgentMemoryCompaction(source, normalizeAgentMemoryPreview(raw))).toBe(false);
    }
  });

  it("invokes snapshot and preview commands with exact optional payloads", async () => {
    invokeMock
      .mockResolvedValueOnce({ sources: [] })
      .mockResolvedValueOnce({ sources: [] })
      .mockResolvedValueOnce({ agent: "codex" });
    await getAgentMemorySnapshot("  /repo  ");
    await getAgentMemorySnapshot("   ");
    await previewAgentMemoryCompaction("/repo", "codex");
    expect(invokeMock.mock.calls).toEqual([
      ["get_agent_memory_snapshot", { repoPath: "/repo" }],
      ["get_agent_memory_snapshot", undefined],
      ["preview_agent_memory_compaction", { repoPath: "/repo", agent: "codex" }],
    ]);
  });

  it("applies and rolls back with content-free receipt payloads", async () => {
    invokeMock
      .mockResolvedValueOnce([{ receipt_id: "r1", agent: "codex" }])
      .mockResolvedValueOnce({ receipt_id: "r2", agent: "codex" });
    await expect(
      applyAgentMemoryCompaction("/repo", "codex", "APPLY"),
    ).resolves.toMatchObject({ receiptId: "r1" });
    await expect(
      rollbackAgentMemoryCompaction("r1", "ROLLBACK"),
    ).resolves.toMatchObject({ receiptId: "r2" });
    expect(invokeMock.mock.calls).toEqual([
      ["apply_agent_memory_compaction", { repoPath: "/repo", agent: "codex", confirmationPhrase: "APPLY" }],
      ["rollback_agent_memory_compaction", { receiptId: "r1", confirmationPhrase: "ROLLBACK" }],
    ]);
  });

  it("rejects apply responses without a receipt", async () => {
    invokeMock.mockResolvedValueOnce({ receipt: "missing" });
    await expect(
      applyAgentMemoryCompaction("/repo", "codex", "APPLY"),
    ).rejects.toThrow("did not return a change receipt");
  });

  it("formats unavailable, standard, and compact token values", () => {
    expect(formatMemoryTokens(null)).toBe("Unavailable");
    expect(formatMemoryTokens(12.34)).toBe("12.3");
    expect(formatMemoryTokens(1200)).toMatch(/1\.2K/i);
  });
});
