import provenanceEvidence from "../../fixtures/chonkify-provenance-evidence.json";
import wrongOmissionFixtures from "../../fixtures/chonkify-wrong-omission-fixtures.json";

export interface ChonkifyProvenanceEvidence {
  schemaVersion: number;
  license: string;
  sourceRepository?: string;
  implementationId?: string;
  legacyCompatibilityId?: string;
  implementationOwner?: string;
  upstreamCodeEmbedded?: boolean;
  provenanceReviewedAt?: string;
  requiredSignals: string[];
  wrongOmissionFixturesPath: string;
  maxWrongOmissionRatePct: number;
  notes?: string[];
}

export interface ChonkifyWrongOmissionFixture {
  name: string;
  relevantFacts: string[];
  wrongOmissions: string[];
}

export type ChonkifyPromotionVerdict = "blocked" | "repo_pack_eligible";

export function wrongOmissionRatePct(fixture: ChonkifyWrongOmissionFixture): number {
  if (fixture.relevantFacts.length === 0) return 0;
  return (
    Math.round(
      (fixture.wrongOmissions.length / fixture.relevantFacts.length) * 1000,
    ) / 10
  );
}

export function evaluateChonkifyPromotionGate(
  evidence: ChonkifyProvenanceEvidence = provenanceEvidence as ChonkifyProvenanceEvidence,
  fixtures: { fixtures: ChonkifyWrongOmissionFixture[] } = wrongOmissionFixtures as {
    fixtures: ChonkifyWrongOmissionFixture[];
  },
): { verdict: ChonkifyPromotionVerdict; reasons: string[] } {
  const reasons: string[] = [];
  if (evidence.license !== "MIT") {
    reasons.push("the Switchboard-native pack compactor must remain covered by the repository MIT licence");
  }
  if (evidence.implementationId !== "switchboard-pack-compaction") {
    reasons.push("implementation identity must be switchboard-pack-compaction");
  }
  if (evidence.implementationOwner !== "ai-switchboard") {
    reasons.push("implementation owner must be AI Switchboard");
  }
  if (evidence.upstreamCodeEmbedded !== false) {
    reasons.push("upstream Chonkify code must not be claimed by the Switchboard-native compactor");
  }
  if (!Array.isArray(evidence.requiredSignals) || evidence.requiredSignals.length === 0) {
    reasons.push("provenance evidence is missing requiredSignals");
  }
  const maxRate = evidence.maxWrongOmissionRatePct ?? 0;
  for (const fixture of fixtures.fixtures ?? []) {
    const rate = wrongOmissionRatePct(fixture);
    if (rate > maxRate) {
      reasons.push(
        `fixture "${fixture.name}" wrong-omission rate ${rate}% exceeds gate ${maxRate}%`,
      );
    }
  }
  if (reasons.length > 0) {
    return { verdict: "blocked", reasons };
  }
  return {
    verdict: "repo_pack_eligible",
    reasons: [
      "AI Switchboard-native implementation identity and MIT coverage passed review.",
      "Wrong-omission fixtures remain at or below the promotion gate.",
    ],
  };
}

export function canActivateChonkifyRepoPack(
  evidence: ChonkifyProvenanceEvidence = provenanceEvidence as ChonkifyProvenanceEvidence,
): boolean {
  return evaluateChonkifyPromotionGate(evidence).verdict === "repo_pack_eligible";
}

export function chonkifyLicenseMetadataForRepoPack(): string {
  return canActivateChonkifyRepoPack() ? "MIT" : "NOASSERTION";
}

export type SwitchboardPackCompactionEvidence = ChonkifyProvenanceEvidence;
export type SwitchboardPackCompactionVerdict = ChonkifyPromotionVerdict;
export const evaluateSwitchboardPackCompactionGate = evaluateChonkifyPromotionGate;
export const canActivateSwitchboardPackCompaction = canActivateChonkifyRepoPack;
export const switchboardPackCompactionLicenseMetadata = chonkifyLicenseMetadataForRepoPack;
