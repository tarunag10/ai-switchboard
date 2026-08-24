import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const read = (path) => fs.readFileSync(path, "utf8");

test("Tauri stages and nests the helper without registering a sidecar", () => {
  const config = JSON.parse(read("src-tauri/tauri.conf.json"));
  assert.equal(config.build.beforeBuildCommand, "npm run build");
  assert.equal(
    config.build.beforeBundleCommand,
    "./scripts/prepare-codex-probe-helper-app.sh",
  );
  assert.equal(
    config.bundle.macOS.files["Helpers/AI Switchboard Codex Probe.app"],
    "target/codex-probe-helper-bundle/AI Switchboard Codex Probe.app",
  );
  assert.equal(config.bundle.externalBin, undefined);
});

test("helper preparation is target-bound, sandbox-signed, and never launches the helper", () => {
  const script = read("scripts/prepare-codex-probe-helper-app.sh");
  for (const signal of [
    "TAURI_ENV_TARGET_TRIPLE",
    "cargo build",
    "--locked",
    "--release",
    "--target",
    "lipo -create",
    "APPLE_SIGNING_IDENTITY",
    "AI_SWITCHBOARD_CODESIGN_KEYCHAIN",
    "--entitlements",
    "--options runtime",
    "verify-codex-probe-helper-app.sh",
  ]) {
    assert.ok(script.includes(signal), `missing preparation signal: ${signal}`);
  }
  assert.ok(!script.includes("eval "));
  assert.ok(!script.includes('open "${STAGED_APP}"'));
  assert.ok(!script.includes('"${TEMP_EXECUTABLE}" --'));
  assert.ok(!script.includes("codex --version"));
});

test("bundle verification requires the exact nested identity and sandbox-only entitlement", () => {
  const script = read("scripts/verify-codex-probe-helper-app.sh");
  for (const signal of [
    "com.tarunagarwal.mac-ai-switchboard.codex-probe",
    "LSBackgroundOnly",
    "LSMinimumSystemVersion",
    "Mach-O",
    "lipo -archs",
    "otool -L",
    "LC_RPATH",
    "codesign --verify --strict",
    "com.apple.security.app-sandbox",
    "keys.length !== 1",
  ]) {
    assert.ok(script.includes(signal), `missing verification signal: ${signal}`);
  }
});

test("local installation signs nested code inside-out instead of using blanket deep signing", () => {
  const script = read("scripts/build-install-local-dmg.sh");
  assert.ok(!script.includes("codesign --force --deep --sign"));
  assert.ok(script.includes("verify-codex-probe-helper-app.sh"));
  assert.ok(script.includes("validate_app_destination"));
  assert.ok(script.includes('"${parent}" != "/Applications"'));
  assert.ok(script.includes("Refusing to replace a symlinked local app destination"));
  assert.ok(script.includes("com.tarunagarwal.mac-ai-switchboard"));
  assert.equal(script.match(/validate_app_destination "\$\{APP_DEST\}"/g)?.length, 2);
  assert.ok(
    script.includes(
      'validate_app_destination "${APP_DEST}"\nrm -rf -- "${APP_DEST:?}"',
    ),
  );
  assert.ok(
    script.lastIndexOf('validate_app_destination "${APP_DEST}"') <
      script.indexOf('rm -rf -- "${APP_DEST:?}"'),
  );
  assert.ok(
    script.indexOf('--entitlements "${HELPER_ENTITLEMENTS}"') <
      script.indexOf('--entitlements "${PARENT_ENTITLEMENTS}"'),
  );
});

