export const shareableDmgGateBooleanFields = [
  "environmentClear",
  "signedAndNotarized",
  "updaterFeedReady",
  "backendValidationReady",
  "staticSmokePreflightReady",
  "installedAppSmokeReady",
];

export function isShareableDmgGateReady(gate) {
  return shareableDmgGateBooleanFields.every((field) => gate?.[field] === true);
}

export function publicReleaseGateBlockers(gate) {
  return [
    gate?.environmentClear ? null : "release environment",
    gate?.signedAndNotarized ? null : "signed/notarized DMG",
    gate?.updaterFeedReady ? null : "updater feed",
    gate?.backendValidationReady ? null : "backend validation",
    gate?.staticSmokePreflightReady ? null : "static smoke preflight",
    gate?.installedAppSmokeReady ? null : "public installed-app smoke",
  ].filter(Boolean);
}
