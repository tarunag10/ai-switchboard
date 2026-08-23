export function actionForBlocker(blocker) {
  if (/missing command: cargo|missing command: rustup/.test(blocker.label)) {
    return { label: "Install Rust toolchain", command: "rustup --version && cargo --version && rustup target add aarch64-apple-darwin x86_64-apple-darwin", detail: "Then run npm run fmt:desktop and npm run test:desktop." };
  }
  if (/missing command: xcodebuild|missing command: codesign|missing command: xcrun/.test(blocker.label)) {
    return { label: "Install Apple developer tools", command: "xcode-select --install", detail: "Then rerun npm run release:ready." };
  }
  if (/missing environment: APPLE_SIGNING_IDENTITY/.test(blocker.label)) {
    return { label: "Set Developer ID identity", command: "security find-identity -v -p codesigning", detail: "Export APPLE_SIGNING_IDENTITY to the Developer ID Application certificate name." };
  }
  if (/TAURI_SIGNING_PRIVATE_KEY/.test(blocker.label)) {
    return { label: "Set updater signing key", command: "export TAURI_SIGNING_PRIVATE_KEY=... && export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=...", detail: "Use the private updater signing key only in your local release shell or CI secret store." };
  }
  if (/missing notarization credentials/.test(blocker.label)) {
    return { label: "Set notarization credentials", command: "export APPLE_API_ISSUER=... APPLE_API_KEY=... APPLE_API_KEY_PATH=...", detail: "Apple ID app-specific password mode also works if APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID are set." };
  }
  return { label: blocker.label, command: "npm run release:env", detail: blocker.hint };
}

export function installedSmokeActions(report) {
  const actions = [];
  if (!report.installedSmoke.installedAppPresent) {
    actions.push({ label: "Install signed DMG", command: "npm run build:mac:dmg", detail: "Install the signed/notarized DMG into /Applications/AI Switchboard.app." });
  }
  if (!report.installedSmoke.evidenceReady) {
    actions.push({ label: "Record installed smoke evidence", command: "npm run smoke:installed -- --confirm", detail: `Missing evidence: ${report.installedSmoke.missingEvidence.join(", ") || "installed smoke summary"}.` });
  }
  return actions;
}

export function buildReleaseReadinessActions(report) {
  const actions = [
    ...report.releaseEnv.blockers.map(actionForBlocker),
    ...(!report.backendValidation.ready ? [{ label: "Run backend validation", command: report.backendValidation.unblockCommands.join(" && "), detail: report.backendValidation.message }] : []),
    ...installedSmokeActions(report),
  ];
  return actions.filter((action, index, allActions) => {
    const key = `${action.label}\n${action.command}`;
    return allActions.findIndex((candidate) => `${candidate.label}\n${candidate.command}` === key) === index;
  });
}

export function validateReleaseReadinessReport(report) {
  const failures = [];
  if (!report || typeof report !== "object" || Array.isArray(report)) return ["report must be an object"];
  if (typeof report.status !== "string" || report.status.trim() === "") failures.push("status must be a non-empty string");
  if (!report.releaseEnv || !Array.isArray(report.releaseEnv.blockers)) failures.push("releaseEnv.blockers must be an array");
  if (report.releaseEnv && Array.isArray(report.releaseEnv.blockers)) {
    report.releaseEnv.blockers.forEach((blocker, index) => {
      if (!blocker || typeof blocker !== "object" || Array.isArray(blocker)) {
        failures.push(`releaseEnv.blockers[${index}] must be an object`);
        return;
      }
      if (typeof blocker.label !== "string" || blocker.label.trim() === "") {
        failures.push(`releaseEnv.blockers[${index}].label must be a non-empty string`);
      }
      if (typeof blocker.hint !== "string") {
        failures.push(`releaseEnv.blockers[${index}].hint must be a string`);
      }
    });
  }
  if (!report.backendValidation || typeof report.backendValidation.ready !== "boolean") failures.push("backendValidation.ready must be boolean");
  if (report.backendValidation && !report.backendValidation.ready) {
    if (!Array.isArray(report.backendValidation.unblockCommands)) failures.push("backendValidation.unblockCommands must be an array when not ready");
    if (typeof report.backendValidation.message !== "string") failures.push("backendValidation.message must be a string when not ready");
  }
  if (!report.installedSmoke || typeof report.installedSmoke.installedAppPresent !== "boolean" || typeof report.installedSmoke.evidenceReady !== "boolean") {
    failures.push("installedSmoke presence and evidenceReady flags must be boolean");
  } else if (!Array.isArray(report.installedSmoke.missingEvidence)) {
    failures.push("installedSmoke.missingEvidence must be an array");
  }
  return failures;
}
