import test from "node:test";
import assert from "node:assert/strict";
import { validateReleaseReportConsistency } from "./release-report-consistency.mjs";

test("accepts internally consistent blocked release evidence", () => {
  assert.deepEqual(validateReleaseReportConsistency({
    staticSmokePreflight: {
      ready: false,
      evidenceReady: false,
      smokeSummaryPresent: false,
      missingEvidence: ["summary"],
      freshness: { fresh: false, generatedAt: null },
    },
    installedSmokeSummary: { present: false },
    installedSmoke: {
      ready: false,
      installedAppPresent: false,
      bundleMetadataPresent: false,
      evidenceReady: false,
      smokeSummaryPresent: false,
      checklistSha256Matches: false,
      metadataMatches: false,
      missingEvidence: ["summary"],
      freshness: { fresh: false, generatedAt: null },
    },
  }), []);
});

test("rejects contradictory freshness, identity, and checklist combinations", () => {
  const errors = validateReleaseReportConsistency({
    staticSmokePreflight: {
      ready: true,
      evidenceReady: true,
      smokeSummaryPresent: false,
      missingEvidence: [],
      freshness: { fresh: false, generatedAt: null },
    },
    installedSmokeSummary: { present: false },
    installedSmoke: {
      ready: true,
      installedAppPresent: true,
      bundleMetadataPresent: true,
      evidenceReady: true,
      smokeSummaryPresent: true,
      checklistSha256Matches: true,
      metadataMatches: false,
      missingEvidence: [],
      freshness: { fresh: true, generatedAt: null },
      currentChecklistSha256: "a",
      recordedChecklistSha256: "b",
    },
  });
  assert.match(errors.join("\n"), /staticSmokePreflight/);
  assert.match(errors.join("\n"), /installedSmoke/);
});
