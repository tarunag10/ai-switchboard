import type { DailySavingsPoint, SwitchboardMode, UsageEvent } from "./types";

export interface CodexConcurrencyGuidance {
  title: string;
  body: string;
  riskLabel: string;
  riskTone: "watch" | "high";
  evidence: string[];
  policies: string[];
  steps: string[];
  recommendedMode: SwitchboardMode;
  actionLabel: string;
}

function formatTokens(value: number) {
  return new Intl.NumberFormat("en-US").format(Math.round(value));
}

export function codexConcurrencyGuidance(
  mode: SwitchboardMode,
  headroomDetail: string,
  recentUsage: UsageEvent[] = [],
  savedHistory: DailySavingsPoint[] = [],
): CodexConcurrencyGuidance | null {
  const codexRouted =
    /codex/i.test(headroomDetail) && (mode === "full" || mode === "headroom");

  if (!codexRouted) {
    return null;
  }

  const codexEvents = recentUsage.filter((event) => /codex/i.test(event.client));
  const codexTokenTotals = codexEvents.map(
    (event) => event.estimatedInputTokens + event.estimatedOutputTokens,
  );
  const largestRequestTokens = Math.max(0, ...codexTokenTotals);
  const totalRecentTokens = codexTokenTotals.reduce((sum, tokens) => sum + tokens, 0);
  const savedHistoryPressure = savedHistory
    .filter((point) => Math.max(0, point.totalTokensSent) > 0)
    .slice(-7);
  const largestHistoryDayTokens = Math.max(
    0,
    ...savedHistoryPressure.map((point) => Math.max(0, point.totalTokensSent)),
  );
  const totalHistoryTokens = savedHistoryPressure.reduce(
    (sum, point) => sum + Math.max(0, point.totalTokensSent),
    0,
  );
  const historyHighRisk =
    largestHistoryDayTokens >= 300_000 ||
    (savedHistoryPressure.length >= 3 && totalHistoryTokens >= 600_000);
  const historyWatchRisk =
    historyHighRisk ||
    largestHistoryDayTokens >= 150_000 ||
    totalHistoryTokens >= 300_000;
  const highRisk =
    largestRequestTokens >= 120_000 ||
    (codexEvents.length >= 3 && totalRecentTokens >= 150_000) ||
    historyHighRisk;
  const watchRisk =
    highRisk ||
    largestRequestTokens >= 60_000 ||
    codexEvents.length >= 2 ||
    totalRecentTokens >= 90_000 ||
    historyWatchRisk;
  const riskLabel = highRisk
    ? "High context pressure"
    : watchRisk
      ? "Context pressure watch"
      : "Preventive guidance";
  const riskTone = highRisk ? "high" : "watch";
  const recentEvidence =
    codexEvents.length > 0
      ? [
          `${codexEvents.length.toLocaleString()} recent Codex request${codexEvents.length === 1 ? "" : "s"}.`,
          `Largest recent Codex request: ${formatTokens(largestRequestTokens)} tokens.`,
          `Recent Codex total: ${formatTokens(totalRecentTokens)} tokens.`,
        ]
      : ["No recent Codex token events in this app session yet."];
  const historyEvidence =
    savedHistoryPressure.length > 0
      ? [
          `${savedHistoryPressure.length.toLocaleString()} saved local history day${savedHistoryPressure.length === 1 ? "" : "s"} with token traffic.`,
          `Largest saved history day: ${formatTokens(largestHistoryDayTokens)} tokens sent.`,
          `Saved history total: ${formatTokens(totalHistoryTokens)} tokens sent.`,
        ]
      : ["No saved local token history is available yet."];
  const evidence = [
    ...recentEvidence,
    ...historyEvidence,
    "Guidance is based on Codex being routed through Headroom.",
  ];

  return {
    title: "Running several Codex goals?",
    body: highRisk
      ? "Codex or saved local token history is large enough that Headroom compression can stall. Compact the largest conversation or switch to RTK only before opening more heavy Codex work."
      : "Headroom compression is best for one main Codex session. Use RTK only before running several heavy active Codex chats or goals so large requests do not stall behind compression.",
    riskLabel,
    riskTone,
    evidence,
    policies: [
      "Full: one main Codex session",
      "RTK only: 2+ heavy sessions",
      "After 413: compact, then reset Codex in Doctor",
      "Unsupported model: Repair Codex setup",
    ],
    steps: [
      "Switch to RTK only before opening several active Codex chats or goals.",
      "Compact or close stale Codex conversations before turning Headroom routing back on.",
      "If Codex was bypassed after a 413 compression_refused error, run Doctor to reset the bypass.",
      "If Codex says the model is unsupported with a ChatGPT account, use Doctor's Repair Codex action instead.",
    ],
    recommendedMode: "rtk",
    actionLabel: "Switch to RTK only",
  };
}
