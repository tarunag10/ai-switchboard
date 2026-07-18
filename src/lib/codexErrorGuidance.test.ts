import { describe, expect, it } from "vitest";

import { classifyCodexError, codexDoctorHint } from "./codexErrorGuidance";

describe("codex error guidance", () => {
  it("classifies Headroom compression refusal as auto-recoverable", () => {
    const guidance = classifyCodexError(
      'unexpected status 413 Payload Too Large: {"detail":{"error":{"type":"compression_refused","message":"headroom: compression timeout on 1510408-byte request - please compact context and retry."}}}',
    );

    expect(guidance.kind).toBe("compression_refused");
    expect(guidance.summary).toContain("preflights oversized Codex");
    expect(guidance.action).toContain("Retry Codex normally");
    expect(guidance.steps).toContain(
      "Run Doctor Codex optimization only if repeated failures continue.",
    );
    expect(codexDoctorHint("reset_codex_bypass")).toBe(guidance.action);
  });

  it("classifies unsupported ChatGPT account model errors as config failures", () => {
    const guidance = classifyCodexError(
      "The '' model is not supported when using Codex with a ChatGPT account.",
    );

    expect(guidance.kind).toBe("unsupported_chatgpt_model");
    expect(guidance.summary).toContain("separate from Headroom compression");
    expect(guidance.action).toContain("Repair Codex setup");
    expect(guidance.steps).toContain("Run Doctor and choose Repair Codex.");
    expect(codexDoctorHint("repair_codex_setup")).toContain(
      "choose a Codex-supported ChatGPT model",
    );
  });

  it("classifies missing Responses write authorization as an upstream provider failure", () => {
    const guidance = classifyCodexError(
      "unexpected status 401 Unauthorized: You have insufficient permissions for this operation. Missing scopes: api.responses.write.",
    );

    expect(guidance.kind).toBe("provider_auth_scope_missing");
    expect(guidance.summary).toContain("upstream provider");
    expect(guidance.summary).toContain("not token compression");
    expect(guidance.action).toContain("Responses: Write");
    expect(guidance.action).toContain("ChatGPT/Codex OAuth");
    expect(guidance.action).toContain("diagnostic");
    expect(guidance.action).toContain("will not repair the credential");
    expect(guidance.steps).toContain(
      "Use a project API key whose permissions include Responses: Write, or authenticate with ChatGPT/Codex OAuth.",
    );
    expect(guidance.steps).toContain(
      "As a routing diagnostic, bypass Headroom and retry direct; if direct also fails, fix upstream authorization rather than compression.",
    );
    expect(guidance.steps).toContain(
      "Verify the credential's organization and project access without pasting or printing the secret.",
    );
  });

  it("recognizes Unauthorized messages that describe a missing scope", () => {
    const guidance = classifyCodexError(
      "Unauthorized (401): insufficient permissions; missing scope api.responses.write",
    );

    expect(guidance.kind).toBe("provider_auth_scope_missing");
  });

  it("falls back for unknown Codex errors", () => {
    const guidance = classifyCodexError("Codex failed unexpectedly.");

    expect(guidance.kind).toBe("unknown");
    expect(guidance.action).toContain("Run Doctor");
    expect(guidance.steps).toContain("Review Codex provider settings.");
    expect(codexDoctorHint("repair_runtime")).toBeNull();
  });
});
