export function validateReleaseReportConsistency(report) {
  const errors = [];
  const shareableGate = report?.shareableDmgGate;
  if (shareableGate && typeof shareableGate === "object") {
    const componentReady = [
      shareableGate.environmentClear,
      shareableGate.signedAndNotarized,
      shareableGate.updaterFeedReady,
      shareableGate.backendValidationReady,
      shareableGate.staticSmokePreflightReady,
      shareableGate.installedAppSmokeReady,
    ].every((value) => value === true);
    if (shareableGate.ready !== componentReady) {
      errors.push("shareableDmgGate.ready must match all component gates");
    }
  }
  const staticSmoke = report?.staticSmokePreflight;
  if (staticSmoke && typeof staticSmoke === "object") {
    if (staticSmoke.ready !== staticSmoke.evidenceReady) {
      errors.push("staticSmokePreflight.ready must match evidenceReady");
    }
    if (staticSmoke.evidenceReady && (
      staticSmoke.smokeSummaryPresent !== true
      || staticSmoke.missingEvidence?.length > 0
      || staticSmoke.freshness?.fresh !== true
    )) {
      errors.push("staticSmokePreflight.evidenceReady requires present, complete, fresh evidence");
    }
    if (staticSmoke.freshness?.fresh === true && (
      staticSmoke.smokeSummaryPresent !== true
      || typeof staticSmoke.freshness.generatedAt !== "string"
    )) {
      errors.push("staticSmokePreflight.freshness cannot be fresh without a timestamped summary");
    }
  }

  const installedSmoke = report?.installedSmoke;
  const installedSummary = report?.installedSmokeSummary;
  if (installedSmoke && typeof installedSmoke === "object") {
    if (installedSummary && installedSmoke.smokeSummaryPresent !== installedSummary.present) {
      errors.push("installedSmokeSummary.present must match installedSmoke.smokeSummaryPresent");
    }
    if (installedSmoke.ready !== (
      installedSmoke.installedAppPresent === true
      && installedSmoke.bundleMetadataPresent === true
      && installedSmoke.evidenceReady === true
    )) {
      errors.push("installedSmoke.ready must match app, metadata, and evidence readiness");
    }
    if (installedSmoke.evidenceReady && (
      installedSmoke.smokeSummaryPresent !== true
      || installedSmoke.checklistSha256Matches !== true
      || installedSmoke.metadataMatches !== true
      || installedSmoke.missingEvidence?.length > 0
      || installedSmoke.freshness?.fresh !== true
    )) {
      errors.push("installedSmoke.evidenceReady requires complete, fresh, identity-bound evidence");
    }
    if (installedSmoke.freshness?.fresh === true && (
      installedSmoke.smokeSummaryPresent !== true
      || typeof installedSmoke.freshness.generatedAt !== "string"
    )) {
      errors.push("installedSmoke.freshness cannot be fresh without a timestamped summary");
    }
    if (installedSmoke.checklistSha256Matches === true && (
      typeof installedSmoke.currentChecklistSha256 !== "string"
      || typeof installedSmoke.recordedChecklistSha256 !== "string"
      || installedSmoke.currentChecklistSha256 !== installedSmoke.recordedChecklistSha256
    )) {
      errors.push("installedSmoke.checklistSha256Matches requires equal recorded and current hashes");
    }
  }
  const measuredSavings = report?.localValidation?.measuredSavingsBenchmark;
  if (measuredSavings && typeof measuredSavings === "object") {
    const expectedPassed =
      Number(measuredSavings.totals?.savedTokens ?? 0) > 0 &&
      measuredSavings.freshness?.fresh === true;
    if (measuredSavings.passed !== expectedPassed) {
      errors.push("measuredSavingsBenchmark.passed must require positive saved tokens and fresh evidence");
    }
    if (measuredSavings.freshness?.fresh === true && typeof measuredSavings.freshness.generatedAt !== "string") {
      errors.push("measuredSavingsBenchmark.freshness cannot be fresh without a timestamp");
    }
  }
  return errors;
}
