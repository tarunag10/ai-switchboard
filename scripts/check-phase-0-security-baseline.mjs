import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fixture = JSON.parse(
  fs.readFileSync(path.join(root, "fixtures", "security-baseline-v1.json"), "utf8"),
);
const rustRoot = path.join(root, "src-tauri", "src");

function rustFiles(directory) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) return rustFiles(target);
    return entry.isFile() && entry.name.endsWith(".rs") ? [target] : [];
  });
}

const rustSource = rustFiles(rustRoot)
  .map((file) => fs.readFileSync(file, "utf8"))
  .join("\n");
const failures = [];
const ids = new Set();

if (fixture.version !== 1) failures.push("fixture version must be 1");
if (fixture.controls?.length !== 9) failures.push("exactly nine Phase 0 controls are required");
for (const control of fixture.controls ?? []) {
  if (ids.has(control.id)) failures.push(`duplicate control id: ${control.id}`);
  ids.add(control.id);
  if (!Array.isArray(control.tests) || control.tests.length === 0) {
    failures.push(`${control.id}: at least one focused Rust test is required`);
  }
  for (const test of control.tests ?? []) {
    if (!rustSource.includes(`fn ${test}(`)) failures.push(`${control.id}: missing Rust test ${test}`);
  }
  for (const evidence of control.sourceNeedles ?? []) {
    const file = path.join(root, evidence.file);
    if (!fs.existsSync(file) || !fs.readFileSync(file, "utf8").includes(evidence.needle)) {
      failures.push(`${control.id}: missing source evidence ${evidence.file}: ${evidence.needle}`);
    }
  }
}

if (failures.length) {
  for (const failure of failures) console.error(`security baseline: ${failure}`);
  process.exit(1);
}
console.log(`Phase 0 security baseline evidence passed (${fixture.controls.length} controls)`);
