import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fixturePath = path.join(root, "connectors", "lifecycle-fixtures.json");
const manifestPath = path.join(root, "connectors", "manifest.json");
const testsPath = path.join(root, "src-tauri", "src", "client_adapters_tests.rs");
const outputPath = path.join(root, "docs", "connector-lifecycle-status.md");

const catalog = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const adapterTests = fs.readFileSync(testsPath, "utf8");
const failures = [];

const expectedStages = ["detect", "preview", "backup", "apply", "verify", "rollback", "off"];
if (JSON.stringify(catalog.requiredStages) !== JSON.stringify(expectedStages)) {
  failures.push(`requiredStages must be exactly ${expectedStages.join(", ")}`);
}

const manifestById = new Map(manifest.map((connector) => [connector.id, connector]));
const fixturesById = new Map();
for (const fixture of catalog.connectors ?? []) {
  if (fixturesById.has(fixture.id)) failures.push(`duplicate fixture id: ${fixture.id}`);
  fixturesById.set(fixture.id, fixture);
  if (!manifestById.has(fixture.id)) failures.push(`fixture missing from manifest: ${fixture.id}`);
  for (const stage of expectedStages) {
    const proof = fixture.stages?.[stage];
    if (proof != null && !adapterTests.includes(`fn ${proof}(`)) {
      failures.push(`${fixture.id}.${stage} references missing Rust test: ${proof}`);
    }
  }
}
for (const connector of manifest) {
  if (!fixturesById.has(connector.id)) failures.push(`manifest connector missing fixture: ${connector.id}`);
  const fixture = fixturesById.get(connector.id);
  const complete = expectedStages.every((stage) => typeof fixture?.stages?.[stage] === "string" && fixture.stages[stage].trim());
  if ((connector.support_status === "managed") !== complete) {
    failures.push(`${connector.id}: manifest status ${connector.support_status} does not match lifecycle proof ${complete ? "complete" : "incomplete"}`);
  }
}

const mark = (proof) => (typeof proof === "string" && proof.trim() ? "✓" : "—");
const lines = [
  "# Connector lifecycle status matrix",
  "",
  "> Generated from `connectors/lifecycle-fixtures.json` by `node scripts/check-connector-lifecycle-matrix.mjs --check`. Do not label a connector **Managed** unless every lifecycle stage has named fixture-test proof.",
  "",
  "| Connector | Detect | Preview | Backup | Apply | Verify | Rollback | Off | Fixture proof | UI status |",
  "|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|",
];
for (const connector of manifest) {
  const fixture = fixturesById.get(connector.id);
  const cells = expectedStages.map((stage) => mark(fixture?.stages?.[stage]));
  const complete = cells.every((cell) => cell === "✓");
  lines.push(`| ${connector.name} | ${cells.join(" | ")} | ${complete ? "Complete" : "Incomplete"} | ${complete ? "Managed" : "Planned"} |`);
}
lines.push(
  "",
  "## Evidence contract",
  "",
  "Each non-empty stage points to a compiled Rust test in `src-tauri/src/client_adapters_tests.rs`. The Rust connector-list path independently parses the same fixture catalog and fails closed to `Planned` when any required stage is absent. Cursor remains Planned because native apply, verify, rollback, and Off-mode fixture proof is intentionally absent.",
  "",
);
const generated = `${lines.join("\n")}`;

if (process.argv.includes("--write")) {
  fs.writeFileSync(outputPath, generated);
} else {
  const current = fs.existsSync(outputPath) ? fs.readFileSync(outputPath, "utf8") : "";
  if (current !== generated) failures.push("docs/connector-lifecycle-status.md is stale; run with --write and review the diff");
}

if (failures.length) {
  for (const failure of failures) console.error(`connector lifecycle check: ${failure}`);
  process.exit(1);
}
console.log(`connector lifecycle check passed (${manifest.length} connectors; ${manifest.filter((item) => item.support_status === "managed").length} managed)`);
