import type { SwitchboardMode } from "./types";

export interface CodexConcurrencyGuidance {
  title: string;
  body: string;
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
    body: "Use RTK only for multiple heavy active Codex chats or goals; keep Full optimization for one main Codex session after compacting context.",
  };
}
