export type CodexErrorKind =
  | "compression_refused"
  | "unsupported_chatgpt_model"
  | "provider_auth_scope_missing"
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

const PROVIDER_AUTH_SCOPE_MISSING_GUIDANCE: CodexErrorGuidance = {
  kind: "provider_auth_scope_missing",
  title: "Provider authorization is missing Responses: Write",
  summary:
    "The upstream provider rejected the credential or its organization/project authorization. This is an upstream authorization issue, not token compression. Switchboard does not need or display your secret.",
  action:
    "Use a valid project key with Responses: Write or ChatGPT/Codex OAuth, then retry. Bypass Headroom or route direct only as a diagnostic; it will not repair the credential.",
  steps: [
    "Use a project API key whose permissions include Responses: Write, or authenticate with ChatGPT/Codex OAuth.",
    "Verify the credential's organization and project access without pasting or printing the secret.",
    "As a routing diagnostic, bypass Headroom and retry direct; if direct also fails, fix upstream authorization rather than compression.",
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

  if (
    (text.includes("401") || text.includes("unauthorized")) &&
    (text.includes("missing scope") || text.includes("insufficient permission")) &&
    text.includes("api.responses.write")
  ) {
    return PROVIDER_AUTH_SCOPE_MISSING_GUIDANCE;
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
