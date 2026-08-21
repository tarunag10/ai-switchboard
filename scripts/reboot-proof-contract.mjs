export function isRebootProofReady(proof) {
  const marker = proof?.rebootMarker;
  const armedBoot = marker?.armedBootTimeUnixSeconds;
  const recordedBoot = marker?.recordedBootTimeUnixSeconds;
  return (
    proof?.proofReady === true &&
    proof?.releaseGateEvidence === true &&
    proof?.destructive === false &&
    Array.isArray(proof?.blockers) &&
    proof.blockers.length === 0 &&
    proof?.trust?.ready === true &&
    marker?.matchesCurrentBoot === true &&
    marker?.installedAppTrustVerified === true &&
    typeof marker?.armPath === "string" &&
    marker.armPath.length > 0 &&
    Number.isFinite(armedBoot) &&
    Number.isFinite(recordedBoot) &&
    armedBoot !== recordedBoot
  );
}
