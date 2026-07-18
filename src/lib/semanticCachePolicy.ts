export type CacheClass = "exact" | "semantic" | "bypass";
export type CacheReceiptLabel =
  | "cache-hit"
  | "cache-miss"
  | "bypass"
  | "invalidation";

export interface SemanticCacheRequest {
  provider: string;
  model: string;
  account: string;
  workspace: string;
  prompt: string;
  temperature?: number;
  streaming?: boolean;
  hasToolCall?: boolean;
  hasMcpCall?: boolean;
  sensitive?: boolean;
  noCache?: boolean;
  repoStateChanging?: boolean;
  openToolCalls?: number;
  exactCacheEligible?: boolean;
}

export interface SemanticCachePolicy {
  policyVersion: string;
  semanticTemperatureMax: number;
}

export interface CacheDecision {
  classification: CacheClass;
  reason?:
    | "streaming"
    | "tool-or-mcp"
    | "sensitive"
    | "no-cache-marker"
    | "high-temperature"
    | "rapid-repo-state"
    | "open-tool-calls";
}

export interface CacheReceipt {
  label: CacheReceiptLabel;
  classification: CacheClass;
  evidence: "estimated" | "observed";
  compression: "separate";
}

const DEFAULT_POLICY: SemanticCachePolicy = {
  policyVersion: "semantic-cache-v1",
  semanticTemperatureMax: 0.2,
};

export const defaultSemanticCachePolicy = DEFAULT_POLICY;

export const semanticCacheBypassReasons = [
  "streaming",
  "tool-or-mcp",
  "sensitive",
  "no-cache-marker",
  "high-temperature",
  "rapid-repo-state",
  "open-tool-calls",
] as const;

export function describeSemanticCachePolicy(): string {
  return "Separate cache savings: exact first, semantic opt-in; bypass streaming, tools/MCP, sensitive or no-cache requests, high temperature, rapid repo changes, and open tool calls.";
}

export function classifySemanticCacheRequest(
  request: SemanticCacheRequest,
  policy: SemanticCachePolicy = DEFAULT_POLICY,
): CacheDecision {
  if (request.streaming) return { classification: "bypass", reason: "streaming" };
  if (request.hasToolCall || request.hasMcpCall) {
    return { classification: "bypass", reason: "tool-or-mcp" };
  }
  if (request.sensitive) return { classification: "bypass", reason: "sensitive" };
  if (request.noCache) return { classification: "bypass", reason: "no-cache-marker" };
  if ((request.temperature ?? 0) > policy.semanticTemperatureMax) {
    return { classification: "bypass", reason: "high-temperature" };
  }
  if (request.repoStateChanging) {
    return { classification: "bypass", reason: "rapid-repo-state" };
  }
  if ((request.openToolCalls ?? 0) > 0) {
    return { classification: "bypass", reason: "open-tool-calls" };
  }
  return { classification: request.exactCacheEligible ? "exact" : "semantic" };
}

export function buildSemanticCacheNamespaceKey(
  request: Pick<SemanticCacheRequest, "provider" | "model" | "account" | "workspace">,
  policy: SemanticCachePolicy = DEFAULT_POLICY,
): string {
  return [request.provider, request.model, request.account, request.workspace, policy.policyVersion]
    .map((part) => encodeURIComponent(part))
    .join(":");
}

export function createCacheReceipt(
  label: CacheReceiptLabel,
  classification: CacheClass,
  evidence: "estimated" | "observed" = "estimated",
): CacheReceipt {
  return { label, classification, evidence, compression: "separate" };
}
