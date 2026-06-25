import type { SwitchboardMode } from "./types";

export interface CodexConcurrencyGuidance {
  title: string;
  body: string;
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
    body: "Use RTK only for multiple heavy active Codex chats or goals. Keep Full optimization for one main Codex session after compacting context.",
    steps: [
      "Switch to RTK only before opening several active Codex chats.",
      "Compact or close stale Codex conversations before turning Headroom routing back on.",
      "If Codex was bypassed after a 413 compression_refused error, run Doctor to reset the bypass.",
    ],
    recommendedMode: "rtk",
    actionLabel: "Switch to RTK only",
  };
}
