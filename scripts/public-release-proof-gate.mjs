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
    gate?.backendValidationReady ? null : "backend validation",
    gate?.staticSmokePreflightReady ? null : "static smoke preflight",
    gate?.installedAppSmokeReady ? null : "public installed-app smoke",
  ].filter(Boolean);
}
