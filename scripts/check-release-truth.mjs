import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const readJson = (relativePath) => JSON.parse(fs.readFileSync(path.join(root, relativePath), "utf8"));
const truth = readJson("docs/release-truth.json");
const packageJson = readJson("package.json");
const tauri = readJson("src-tauri/tauri.conf.json");
const failures = [];

if (packageJson.version !== truth.appVersion) failures.push(`package.json version ${packageJson.version} != release truth ${truth.appVersion}`);
if (tauri.version !== truth.appVersion) failures.push(`tauri.conf.json version ${tauri.version} != release truth ${truth.appVersion}`);
if (truth.publicRelease.status === "verified" && truth.publicRelease.tag === `v${truth.appVersion}`) {
  failures.push("public release tag must not be treated as the current app version without an explicit release update");
}
for (const [name, status] of Object.entries(truth.evidence)) {
  if (!["verified", "documented", "unverified", "blocked"].includes(status)) failures.push(`invalid evidence status for ${name}: ${status}`);
}

const readme = fs.readFileSync(path.join(root, "README.md"), "utf8");
if (/current `main` branch/i.test(readme)) failures.push("README must not describe an arbitrary checkout as the current main branch");
if (/Until signed DMGs are published/i.test(readme)) failures.push("README contains a stale unpublished-DMG claim");
if (/signed DMGs will be published in the future/i.test(readme)) failures.push("README contains a stale future-publication claim");

if (failures.length) {
  console.error(failures.join("\n"));
  process.exitCode = 1;
} else {
  console.log(`release truth ok: app ${truth.appVersion}, public release ${truth.publicRelease.tag} (${truth.publicRelease.status})`);
}
