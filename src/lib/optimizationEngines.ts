import { describeSemanticCachePolicy } from "./semanticCachePolicy";

export const optimizationEngineIds = [
  "headroom-native",
  "rtk",
  "leanctx",
  "llmlingua-2",
  "chonkify",
  "semantic-cache",
  "pxpipe-text-image",
] as const;

export type OptimizationEngineId = (typeof optimizationEngineIds)[number];
export type OptimizationEngineStatus =
  | "disabled"
  | "available"
  | "shadow"
  | "enabled"
  | "needs-repair"
  | "blocked";
export type OptimizationBoundary = "local" | "remote";
export type OptimizationVisibility = "none" | "prompt" | "output" | "prompt-and-output";
export type OptimizationLossiness = "lossless" | "lossy" | "configurable";
export type OptimizationScope = "text" | "image" | "text-and-image";
export type OptimizationEvidenceType = "native-metric" | "command-output" | "benchmark" | "cache-hit-rate" | "manual-review";
export type OptimizationReceiptScope = "live-request" | "repo-pack" | "cache-hit" | "shadow";
export type OptimizationReceiptEvidence = "measured" | "estimated" | "external" | "none";

export interface OptimizationReceipt {
  id: string;
  engine: OptimizationEngineId;
  scope: OptimizationReceiptScope;
  beforeTokens?: number;
  afterTokens?: number;
  savedTokens?: number;
  savedUsd?: number;
  latencyMs?: number;
  fallbackReason?: string;
  protectedBytes?: number;
  evidence: OptimizationReceiptEvidence;
  provider?: string;
  model?: string;
  createdAt: string;
}

export interface OptimizationEngine {
  id: OptimizationEngineId;
  label: string;
  status: OptimizationEngineStatus;
  boundary: OptimizationBoundary;
  visibility: OptimizationVisibility;
  lossiness: OptimizationLossiness;
  supportedScope: OptimizationScope;
  evidenceType: OptimizationEvidenceType;
  setup: string;
  rollback: string;
  off: string;
  governance: {
    userOptIn: boolean;
    secretSafePreview: boolean;
    reversible: boolean;
    remoteDisclosure: boolean;
    evidenceRequired: boolean;
  };
  config: Record<string, string | number | boolean | null>;
}

export function createOptimizationLifecycleReceipt(
  engine: OptimizationEngineId,
  action: string,
): OptimizationReceipt {
  const scope: OptimizationReceiptScope = engine === "chonkify"
    ? "repo-pack"
    : engine === "semantic-cache"
      ? "cache-hit"
      : engine === "leanctx" || engine === "llmlingua-2" || engine === "pxpipe-text-image"
        ? "shadow"
        : "live-request";
  return {
    id: `${engine}-${Date.now()}`,
    engine,
    scope,
    evidence: "none",
    fallbackReason: `Lifecycle action ${action}; no optimization measurement was recorded.`,
    createdAt: new Date().toISOString(),
  };
}

const guidance = (setup: string, rollback: string, off: string) => ({ setup, rollback, off });

