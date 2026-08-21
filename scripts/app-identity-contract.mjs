import fs from "node:fs";

export const canonicalInstalledAppPath = "/Applications/AI Switchboard.app";

export function readCanonicalAppIdentity(
  configPath = "src-tauri/tauri.conf.json",
) {
  const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
  return {
    productName: config.productName,
    bundleIdentifier: config.identifier,
    version: config.version,
  };
}

export function validateAppIdentity(metadata, expected) {
  const failures = [];
  if (!metadata || typeof metadata !== "object") return ["metadata must be an object"];
  if (metadata.bundleIdentifier !== expected.bundleIdentifier) {
    failures.push("bundle identifier does not match the canonical app identity");
  }
  if (metadata.version !== expected.version) {
    failures.push("version does not match the canonical app identity");
  }
  if (metadata.displayName !== expected.productName && metadata.bundleName !== expected.productName) {
    failures.push("display or bundle name does not match the canonical app identity");
  }
  return failures;
}
