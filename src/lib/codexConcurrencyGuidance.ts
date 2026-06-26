import type { SwitchboardMode } from "./types";

export interface CodexConcurrencyGuidance {
  title: string;
  body: string;
  policies: string[];
  steps: string[];
  recommendedMode: SwitchboardMode;
  actionLabel: string;
}

export function codexConcurrencyGuidance(
  mode: SwitchboardMode,
  headroomDetail: string,
): CodexConcurrencyGuidance | null {
  const codexRouted =
    /codex/i.test(headroomDetail) && (mode === "full" || mode === "headroom");

  if (!codexRouted) {
    return null;
  }

  return {
    title: "Running several Codex goals?",
    body: "Headroom compression is best for one main Codex session. Use RTK only before running several heavy active Codex chats or goals so large requests do not stall behind compression.",
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
