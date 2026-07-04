export type CodexErrorKind =
  | "compression_refused"
  | "unsupported_chatgpt_model"
  | "unknown";

export interface CodexErrorGuidance {
  kind: CodexErrorKind;
  title: string;
  summary: string;
  action: string;
  steps: string[];
}

const COMPRESSION_REFUSED_GUIDANCE: CodexErrorGuidance = {
  kind: "compression_refused",
  title: "Codex auto-routed around Headroom compression",
  summary:
    "Switchboard preflights oversized Codex turns and routes them direct so Codex can use its native compact/retry flow.",
  action:
    "Retry Codex normally. Run Doctor only if this keeps happening after the automatic route change.",
  steps: [
    "Continue the Codex conversation; Switchboard will bypass oversized turns automatically.",
    "Use Compact Conversation if Codex itself asks for room.",
    "Run Doctor Codex optimization only if repeated failures continue.",
  ],
};

const UNSUPPORTED_CHATGPT_MODEL_GUIDANCE: CodexErrorGuidance = {
  kind: "unsupported_chatgpt_model",
  title: "Codex model/provider config is unsupported",
  summary:
    "This is separate from Headroom compression. Codex is trying to use a blank or unsupported model with a ChatGPT account.",
  action:
    "Repair Codex setup to re-apply the managed provider block, then choose a Codex-supported ChatGPT model before retrying.",
  steps: [
    "Run Doctor and choose Repair Codex.",
    "Check Codex model/provider settings for a blank or unsupported model name.",
    "Retry after selecting a model supported by your Codex ChatGPT account.",
  ],
};

const UNKNOWN_GUIDANCE: CodexErrorGuidance = {
  kind: "unknown",
  title: "Codex error needs review",
  summary:
    "The switchboard does not recognize this as a known Headroom compression or ChatGPT model configuration failure.",
  action:
    "Run Doctor, check Codex provider settings, and inspect the latest Headroom log before changing routing.",
  steps: [
    "Run Doctor first.",
    "Review Codex provider settings.",
    "Inspect the latest Headroom log before changing routing.",
  ],
};

export function classifyCodexError(
  message: string | null | undefined,
): CodexErrorGuidance {
  const text = (message ?? "").toLowerCase();

  if (
    text.includes("413") &&
    text.includes("compression_refused") &&
    text.includes("headroom")
  ) {
    return COMPRESSION_REFUSED_GUIDANCE;
  }

  if (
    text.includes("model") &&
    text.includes("not supported") &&
    text.includes("codex") &&
    text.includes("chatgpt account")
  ) {
    return UNSUPPORTED_CHATGPT_MODEL_GUIDANCE;
  }

  return UNKNOWN_GUIDANCE;
}

export function codexDoctorHint(action: string) {
  if (action === "reset_codex_bypass") {
    return COMPRESSION_REFUSED_GUIDANCE.action;
  }

  if (action === "repair_codex_setup") {
    return UNSUPPORTED_CHATGPT_MODEL_GUIDANCE.action;
  }

  return null;
}
