export interface CursorNativeSchemaAssessment {
  schemaId: string;
  supported: boolean;
  reason: string;
  docsUrl: string;
  surfacesDetected: number;
  evidence: string[];
}

export function describeCursorNativeGate(
  assessment: CursorNativeSchemaAssessment | null | undefined,
): {
  nativeWritesAllowed: boolean;
  sidecarAllowed: boolean;
  summary: string;
} {
  if (!assessment) {
    return {
      nativeWritesAllowed: false,
      sidecarAllowed: true,
      summary:
        "Cursor native provider writes remain blocked until a documented on-disk schema and full lifecycle proof exist.",
    };
  }

  if (assessment.supported) {
    return {
      nativeWritesAllowed: true,
      sidecarAllowed: true,
      summary: `Cursor native schema ${assessment.schemaId} is allowlisted.`,
    };
  }

  return {
    nativeWritesAllowed: false,
    sidecarAllowed: true,
    summary: `${assessment.reason} Sidecar routing and Repo Intelligence packs remain available.`,
  };
}

export type CursorNativePromotionVerdict = "blocked" | "native_promoted";

export function evaluateCursorNativePromotionGate(
  assessment: CursorNativeSchemaAssessment | null | undefined,
): {
  verdict: CursorNativePromotionVerdict;
  summary: string;
} {
  const gate = describeCursorNativeGate(assessment);
  return {
    verdict: gate.nativeWritesAllowed ? "native_promoted" : "blocked",
    summary: gate.summary,
  };
}
