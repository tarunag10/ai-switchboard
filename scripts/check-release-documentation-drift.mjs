#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const files = [
  "docs/ai-switchboard-rebrand-release-evidence.md",
  "docs/plan-status-ledger.md",
];
const forbidden = [
  /Verified live release\s*:/i,
  /Public release `v0\.0\.0` has a verified/i,
];
const scopedWords = /(historical|documented|unverified|current release-truth)/i;
const failures = [];

for (const relative of files) {
  const absolute = path.join(root, relative);
  const contents = fs.readFileSync(absolute, "utf8");
  for (const pattern of forbidden) {
    const match = contents.match(pattern);
    if (match && !scopedWords.test(contents.slice(Math.max(0, match.index - 180), match.index + 240))) {
      failures.push(`${relative}: unscoped historical release claim: ${match[0]}`);
    }
  }
}

if (failures.length > 0) {
  console.error("release documentation drift check failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(JSON.stringify({ ok: true, files, rule: "historical release evidence must be explicitly scoped" }, null, 2));
