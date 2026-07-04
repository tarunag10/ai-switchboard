import { spawnSync } from "node:child_process";
import fs from "node:fs";

const strict = process.argv.includes("--strict");
const jsonOutput = process.argv.includes("--json");
const reportJsonPath = "dist/release-readiness-report.json";

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    stdio: jsonOutput ? "pipe" : "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
    throw new Error(`${command} ${args.join(" ")} failed${output ? `:\n${output}` : ""}`);
  }
}

function actionForBlocker(blocker) {
  if (/missing command: cargo|missing command: rustup/.test(blocker.label)) {
    return {
      label: "Install Rust toolchain",
      command: "rustup --version && cargo --version && rustup target add aarch64-apple-darwin x86_64-apple-darwin",
      detail: "Then run npm run fmt:desktop and npm run test:desktop.",
    };
  }
  if (/missing command: xcodebuild|missing command: codesign|missing command: xcrun/.test(blocker.label)) {
    return {
      label: "Install Apple developer tools",
      command: "xcode-select --install",
      detail: "Then rerun npm run release:ready.",
    };
  }
  if (/missing environment: APPLE_SIGNING_IDENTITY/.test(blocker.label)) {
    return {
      label: "Set Developer ID identity",
      command: "security find-identity -v -p codesigning",
      detail: "Export APPLE_SIGNING_IDENTITY to the Developer ID Application certificate name.",
    };
  }
  if (/TAURI_SIGNING_PRIVATE_KEY/.test(blocker.label)) {
    return {
      label: "Set updater signing key",
      command: "export TAURI_SIGNING_PRIVATE_KEY=... && export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=...",
      detail: "Use the private updater signing key only in your local release shell or CI secret store.",
    };
  }
  if (/missing notarization credentials/.test(blocker.label)) {
    return {
      label: "Set notarization credentials",
      command: "export APPLE_API_ISSUER=... APPLE_API_KEY=... APPLE_API_KEY_PATH=...",
      detail: "Apple ID app-specific password mode also works if APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID are set.",
    };
  }
  return {
    label: blocker.label,
    command: "npm run release:env",
    detail: blocker.hint,
  };
}

function installedSmokeActions(report) {
  const actions = [];
  if (!report.installedSmoke.installedAppPresent) {
    actions.push({
      label: "Install signed DMG",
      command: "npm run build:mac:dmg",
      detail: "Install the signed/notarized DMG into /Applications/Mac AI Switchboard.app.",
    });
  }
  if (!report.installedSmoke.evidenceReady) {
    actions.push({
      label: "Record installed smoke evidence",
      command: "npm run smoke:installed -- --confirm",
      detail: `Missing evidence: ${report.installedSmoke.missingEvidence.join(", ") || "installed smoke summary"}.`,
    });
  }
  return actions;
}

run("npm", ["run", "check:branding"]);
run("npm", ["run", "check:local-only-network"]);
run("npm", ["run", "release:report"]);
run("npm", ["run", "release:report:check"]);

const report = JSON.parse(fs.readFileSync(reportJsonPath, "utf8"));
const actions = [
  ...report.releaseEnv.blockers.map(actionForBlocker),
  ...(!report.backendValidation.ready
    ? [
        {
          label: "Run backend validation",
          command: report.backendValidation.unblockCommands.join(" && "),
          detail: report.backendValidation.message,
        },
      ]
    : []),
  ...installedSmokeActions(report),
].filter((action, index, allActions) => {
  const key = `${action.label}\n${action.command}`;
  return allActions.findIndex((candidate) => `${candidate.label}\n${candidate.command}` === key) === index;
});

if (jsonOutput) {
  console.log(
    JSON.stringify(
      {
        status: report.status,
        strict,
        reportPath: reportJsonPath,
        actions,
      },
      null,
      2,
    ),
  );
} else {
  console.log(`Release readiness: ${report.status}`);
  console.log(`Report: ${reportJsonPath}`);
  if (actions.length > 0) {
    console.log("Next actions:");
    for (const action of actions) {
      console.log(`- ${action.label}`);
      console.log(`  ${action.command}`);
      console.log(`  ${action.detail}`);
    }
  } else {
    console.log("No release blockers found.");
  }
}

if (strict && report.status !== "ready") {
  process.exit(1);
}
