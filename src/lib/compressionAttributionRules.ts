export type CompressionAttributionLabel =
  | "measured"
  | "estimated"
  | "inferred"
  | "external";

export type CompressionAttributionFamily =
  | "compression"
  | "cache"
  | "context-avoidance"
  | "addon"
  | "gateway";

export interface CompressionAttributionRule {
  source: string;
  label: CompressionAttributionLabel;
  family: CompressionAttributionFamily;
}

/** Cross-cutting attribution table from the comprehensive compression design (§4.1). */
export const compressionAttributionRules: readonly CompressionAttributionRule[] = [
  {
    source: "Headroom /stats delta",
    label: "measured",
    family: "compression",
  },
  {
    source: "RTK gain stats",
    label: "measured",
    family: "compression",
  },
  {
    source: "Exact/semantic cache hit",
    label: "estimated",
    family: "cache",
  },
  {
    source: "Repo Intelligence pack",
    label: "estimated",
    family: "context-avoidance",
  },
  {
    source: "MarkItDown/Ponytail/Caveman",
    label: "inferred",
    family: "addon",
  },
  {
    source: "LiteLLM/Cloudflare",
    label: "external",
    family: "gateway",
  },
] as const;

export function compressionAttributionFamilyLabel(
  family: CompressionAttributionFamily,
): string {
  switch (family) {
    case "compression":
      return "Compression";
    case "cache":
      return "Cache";
    case "context-avoidance":
      return "Context avoidance";
    case "addon":
      return "Add-on";
    case "gateway":
      return "Gateway";
  }
}

export function describeCompressionAttributionPolicy(): string {
  return "Cache replay, context-avoidance, add-on, and gateway savings stay separate from live Headroom compression; labels remain measured, estimated, inferred, or external.";
}
