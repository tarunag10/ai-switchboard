import { invoke } from "@tauri-apps/api/core";

import type {
  MeasuredSavingsAttributionRequest,
  SavingsAttributionEvent,
} from "./types";

export type MeasuredAddonSavingsSource = Extract<
  SavingsAttributionEvent["source"],
  "caveman" | "compact_chinese" | "ponytail" | "markitdown"
>;

/**
 * Add-ons whose savings can be measured from an explicit before/after pair.
 * Installation, a managed-file change, or a smoke check alone is not a pair.
 */
export type AddonMeasurementSource = Extract<
  MeasuredAddonSavingsSource,
  "caveman" | "ponytail" | "markitdown"
>;

export interface AddonMeasurementEvidence {
  /** Where the unoptimized token count was observed. */
  baseline: string;
  /** Where the optimized token count was observed. */
  optimized: string;
}

export type AddonMeasurementValidationReason =
  | "unsupported_source"
  | "invalid_baseline_tokens"
  | "invalid_optimized_tokens"
  | "invalid_request_delta"
  | "missing_baseline_evidence"
  | "missing_optimized_evidence"
  | "empty_delta";

export interface AddonMeasurementValidation {
  valid: boolean;
  confidence: "measured" | "estimated";
  reason?: AddonMeasurementValidationReason;
}

export interface MeasuredAddonSavingsInput {
  source: MeasuredAddonSavingsSource;
  label?: string;
  baselineTokens: number;
  optimizedTokens: number;
  requestDelta?: number;
  /**
   * Independent evidence for both sides of the observed token pair. Without
   * it, an add-on remains estimated and cannot be recorded as measured.
   */
  measurementEvidence?: Partial<AddonMeasurementEvidence>;
  detail?: string;
}

export interface MeasuredAddonSavingsResult {
  recorded: boolean;
  tokensSaved: number;
  requestDelta: number;
  confidence: "measured" | "estimated";
  reason?: AddonMeasurementValidationReason;
}

const measuredAddonLabels: Record<MeasuredAddonSavingsSource, string> = {
  caveman: "Caveman",
  compact_chinese: "Compact Chinese",
  ponytail: "Ponytail",
  markitdown: "MarkItDown",
};

export type MeasuredSavingsInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

function isMeasurementSource(
  source: MeasuredAddonSavingsSource,
): source is AddonMeasurementSource {
  return source === "caveman" || source === "ponytail" || source === "markitdown";
}

function hasEvidence(value: string | undefined) {
  return Boolean(value?.trim());
}

function validTokenCount(value: number) {
  return Number.isFinite(value) && value >= 0 && Math.floor(value) <= Number.MAX_SAFE_INTEGER;
}

function validRequestDelta(value: number) {
  return Number.isFinite(value) && value > 0 && Math.floor(value) <= Number.MAX_SAFE_INTEGER;
}

function normalizedCount(value: number) {
  return Math.floor(value);
}

/**
 * Gates measured attribution on two independently described token readings.
 * Until this validates, callers must keep the add-on's savings confidence at
 * "estimated" and must not emit a measured attribution event.
 */
export function validateAddonMeasurement(
  input: MeasuredAddonSavingsInput,
): AddonMeasurementValidation {
  if (!isMeasurementSource(input.source)) {
    return { valid: false, confidence: "estimated", reason: "unsupported_source" };
  }
  if (!validTokenCount(input.baselineTokens)) {
    return {
      valid: false,
      confidence: "estimated",
      reason: "invalid_baseline_tokens",
    };
  }
  if (!validTokenCount(input.optimizedTokens)) {
    return {
      valid: false,
      confidence: "estimated",
      reason: "invalid_optimized_tokens",
    };
  }
  if (!validRequestDelta(input.requestDelta ?? 1)) {
    return {
      valid: false,
      confidence: "estimated",
      reason: "invalid_request_delta",
    };
  }
  if (!hasEvidence(input.measurementEvidence?.baseline)) {
    return {
      valid: false,
      confidence: "estimated",
      reason: "missing_baseline_evidence",
    };
  }
  if (!hasEvidence(input.measurementEvidence?.optimized)) {
    return {
      valid: false,
      confidence: "estimated",
      reason: "missing_optimized_evidence",
    };
  }
  if (normalizedCount(input.baselineTokens) <= normalizedCount(input.optimizedTokens)) {
    return { valid: false, confidence: "estimated", reason: "empty_delta" };
  }
  return { valid: true, confidence: "measured" };
}

function measurementDetail(input: MeasuredAddonSavingsInput) {
  const evidence = input.measurementEvidence!;
  const detail = input.detail?.trim();
  return [
    detail,
    `Baseline evidence: ${evidence.baseline!.trim()}.`,
    `Optimized evidence: ${evidence.optimized!.trim()}.`,
  ]
    .filter(Boolean)
    .join(" ");
}

export function buildMeasuredAddonSavingsRequest(
  input: MeasuredAddonSavingsInput,
): MeasuredSavingsAttributionRequest | null {
  const validation = validateAddonMeasurement(input);
  if (!validation.valid) {
    return null;
  }

  return {
    source: input.source,
    label: input.label?.trim() || measuredAddonLabels[input.source],
    baselineTokens: normalizedCount(input.baselineTokens),
    optimizedTokens: normalizedCount(input.optimizedTokens),
    requestDelta: normalizedCount(input.requestDelta ?? 1),
    detail: measurementDetail(input),
  };
}

export async function recordMeasuredAddonSavings(
  input: MeasuredAddonSavingsInput,
  invokeCommand: MeasuredSavingsInvoke = invoke,
): Promise<MeasuredAddonSavingsResult> {
  const validation = validateAddonMeasurement(input);
  const request = buildMeasuredAddonSavingsRequest(input);
  const requestDelta = validRequestDelta(input.requestDelta ?? 1)
    ? normalizedCount(input.requestDelta ?? 1)
    : 0;

  if (!request || !validation.valid) {
    return {
      recorded: false,
      tokensSaved: 0,
      requestDelta,
      confidence: "estimated",
      reason: validation.reason,
    };
  }

  await invokeCommand("record_measured_savings_attribution", { request });

  return {
    recorded: true,
    tokensSaved:
      normalizedCount(input.baselineTokens) - normalizedCount(input.optimizedTokens),
    requestDelta,
    confidence: "measured",
  };
}
