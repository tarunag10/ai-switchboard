import { invoke } from "@tauri-apps/api/core";

import type {
  MeasuredSavingsAttributionRequest,
  SavingsAttributionEvent,
} from "./types";

export type MeasuredAddonSavingsSource = Extract<
  SavingsAttributionEvent["source"],
  "caveman" | "compact_chinese" | "ponytail" | "markitdown"
>;

export interface MeasuredAddonSavingsInput {
  source: MeasuredAddonSavingsSource;
  label?: string;
  baselineTokens: number;
  optimizedTokens: number;
  requestDelta?: number;
  detail?: string;
}

export interface MeasuredAddonSavingsResult {
  recorded: boolean;
  tokensSaved: number;
  requestDelta: number;
  reason?: "empty_delta" | "invalid_request_delta";
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

function finiteTokenCount(value: number) {
  return Number.isFinite(value) ? Math.max(0, Math.floor(value)) : 0;
}

export function buildMeasuredAddonSavingsRequest(
  input: MeasuredAddonSavingsInput,
): MeasuredSavingsAttributionRequest | null {
  const baselineTokens = finiteTokenCount(input.baselineTokens);
  const optimizedTokens = finiteTokenCount(input.optimizedTokens);
  const requestDelta = finiteTokenCount(input.requestDelta ?? 1);

  if (requestDelta === 0 || baselineTokens <= optimizedTokens) {
    return null;
  }

  return {
    source: input.source,
    label: input.label?.trim() || measuredAddonLabels[input.source],
    baselineTokens,
    optimizedTokens,
    requestDelta,
    detail: input.detail?.trim() ?? "",
  };
}

export async function recordMeasuredAddonSavings(
  input: MeasuredAddonSavingsInput,
  invokeCommand: MeasuredSavingsInvoke = invoke,
): Promise<MeasuredAddonSavingsResult> {
  const request = buildMeasuredAddonSavingsRequest(input);
  const requestDelta = finiteTokenCount(input.requestDelta ?? 1);
  const baselineTokens = finiteTokenCount(input.baselineTokens);
  const optimizedTokens = finiteTokenCount(input.optimizedTokens);

  if (requestDelta === 0) {
    return {
      recorded: false,
      tokensSaved: 0,
      requestDelta,
      reason: "invalid_request_delta",
    };
  }

  if (!request) {
    return {
      recorded: false,
      tokensSaved: 0,
      requestDelta,
      reason: "empty_delta",
    };
  }

  await invokeCommand("record_measured_savings_attribution", { request });

  return {
    recorded: true,
    tokensSaved: baselineTokens - optimizedTokens,
    requestDelta,
  };
}
