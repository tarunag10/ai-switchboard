import provenanceEvidence from "../../fixtures/pxpipe-promotion-evidence.json";

export interface PxpipeProvenanceEvidence {
  schemaVersion: number;
  headroomCapability: string;
  minHeadroomVersion: string;
  visualQualityChecklistSigned: boolean;
  requiredSignals: string[];
}

export type PxpipePromotionVerdict = "blocked" | "experimental_eligible";

export function evaluatePxpipePromotionGate(
  evidence: PxpipeProvenanceEvidence = provenanceEvidence as PxpipeProvenanceEvidence,
): { verdict: PxpipePromotionVerdict; reasons: string[] } {
  const reasons: string[] = [];
  if (!evidence.headroomCapability) {
    reasons.push("Headroom text_image capability id is missing from provenance evidence.");
  }
  if (!evidence.visualQualityChecklistSigned) {
    reasons.push("Visual quality checklist is not signed; pxpipe-text-image stays experimental.");
  }
  if (!Array.isArray(evidence.requiredSignals) || evidence.requiredSignals.length === 0) {
    reasons.push("Provenance evidence is missing requiredSignals.");
  }
  if (reasons.length > 0) {
    return { verdict: "blocked", reasons };
  }
  return {
    verdict: "experimental_eligible",
    reasons: [
      "Provenance fixture present; pxpipe remains experimental until live Headroom seam ships.",
    ],
  };
}

export function canPromotePxpipeExperimental(): boolean {
  return evaluatePxpipePromotionGate().verdict === "experimental_eligible";
}
