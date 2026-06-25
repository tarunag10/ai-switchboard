export interface ReleaseReadinessItem {
  id: string;
  label: string;
  detail: string;
}

export interface ReleaseReadinessGroup {
  id: string;
  title: string;
  items: ReleaseReadinessItem[];
}

export const releaseReadinessCommand = "npm run release:report";

export const releaseReadinessGroups: ReleaseReadinessGroup[] = [
  {
    id: "environment",
    title: "Environment",
    items: [
      {
        id: "rust",
        label: "Rust toolchain",
        detail: "cargo and rustup must be available for Tauri tests, formatting, and DMG builds.",
      },
      {
        id: "xcode",
        label: "Apple tools",
        detail: "xcodebuild, codesign, and xcrun are required for signed macOS packaging.",
      },
      {
        id: "account-api",
        label: "Account API URL",
        detail: "HEADROOM_ACCOUNT_API_BASE_URL must point packaged sign-in at the deployed account service.",
      },
    ],
  },
  {
    id: "signing",
    title: "Signing",
    items: [
      {
        id: "developer-id",
        label: "Developer ID",
        detail: "APPLE_SIGNING_IDENTITY must identify the Developer ID Application certificate.",
      },
      {
        id: "updater-key",
        label: "Updater signing key",
        detail: "TAURI_SIGNING_PRIVATE_KEY and password must be present for update metadata.",
      },
      {
        id: "notarization",
        label: "Notarization",
        detail: "Use App Store Connect API credentials or Apple ID credentials before publishing.",
      },
    ],
  },
  {
    id: "smoke",
    title: "Installed App Smoke",
    items: [
      {
        id: "dmg-install",
        label: "Install signed DMG",
        detail: "Install the built DMG into /Applications before the final smoke run.",
      },
      {
        id: "beta-smoke",
        label: "Run beta smoke test",
        detail: "Follow docs/beta-smoke-test.md against the installed app, including Switchboard, Doctor, and planned connectors.",
      },
      {
        id: "release-report",
        label: "Archive readiness report",
        detail: "Keep dist/release-readiness-report.md with the release artifacts for handoff.",
      },
    ],
  },
];

export function releaseReadinessItemCount() {
  return releaseReadinessGroups.reduce(
    (count, group) => count + group.items.length,
    0,
  );
}
