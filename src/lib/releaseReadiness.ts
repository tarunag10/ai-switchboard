export interface ReleaseReadinessItem {
  id: string;
  label: string;
  detail: string;
  command?: string;
}

export interface ReleaseReadinessGroup {
  id: string;
  title: string;
  items: ReleaseReadinessItem[];
}

export interface ReleaseShareableGate {
  id: string;
  label: string;
  detail: string;
}

export const releaseReadinessCommand = "npm run release:ready";

export const releaseShareableGates: ReleaseShareableGate[] = [
  {
    id: "environment-clear",
    label: "Environment clear",
    detail: "release:report has no environment blockers.",
  },
  {
    id: "backend-validation",
    label: "Backend validation",
    detail: "cargo and rustup are available so desktop checks can run.",
  },
  {
    id: "signed-notarized",
    label: "Signed and notarized",
    detail:
      "Developer ID, updater signing, and notarization credentials are configured.",
  },
  {
    id: "updater-feed",
    label: "Updater feed",
    detail: "HEADROOM_UPDATER_PUBLIC_KEY and HEADROOM_UPDATER_ENDPOINTS are set.",
  },
  {
    id: "static-smoke-preflight",
    label: "Static smoke preflight",
    detail:
      "smoke:preflight passes and writes dist/smoke-preflight-summary.md with planned connector safety evidence.",
  },
  {
    id: "installed-smoke",
    label: "Installed smoke",
    detail:
      "/Applications/Mac AI Switchboard.app exists, beta smoke passes, and npm run smoke:installed writes dist/installed-smoke-summary.md.",
  },
];

export const releaseReadinessGroups: ReleaseReadinessGroup[] = [
  {
    id: "environment",
    title: "Environment",
    items: [
      {
        id: "rust",
        label: "Rust toolchain",
        detail:
          "cargo and rustup must be available so release:report can prove backend validation is runnable.",
        command:
          "rustup --version && cargo --version && rustup target add aarch64-apple-darwin x86_64-apple-darwin",
      },
      {
        id: "xcode",
        label: "Apple tools",
        detail:
          "xcodebuild, codesign, and xcrun are required for signed macOS packaging.",
        command: "xcodebuild -version && codesign --version && xcrun --version",
      },
      {
        id: "account-api",
        label: "Account API URL",
        detail:
          "HEADROOM_ACCOUNT_API_BASE_URL must point to the packaged sign-in account service.",
        command:
          "export HEADROOM_ACCOUNT_API_BASE_URL=https://your-account-api.example.com/api/v1",
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
        detail:
          "APPLE_SIGNING_IDENTITY must identify the Developer ID Application certificate.",
        command: "security find-identity -v -p codesigning",
      },
      {
        id: "updater-key",
        label: "Updater signing key",
        detail:
          "TAURI_SIGNING_PRIVATE_KEY and password must be present for update metadata.",
        command:
          "export TAURI_SIGNING_PRIVATE_KEY=... TAURI_SIGNING_PRIVATE_KEY_PASSWORD=...",
      },
      {
        id: "notarization",
        label: "Notarization",
        detail:
          "Use App Store Connect API credentials or Apple ID credentials before publishing.",
        command:
          "export APPLE_API_ISSUER=... APPLE_API_KEY=... APPLE_API_KEY_PATH=...",
      },
    ],
  },
  {
    id: "smoke",
    title: "Smoke Evidence",
    items: [
      {
        id: "static-preflight",
        label: "Run smoke preflight",
        detail:
          "Run npm run smoke:preflight and keep dist/smoke-preflight-summary.md as release evidence, including planned connector automation gates and manual workflow.",
        command: "npm run smoke:preflight",
      },
      {
        id: "dmg-install",
        label: "Install signed DMG",
        detail:
          "Install the signed and notarized DMG into /Applications before the final smoke run.",
        command: "npm run build:mac:dmg",
      },
      {
        id: "beta-smoke",
        label: "Run beta smoke test",
        detail:
          "Follow docs/beta-smoke-test.md against the installed app, including planned connector evidence, automation gates, manual workflow, Repo Intelligence recipes, and per-tool agent handoffs; then run npm run smoke:installed to write dist/installed-smoke-summary.md.",
        command: "open docs/beta-smoke-test.md",
      },
      {
        id: "release-report",
        label: "Archive readiness report",
        detail: "Keep dist/release-readiness-report.md with release artifacts for handoff.",
        command: "npm run release:ready -- --strict",
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
