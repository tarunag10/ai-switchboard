import { invoke } from "@tauri-apps/api/core";

export type ProviderBilledProvider = "codex" | "claude" | "headroom_stats";

export interface ProviderBilledReading {
  provider: ProviderBilledProvider;
  billedInputTokens: number;
  sourceEndpoint: string;
  observedAt: string;
}

export interface ProviderBilledCounterfactualInput {
  provider: ProviderBilledProvider;
  baselineTokens: number;
  optimizedTokens: number;
  baselineEvidence: string;
  optimizedEvidence: string;
  requestDelta?: number;
}

export type ProviderBilledValidationReason =
  | "invalid_baseline_tokens"
  | "invalid_optimized_tokens"
  | "invalid_request_delta"
  | "missing_baseline_evidence"
  | "missing_optimized_evidence"
  | "empty_delta";

export interface ProviderBilledValidation {
  valid: boolean;
  confidence: "measured" | "estimated";
  reason?: ProviderBilledValidationReason;
}

export interface ProviderBilledRecordResult {
  recorded: boolean;
  tokensSaved: number;
  requestDelta: number;
  confidence: "measured" | "estimated";
  reason?: ProviderBilledValidationReason;
}

const providerLabels: Record<ProviderBilledProvider, string> = {
  codex: "Codex usage endpoint",
  claude: "Claude OAuth usage endpoint",
  headroom_stats: "Headroom /stats",
};

function validTokenCount(value: number) {
  return Number.isFinite(value) && value > 0 && Math.floor(value) <= Number.MAX_SAFE_INTEGER;
}

function validRequestDelta(value: number) {
  return Number.isFinite(value) && value > 0 && Math.floor(value) <= Number.MAX_SAFE_INTEGER;
}

export function validateProviderBilledCounterfactual(
  input: ProviderBilledCounterfactualInput,
): ProviderBilledValidation {
  if (!validTokenCount(input.baselineTokens)) {
    return { valid: false, confidence: "estimated", reason: "invalid_baseline_tokens" };
  }
  if (!validTokenCount(input.optimizedTokens)) {
    return { valid: false, confidence: "estimated", reason: "invalid_optimized_tokens" };
  }
  if (!validRequestDelta(input.requestDelta ?? 1)) {
    return { valid: false, confidence: "estimated", reason: "invalid_request_delta" };
  }
  if (!input.baselineEvidence.trim()) {
    return { valid: false, confidence: "estimated", reason: "missing_baseline_evidence" };
  }
  if (!input.optimizedEvidence.trim()) {
    return { valid: false, confidence: "estimated", reason: "missing_optimized_evidence" };
  }
  if (Math.floor(input.baselineTokens) <= Math.floor(input.optimizedTokens)) {
    return { valid: false, confidence: "estimated", reason: "empty_delta" };
  }
  return { valid: true, confidence: "measured" };
}

export function describeProviderBilledProvider(provider: ProviderBilledProvider) {
  return providerLabels[provider];
}

export async function loadProviderBilledUsageSnapshot(): Promise<ProviderBilledReading | null> {
  try {
    return await invoke<ProviderBilledReading>("get_provider_billed_usage_snapshot");
  } catch {
    return null;
  }
}

export async function recordProviderBilledCounterfactual(
  input: ProviderBilledCounterfactualInput,
): Promise<ProviderBilledRecordResult> {
  const validation = validateProviderBilledCounterfactual(input);
  const requestDelta = validRequestDelta(input.requestDelta ?? 1)
    ? Math.floor(input.requestDelta ?? 1)
    : 0;

  if (!validation.valid) {
    return {
      recorded: false,
      tokensSaved: 0,
      requestDelta,
      confidence: "estimated",
      reason: validation.reason,
    };
  }

  await invoke("record_provider_billed_counterfactual", {
    request: {
      provider: input.provider,
      baselineTokens: Math.floor(input.baselineTokens),
      optimizedTokens: Math.floor(input.optimizedTokens),
      baselineEvidence: input.baselineEvidence.trim(),
      optimizedEvidence: input.optimizedEvidence.trim(),
      requestDelta,
    },
  });

  return {
    recorded: true,
    tokensSaved: Math.floor(input.baselineTokens) - Math.floor(input.optimizedTokens),
    requestDelta,
    confidence: "measured",
  };
}
