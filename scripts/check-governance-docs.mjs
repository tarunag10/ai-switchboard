import fs from "node:fs";

const requiredFiles = [
  "LICENSE",
  "NOTICE",
  "TRADEMARKS.md",
  "CONTRIBUTING.md",
  "SECURITY.md",
];

const read = (path) => fs.readFileSync(path, "utf8");
const failures = [];

for (const file of requiredFiles) {
  if (!fs.existsSync(file)) {
    failures.push(`Missing ${file}`);
  }
}

if (failures.length === 0) {
  const readme = read("README.md");
  for (const file of requiredFiles) {
    if (!readme.includes(file)) {
      failures.push(`README.md does not link ${file}`);
    }
  }

  const trademarks = read("TRADEMARKS.md");
  for (const phrase of [
    "bundle identifier",
    "signing identity",
    "update channel",
  ]) {
    if (!trademarks.includes(phrase)) {
      failures.push(`TRADEMARKS.md does not mention ${phrase}`);
    }
  }

  const contributing = read("CONTRIBUTING.md");
  if (!contributing.includes("MIT License")) {
    failures.push("CONTRIBUTING.md does not confirm MIT contribution terms");
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("Governance docs are present and linked.");
