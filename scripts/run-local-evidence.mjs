#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";

const steps = [
  {
    id: "desktop-validation",
    label: "Desktop validation",
    command: "npm run fmt:desktop && npm run test:desktop",
    commands: [
      ["npm", ["run", "fmt:desktop"]],
      ["npm", ["run", "test:desktop"]],
    ],
  },
  {
    id: "static-preflight",
    label: "Static smoke preflight",
    command: "npm run smoke:preflight",
    commands: [["npm", ["run", "smoke:preflight"]]],
    summaryPath: "dist/smoke-preflight-summary.md",
  },
  {
    id: "local-dmg-build-install",
    label: "Local DMG build/install",
    command: "npm run build:mac:local-install",
    commands: [["npm", ["run", "build:mac:local-install"]]],
    summaryPath: "dist/local-installed-smoke-summary.md",
  },
  {
    id: "local-installed-smoke",
    label: "Local installed smoke",
    command: "npm run smoke:installed:local",
    commands: [["npm", ["run", "smoke:installed:local"]]],
    summaryPath: "dist/local-installed-smoke-summary.md",
  },
  {
    id: "local-mode-relaunch-smoke",
    label: "Local Off/RTK relaunch smoke",
    command: "npm run smoke:mode-relaunch:local -- --confirm",
    commands: [["npm", ["run", "smoke:mode-relaunch:local", "--", "--confirm"]]],
    summaryPath: "dist/local-mode-relaunch-smoke-summary.md",
  },
  {
    id: "rollback-center-validation",
    label: "Rollback Center validation",
    command: "npm run smoke:rollback:local",
    commands: [["npm", ["run", "smoke:rollback:local"]]],
    summaryPath: "dist/local-rollback-validation-summary.md",
  },
  {
    id: "doctor-repair-validation",
    label: "Doctor repair validation",
    command: "npm run smoke:doctor-repair:local",
    commands: [["npm", ["run", "smoke:doctor-repair:local"]]],
    summaryPath: "dist/local-doctor-repair-validation-summary.md",
  },
  {
    id: "uninstall-validation",
    label: "Uninstall dry-run validation",
    command: "npm run smoke:uninstall:local",
    commands: [["npm", ["run", "smoke:uninstall:local"]]],
    summaryPath: "dist/local-uninstall-validation-summary.md",
  },
  {
    id: "repo-intelligence-validation",
    label: "Repo Intelligence validation",
    command: "npm run smoke:repo-intelligence:local",
    commands: [["npm", ["run", "smoke:repo-intelligence:local"]]],
    summaryPath: "dist/local-repo-intelligence-validation-summary.md",
  },
  {
    id: "repo-memory-mcp-validation",
    label: "Repo Memory MCP validation",
    command: "npm run smoke:repo-memory-mcp:local",
    commands: [["npm", ["run", "smoke:repo-memory-mcp:local"]]],
    summaryPath: "dist/local-repo-memory-mcp-validation-summary.md",
  },
  {
    id: "local-only-network-validation",
    label: "Local-only network validation",
    command: "npm run smoke:local-only:local",
    commands: [["npm", ["run", "smoke:local-only:local"]]],
    summaryPath: "dist/local-only-network-validation-summary.md",
  },
  {
    id: "release-report",
    label: "Refresh release readiness report",
    command: "npm run release:report",
    commands: [["npm", ["run", "release:report"]]],
    summaryPath: "dist/release-readiness-report.md",
  },
];

const verifyOnly = process.argv.includes("--verify");
const skipInstall = process.argv.includes("--skip-install");
const selectedSteps = skipInstall
  ? steps.filter((step) => step.id !== "local-dmg-build-install")
  : steps;

function ensureDist() {
  fs.mkdirSync("dist", { recursive: true });
}

function renderSummary(results, status) {
  const lines = [
    "# Local Evidence Summary",
    "",
    `- Status: ${status}`,
    "- Release gate evidence: no",
    "- Boundary: local unsigned/ad-hoc evidence does not run signing, notarization, updater publication, or the strict public-release gate.",
    "",
    "## Steps",
    "",
    ...selectedSteps.map((step) => {
      const result = results.find((item) => item.id === step.id);
      const marker = result?.ok ? "pass" : result ? "fail" : "pending";
      const summary = step.summaryPath ? ` Summary: ${step.summaryPath}.` : "";
      return `- ${marker}: ${step.label} (${step.command}).${summary}`;
    }),
  ];
  return `${lines.join("\n")}\n`;
}

if (verifyOnly) {
  ensureDist();
  fs.writeFileSync(
    "dist/local-evidence-summary.md",
    renderSummary([], "verified command graph only"),
  );
  console.log(
    `Local evidence command graph verified: ${selectedSteps.length} steps.`,
  );
  process.exit(0);
}

ensureDist();
const results = [];

for (const step of selectedSteps) {
  console.log(`\n==> ${step.label}`);
  for (const [program, args] of step.commands) {
    console.log(`$ ${program} ${args.join(" ")}`);
    const result = spawnSync(program, args, {
      cwd: process.cwd(),
      env: process.env,
      stdio: "inherit",
    });
    if (result.status !== 0) {
      results.push({ id: step.id, ok: false });
      fs.writeFileSync(
        "dist/local-evidence-summary.md",
        renderSummary(results, `failed at ${step.label}`),
      );
      process.exit(result.status ?? 1);
    }
  }
  results.push({ id: step.id, ok: true });
}

fs.writeFileSync(
  "dist/local-evidence-summary.md",
  renderSummary(results, "passed"),
);
console.log("\nLocal evidence sequence completed.");
console.log("Summary: dist/local-evidence-summary.md");