export const optimizationEngines: readonly OptimizationEngine[] = [
  { id: "headroom-native", label: "Headroom Native", status: "enabled", boundary: "local", visibility: "prompt", lossiness: "configurable", supportedScope: "text-and-image", evidenceType: "native-metric", ...guidance("Use the bundled native engine.", "Restore the previous Switchboard profile.", "Disable native optimization in Settings."), governance: { userOptIn: true, secretSafePreview: true, reversible: true, remoteDisclosure: false, evidenceRequired: true }, config: {} },
  { id: "rtk", label: "RTK", status: "available", boundary: "local", visibility: "prompt", lossiness: "configurable", supportedScope: "text", evidenceType: "command-output", ...guidance("Install or select the local RTK binary.", "Remove the RTK preset from the profile.", "Turn off RTK presets."), governance: { userOptIn: true, secretSafePreview: true, reversible: true, remoteDisclosure: false, evidenceRequired: true }, config: {} },
  { id: "leanctx", label: "Lean Context", status: "shadow", boundary: "local", visibility: "prompt", lossiness: "configurable", supportedScope: "text", evidenceType: "benchmark", ...guidance("Configure a local context budget and observe-only mode.", "Restore the uncompressed context pack.", "Set the engine to disabled."), governance: { userOptIn: true, secretSafePreview: true, reversible: true, remoteDisclosure: false, evidenceRequired: true }, config: {} },
  { id: "llmlingua-2", label: "LLMLingua-2", status: "blocked", boundary: "local", visibility: "prompt", lossiness: "lossy", supportedScope: "text", evidenceType: "benchmark", ...guidance("Install the local model and record a quality baseline.", "Discard the compressed prompt and use the original.", "Keep the engine blocked until quality gates pass."), governance: { userOptIn: true, secretSafePreview: true, reversible: true, remoteDisclosure: false, evidenceRequired: true }, config: {} },
  { id: "chonkify", label: "Chonkify", status: "blocked", boundary: "local", visibility: "prompt", lossiness: "lossy", supportedScope: "text", evidenceType: "manual-review", ...guidance("Confirm license and source-provenance evidence before installation.", "Restore the original context pack from backup.", "Keep disabled until license and provenance gates pass."), governance: { userOptIn: true, secretSafePreview: true, reversible: true, remoteDisclosure: false, evidenceRequired: true }, config: {} },
  { id: "semantic-cache", label: "Exact Replay Cache", status: "available", boundary: "local", visibility: "prompt-and-output", lossiness: "configurable", supportedScope: "text", evidenceType: "cache-hit-rate", ...guidance(describeSemanticCachePolicy(), "Clear the cache and resume direct requests.", "Disable cache reads and writes."), governance: { userOptIn: true, secretSafePreview: true, reversible: true, remoteDisclosure: false, evidenceRequired: true }, config: { policy: "exact-v1", evidence: "estimated until counterfactual provider evidence", compression: "separate" } },
  { id: "pxpipe-text-image", label: "PXPipe Text/Image", status: "blocked", boundary: "local", visibility: "prompt", lossiness: "lossy", supportedScope: "text", evidenceType: "manual-review", ...guidance("Wait for a versioned Headroom text_image capability and run shadow mode.", "Restore native Headroom text handling.", "Keep disabled until upstream support and quality gates pass."), governance: { userOptIn: true, secretSafePreview: true, reversible: true, remoteDisclosure: false, evidenceRequired: true }, config: {} },
] as const;

export function validateOptimizationEngineGovernance(engines: readonly OptimizationEngine[] = optimizationEngines): string[] {
  const errors: string[] = [];
  const ids = new Set<string>();
  for (const engine of engines) {
    if (ids.has(engine.id)) errors.push(`duplicate engine id: ${engine.id}`);
    ids.add(engine.id);
    if (!engine.setup || !engine.rollback || !engine.off) errors.push(`missing lifecycle guidance: ${engine.id}`);
    if (!engine.governance.userOptIn || !engine.governance.secretSafePreview || !engine.governance.reversible || !engine.governance.evidenceRequired) errors.push(`governance gate failed: ${engine.id}`);
    if (engine.boundary === "remote" && !engine.governance.remoteDisclosure) errors.push(`remote disclosure missing: ${engine.id}`);
  }
  return errors;
}

export function summarizeOptimizationEngineStatus(engines: readonly OptimizationEngine[] = optimizationEngines): string {
  return engines.map(({ id, status }) => `${id}: ${status}`).join("; ");
}

const secretKey = /(?:api[_-]?key|token|secret|password|authorization|credential)/i;
export function previewOptimizationEngineConfig(engine: OptimizationEngine, config: Record<string, unknown> = engine.config): Record<string, string | number | boolean | null> {
  return Object.fromEntries(Object.entries(config).map(([key, value]) => [key, secretKey.test(key) ? "[redacted]" : typeof value === "string" || typeof value === "number" || typeof value === "boolean" || value === null ? value : "[omitted]"])) as Record<string, string | number | boolean | null>;
}
