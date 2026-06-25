import fs from "node:fs";
import path from "node:path";

const strict = process.argv.includes("--strict");

const requiredCommands = [
  {
    name: "node",
    hint: "Install Node.js, then run npm install.",
  },
  {
    name: "npm",
    hint: "Install Node.js/npm, then run npm install.",
  },
  {
    name: "npx",
    hint: "Install npm, which provides npx for the Tauri CLI.",
  },
  {
    name: "cargo",
    hint: "Install Rust with rustup so fmt:desktop and test:desktop can run.",
  },
  {
    name: "rustup",
    hint: "Install Rust with rustup and add macOS targets before universal builds.",
  },
  {
    name: "xcodebuild",
    hint: "Install Xcode or Command Line Tools and run xcode-select --install.",
  },
  {
    name: "codesign",
    hint: "codesign ships with Apple's developer tools.",
  },
  {
    name: "xcrun",
    hint: "xcrun is required for notarization tooling.",
  },
];

const requiredReleaseEnv = [
  {
    name: "HEADROOM_ACCOUNT_API_BASE_URL",
    hint: "Set the deployed account API base URL for packaged sign-in.",
  },
  {
    name: "APPLE_SIGNING_IDENTITY",
    hint: "Set your Developer ID Application certificate name.",
  },
  {
    name: "TAURI_SIGNING_PRIVATE_KEY",
    hint: "Set the private updater signing key contents.",
  },
  {
    name: "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
    hint: "Set the updater signing key password.",
  },
];

const recommendedReleaseEnv = [
  {
    name: "HEADROOM_UPDATER_PUBLIC_KEY",
    hint: "Recommended for updater-enabled release builds.",
  },
  {
    name: "HEADROOM_UPDATER_ENDPOINTS",
    hint: "Recommended so installed apps can find latest.json.",
  },
];

function hasCommand(command) {
  const pathEntries = (process.env.PATH ?? "").split(path.delimiter).filter(Boolean);
  return pathEntries.some((entry) => {
    try {
      fs.accessSync(path.join(entry, command), fs.constants.X_OK);
      return true;
    } catch {
      return false;
    }
  });
}

function hasEnv(name) {
  return Boolean(process.env[name]?.trim());
}

function hasNotarizationMode() {
  const apiMode =
    hasEnv("APPLE_API_ISSUER") &&
    hasEnv("APPLE_API_KEY") &&
    (hasEnv("APPLE_API_KEY_PATH") || hasEnv("APPLE_API_PRIVATE_KEY_P8"));
  const appleIdMode =
    hasEnv("APPLE_ID") && hasEnv("APPLE_PASSWORD") && hasEnv("APPLE_TEAM_ID");
  return apiMode || appleIdMode;
}

function repoFileExists(path) {
  return fs.existsSync(path);
}

const missingCommands = requiredCommands.filter((entry) => !hasCommand(entry.name));
const missingEnv = requiredReleaseEnv.filter((entry) => !hasEnv(entry.name));
const missingRecommendedEnv = recommendedReleaseEnv.filter((entry) => !hasEnv(entry.name));
const missingFiles = [
  "src-tauri/tauri.conf.json",
  "src-tauri/Cargo.toml",
  "package-lock.json",
].filter((path) => !repoFileExists(path));
const notarizationConfigured = hasNotarizationMode();

const blockers = [
  ...missingCommands.map((entry) => ({
    label: `missing command: ${entry.name}`,
    hint: entry.hint,
  })),
  ...missingFiles.map((path) => ({
    label: `missing release file: ${path}`,
    hint: "Restore the release configuration file before building a DMG.",
  })),
  ...missingEnv.map((entry) => ({
    label: `missing environment: ${entry.name}`,
    hint: entry.hint,
  })),
  ...(notarizationConfigured
    ? []
    : [
        {
          label: "missing notarization credentials",
          hint:
            "Set App Store Connect API credentials or APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID.",
        },
      ]),
];

const warnings = missingRecommendedEnv.map((entry) => ({
  label: `recommended environment missing: ${entry.name}`,
  hint: entry.hint,
}));

if (blockers.length === 0) {
  console.log("Release environment preflight passed.");
} else {
  console.log(
    strict
      ? "Release environment preflight found blocking issues:"
      : "Release environment preflight found issues:",
  );
  for (const blocker of blockers) {
    console.log(`- ${blocker.label}`);
    console.log(`  ${blocker.hint}`);
  }
}

if (warnings.length > 0) {
  console.log("Recommended release settings:");
  for (const warning of warnings) {
    console.log(`- ${warning.label}`);
    console.log(`  ${warning.hint}`);
  }
}

if (strict && blockers.length > 0) {
  process.exit(1);
}
