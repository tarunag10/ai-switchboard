export type CodexErrorKind =
  | "compression_refused"
  | "unsupported_chatgpt_model"
  | "unknown";

export interface CodexErrorGuidance {
  kind: CodexErrorKind;
  title: string;
  summary: string;
  action: string;
}

const COMPRESSION_REFUSED_GUIDANCE: CodexErrorGuidance = {
  kind: "compression_refused",
  title: "Codex request was too large for Headroom compression",
  summary:
    "This is the Headroom 413 path: Headroom timed out while compacting an oversized request, then the switchboard can bypass Codex temporarily.",
  action:
    "Compact the Codex conversation or switch to RTK only, then reset the Codex bypass when you want Headroom routing again.",
};

const UNSUPPORTED_CHATGPT_MODEL_GUIDANCE: CodexErrorGuidance = {
  kind: "unsupported_chatgpt_model",
  title: "Codex model/provider config is unsupported",
  summary:
    "This is separate from Headroom compression. Codex is trying to use a blank or unsupported model with a ChatGPT account.",
  action:
    "Repair Codex setup to re-apply the managed provider block, then choose a Codex-supported ChatGPT model before retrying.",
};

const UNKNOWN_GUIDANCE: CodexErrorGuidance = {
  kind: "unknown",
  title: "Codex error needs review",
  summary:
    "The switchboard does not recognize this as a known Headroom compression or ChatGPT model configuration failure.",
  action:
    "Run Doctor, check Codex provider settings, and inspect the latest Headroom log before changing routing.",
};

export function classifyCodexError(message: string | null | undefined) {
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
    return `${UNSUPPORTED_CHATGPT_MODEL_GUIDANCE.action} This also refreshes the localhost proxy URL with backups.`;
  }

  return null;
}
