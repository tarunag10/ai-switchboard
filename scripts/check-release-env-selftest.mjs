import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

const releaseEnvScript = "scripts/check-release-env.mjs";

function runReleaseEnv(extraEnv) {
  const result = spawnSync(process.execPath, [releaseEnvScript, "--json"], {
    cwd: process.cwd(),
    encoding: "utf8",
    env: {
      ...process.env,
      ...extraEnv,
    },
  });

  assert.equal(
    result.status,
    0,
    `${releaseEnvScript} should not exit non-zero without --strict\n${result.stderr}`,
  );
  assert.equal(result.stderr, "", `${releaseEnvScript} wrote to stderr`);

  try {
    return JSON.parse(result.stdout);
  } catch (error) {
    assert.fail(
      `${releaseEnvScript} did not emit valid JSON: ${error.message}\n${result.stdout}`,
    );
  }
}

function labelsFor(entries) {
  return entries.map((entry) => entry.label);
}

function assertHasLabel(labels, expected) {
  assert.ok(
    labels.includes(expected),
    `Expected label "${expected}" in:\n${labels.join("\n")}`,
  );
}

function assertNoLabelContaining(labels, fragment) {
  assert.ok(
    !labels.some((label) => label.includes(fragment)),
    `Did not expect a label containing "${fragment}" in:\n${labels.join("\n")}`,
  );
}

const placeholderResult = runReleaseEnv({
  HEADROOM_ACCOUNT_API_BASE_URL: "REPLACE_WITH_ACCOUNT_API_URL",
  APPLE_SIGNING_IDENTITY: "your-developer-id-application",
  TAURI_SIGNING_PRIVATE_KEY: "/absolute/path/to/private.key",
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "your-updater-password",
  HEADROOM_UPDATER_PUBLIC_KEY: "REPLACE_WITH_UPDATER_PUBLIC_KEY",
  HEADROOM_UPDATER_ENDPOINTS: "https://example.com/latest.json",
  APPLE_API_ISSUER: "your-issuer-id",
  APPLE_API_KEY: "REPLACE_WITH_APPLE_API_KEY_ID",
  APPLE_API_PRIVATE_KEY_P8: "REPLACE_WITH_PRIVATE_KEY_CONTENTS",
});

assert.equal(placeholderResult.ok, false);

const placeholderBlockers = labelsFor(placeholderResult.blockers);
assertHasLabel(
  placeholderBlockers,
  "placeholder environment: HEADROOM_ACCOUNT_API_BASE_URL",
);
assertHasLabel(placeholderBlockers, "placeholder environment: APPLE_SIGNING_IDENTITY");
assertHasLabel(
  placeholderBlockers,
  "placeholder environment: TAURI_SIGNING_PRIVATE_KEY",
);
assertHasLabel(
  placeholderBlockers,
  "placeholder environment: TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
);
assertHasLabel(placeholderBlockers, "missing notarization credentials");

const placeholderWarnings = labelsFor(placeholderResult.warnings);
assertHasLabel(
  placeholderWarnings,
  "recommended environment is placeholder: HEADROOM_UPDATER_PUBLIC_KEY",
);
assertNoLabelContaining(
  placeholderWarnings,
  "recommended environment missing: HEADROOM_UPDATER_ENDPOINTS",
);

const validLookingResult = runReleaseEnv({
  HEADROOM_ACCOUNT_API_BASE_URL: "https://accounts.example.com",
  APPLE_SIGNING_IDENTITY: "Developer ID Application: Example Inc (ABCDE12345)",
  TAURI_SIGNING_PRIVATE_KEY:
    "-----BEGIN PRIVATE KEY-----\\nexample\\n-----END PRIVATE KEY-----",
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "example-password",
  HEADROOM_UPDATER_PUBLIC_KEY: "example-public-key",
  HEADROOM_UPDATER_ENDPOINTS: "https://updates.example.com/latest.json",
  APPLE_API_ISSUER: "11111111-2222-3333-4444-555555555555",
  APPLE_API_KEY: "ABCDEF1234",
  APPLE_API_PRIVATE_KEY_P8:
    "-----BEGIN PRIVATE KEY-----\\nexample\\n-----END PRIVATE KEY-----",
});

const validLookingBlockers = labelsFor(validLookingResult.blockers);
assertNoLabelContaining(validLookingBlockers, "placeholder environment:");
assertNoLabelContaining(validLookingBlockers, "missing environment:");
assertNoLabelContaining(validLookingBlockers, "missing notarization credentials");

const validLookingWarnings = labelsFor(validLookingResult.warnings);
assertNoLabelContaining(validLookingWarnings, "recommended environment placeholder:");
assertNoLabelContaining(validLookingWarnings, "recommended environment is placeholder:");
assertNoLabelContaining(validLookingWarnings, "recommended environment missing:");

console.log("Release environment placeholder self-test passed.");