test("release workflows import before build and always clean the ephemeral signing keychain", () => {
  for (const path of [
    ".github/workflows/release-macos.yml",
    ".github/workflows/release-macos-staging.yml",
  ]) {
    const workflow = read(path);
    const importAt = workflow.indexOf("./scripts/import-macos-signing-certificate.sh");
    const buildAt = workflow.indexOf("uses: tauri-apps/tauri-action");
    const verifyAt = workflow.indexOf("Verify packaged Codex probe helper");
    const cleanupAt = workflow.indexOf("./scripts/cleanup-macos-signing-keychain.sh");
    const publishAt = workflow.indexOf("Publish verified release draft");
    assert.ok(importAt >= 0, `${path} does not import the signing certificate`);
    assert.ok(importAt < buildAt, `${path} imports the certificate too late`);
    assert.ok(buildAt < verifyAt, `${path} verifies before packaging`);
    assert.ok(cleanupAt > buildAt, `${path} cleans the keychain too early`);
    assert.ok(cleanupAt < publishAt, `${path} publishes before keychain cleanup`);
    assert.ok(workflow.includes("if: always()"));
    assert.ok(workflow.includes("releaseDraft: true"));
    assert.ok(
      !workflow.slice(buildAt, verifyAt).includes("APPLE_CERTIFICATE"),
      `${path} asks Tauri to re-import the certificate`,
    );
    assert.ok(
      !workflow.slice(buildAt, verifyAt).includes("APPLE_SIGNING_IDENTITY:"),
      `${path} overwrites the imported identity fingerprint`,
    );
  }
});

test("CI certificate material stays in a validated ephemeral root", () => {
  const importer = read("scripts/import-macos-signing-certificate.sh");
  for (const signal of [
    "umask 077",
    "RUNNER_TEMP",
    "mktemp -d",
    "APPLE_CERTIFICATE",
    "APPLE_CERTIFICATE_PASSWORD",
    "APPLE_SIGNING_IDENTITY",
    "-f pkcs12",
    "-t agg",
    "AI_SWITCHBOARD_CODESIGN_KEYCHAIN",
    "AI_SWITCHBOARD_SIGNING_KEYCHAIN_ROOT",
    "IMPORTED_IDENTITY_SHA1",
    "security default-keychain -d user",
    "ai-switchboard-signing-$(uuidgen).keychain-db",
  ]) {
    assert.ok(importer.includes(signal), `missing import safety signal: ${signal}`);
  }
  assert.ok(!importer.includes("set -x"));
  assert.ok(!importer.includes('echo "${APPLE_CERTIFICATE}"'));

  const cleanup = read("scripts/cleanup-macos-signing-keychain.sh");
  assert.ok(cleanup.includes('"${SIGNING_TEMP_BASE}"/ai-switchboard-signing.*'));
  assert.ok(cleanup.includes("EXPECTED_KEYCHAIN_DIRECTORY"));
  assert.ok(cleanup.includes("ai-switchboard-signing-*.keychain-db"));
  assert.ok(cleanup.includes("security delete-keychain"));
});

test("local signed builds pre-import p12 material and prevent Tauri duplicate imports", () => {
  const script = read("scripts/build-macos-dmg.sh");
  const importAt = script.lastIndexOf("import_signing_certificate_if_needed");
  const buildAt = script.indexOf("npx tauri build");
  assert.ok(importAt >= 0 && importAt < buildAt);
  assert.ok(script.includes("AI_SWITCHBOARD_SIGNING_ENV_FILE"));
  assert.ok(script.includes("cleanup-macos-signing-keychain.sh"));
  assert.ok(script.includes("unset APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD"));
  assert.ok(script.includes("AI_SWITCHBOARD_SIGNING_TEMP_BASE|APPLE_SIGNING_IDENTITY"));
  assert.ok(script.includes("skipped duplicate PKCS#12 import"));
});

test("packaging scripts are executable", () => {
  for (const path of [
    "scripts/prepare-codex-probe-helper-app.sh",
    "scripts/verify-codex-probe-helper-app.sh",
    "scripts/import-macos-signing-certificate.sh",
    "scripts/cleanup-macos-signing-keychain.sh",
  ]) {
    assert.notEqual(fs.statSync(path).mode & 0o111, 0, `${path} is not executable`);
  }
});
