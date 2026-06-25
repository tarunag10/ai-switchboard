import { describe, expect, it } from "vitest";

import { classifyCodexError, codexDoctorHint } from "./codexErrorGuidance";

describe("codex error guidance", () => {
  it("classifies Headroom compression refusal as the 413 bypass path", () => {
    const guidance = classifyCodexError(
      'unexpected status 413 Payload Too Large: {"detail":{"error":{"type":"compression_refused","message":"headroom: compression timeout on a 1510408-byte request - please compact context and retry."}}}',
    );

    expect(guidance.kind).toBe("compression_refused");
    expect(guidance.summary).toContain("Headroom 413 path");
    expect(guidance.action).toContain("Compact the Codex conversation");
    expect(codexDoctorHint("reset_codex_bypass")).toBe(guidance.action);
  });

  it("classifies unsupported ChatGPT account model errors as config failures", () => {
    const guidance = classifyCodexError(
      "The '' model is not supported when using Codex with a ChatGPT account.",
    );

    expect(guidance.kind).toBe("unsupported_chatgpt_model");
    expect(guidance.summary).toContain("separate from Headroom compression");
    expect(guidance.action).toContain("Repair Codex setup");
    expect(codexDoctorHint("repair_codex_setup")).toContain(
      "choose a Codex-supported ChatGPT model",
    );
  });

  it("falls back for unknown Codex errors", () => {
    const guidance = classifyCodexError("Codex failed unexpectedly.");

    expect(guidance.kind).toBe("unknown");
    expect(guidance.action).toContain("Run Doctor");
    expect(codexDoctorHint("repair_runtime")).toBeNull();
  });
});
