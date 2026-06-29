import fs from "node:fs";

const publicDocFiles = [
  "LICENSE",
  "NOTICE",
  "TRADEMARKS.md",
  "CONTRIBUTING.md",
  "SECURITY.md",
  "GOVERNANCE.md",
  "MAINTAINERS.md",
  "PRIVACY.md",
  "TERMS.md",
  "CODE_OF_CONDUCT.md",
  "SUPPORT.md",
  "docs/remote-destinations.md",
  "docs/repository-settings.md",
  "docs/connectors.md",
];

const requiredFiles = [
  ...publicDocFiles,
  ".github/CODEOWNERS",
  ".github/pull_request_template.md",
  ".github/ISSUE_TEMPLATE/bug_report.md",
  ".github/ISSUE_TEMPLATE/feature_request.md",
  ".github/ISSUE_TEMPLATE/security_report.md",
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
  for (const file of publicDocFiles) {
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
  if (!contributing.includes("@tarunag10")) {
    failures.push(
      "CONTRIBUTING.md does not name the maintainer approval owner",
    );
  }

  const governance = read("GOVERNANCE.md");
  for (const phrase of [
    "No pull request",
    "Tarun Agarwal",
    "explicitly approved",
    "Require review from Code Owners",
  ]) {
    if (!governance.includes(phrase)) {
      failures.push(`GOVERNANCE.md does not mention ${phrase}`);
    }
  }

  const maintainers = read("MAINTAINERS.md");
  if (!maintainers.includes("@tarunag10")) {
    failures.push("MAINTAINERS.md does not list @tarunag10");
  }

  const codeowners = read(".github/CODEOWNERS");
  if (!codeowners.includes("* @tarunag10")) {
    failures.push(".github/CODEOWNERS does not require @tarunag10 ownership");
  }

  const privacy = read("PRIVACY.md");
  for (const phrase of ["Local-First", "Secrets", "Remote Services"]) {
    if (!privacy.includes(phrase)) {
      failures.push(`PRIVACY.md does not mention ${phrase}`);
    }
  }

  const remoteDestinations = read("docs/remote-destinations.md");
  for (const phrase of [
    "Local-Only Boundary",
    "App-Owned Remote Destinations",
    "HEADROOM_UPDATER_ENDPOINTS",
    "HEADROOM_SENTRY_DSN",
    "VITE_SENTRY_DSN",
    "HEADROOM_APTABASE_APP_KEY",
    "VITE_CLARITY_PROJECT_ID",
    "GitHub Issues",
    "Provider Traffic",
    "Change Control",
  ]) {
    if (!remoteDestinations.includes(phrase)) {
      failures.push(`docs/remote-destinations.md does not mention ${phrase}`);
    }
  }

  const connectors = read("docs/connectors.md");
  for (const phrase of [
    "Status Labels",
    "Support Matrix",
    "Managed",
    "Guided",
    "Detected",
    "Planned",
    "Limited managed adapter",
    "Claude Code",
    "Codex",
    "Cursor",
    "Windsurf",
    "OpenCode",
    "Automation Gates",
  ]) {
    if (!connectors.includes(phrase)) {
      failures.push(`docs/connectors.md does not mention ${phrase}`);
    }
  }

  const terms = read("TERMS.md");
  for (const phrase of ["MIT License", "No Warranty", "Maintainer Approval"]) {
    if (!terms.includes(phrase)) {
      failures.push(`TERMS.md does not mention ${phrase}`);
    }
  }

  const pullRequestTemplate = read(".github/pull_request_template.md");
  if (!pullRequestTemplate.includes("@tarunag10")) {
    failures.push(
      ".github/pull_request_template.md does not require @tarunag10 approval",
    );
  }

  const repositorySettings = read("docs/repository-settings.md");
  for (const phrase of [
    "Require review from Code Owners",
    "Allow only `@tarunag10`",
    "Enable private vulnerability reporting",
    "Enable secret scanning",
  ]) {
    if (!repositorySettings.includes(phrase)) {
      failures.push(`docs/repository-settings.md does not mention ${phrase}`);
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("Governance docs are present and linked.");
