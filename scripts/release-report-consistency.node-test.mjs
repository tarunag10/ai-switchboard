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
    shareableDmgGate: {
      ready: false,
      environmentClear: false,
      signedAndNotarized: false,
      updaterFeedReady: true,
      backendValidationReady: true,
      staticSmokePreflightReady: false,
      installedAppSmokeReady: false,
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
    shareableDmgGate: {
      ready: true,
      environmentClear: true,
      signedAndNotarized: false,
      updaterFeedReady: true,
      backendValidationReady: true,
      staticSmokePreflightReady: true,
      installedAppSmokeReady: true,
    },
  });
  assert.match(errors.join("\n"), /staticSmokePreflight/);
  assert.match(errors.join("\n"), /installedSmoke/);
  assert.match(errors.join("\n"), /shareableDmgGate/);
});

test("requires measured savings readiness to include fresh evidence", () => {
  const base = {
    shareableDmgGate: {
      ready: false,
      environmentClear: false,
      signedAndNotarized: false,
      updaterFeedReady: true,
      backendValidationReady: true,
      staticSmokePreflightReady: false,
      installedAppSmokeReady: false,
    },
    localValidation: {
      measuredSavingsBenchmark: {
        totals: { savedTokens: 100 },
        passed: true,
        freshness: { fresh: false, generatedAt: "2020-01-01T00:00:00Z" },
      },
    },
  };
  expectConsistencyError(base, /measuredSavingsBenchmark/);

  const fresh = structuredClone(base);
  fresh.localValidation.measuredSavingsBenchmark.freshness = {
    fresh: true,
    generatedAt: new Date().toISOString(),
  };
  assert.deepEqual(validateReleaseReportConsistency(fresh), []);
});

function expectConsistencyError(report, pattern) {
  assert.match(validateReleaseReportConsistency(report).join("\n"), pattern);
}
