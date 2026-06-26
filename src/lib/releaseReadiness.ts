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

export interface ReleaseShareableGate {
  id: string;
  label: string;
  detail: string;
}

export const releaseReadinessCommand = "npm run release:report";

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
    detail: "Developer ID, updater signing, and notarization credentials are configured.",
  },
  {
    id: "updater-feed",
    label: "Updater feed",
    detail: "HEADROOM_UPDATER_PUBLIC_KEY and HEADROOM_UPDATER_ENDPOINTS are set.",
  },
{
id: "static-smoke-preflight",
label: "Static smoke preflight",
detail: "smoke:preflight passes and writes dist/smoke-preflight-summary.md.",
},
  {
    id: "installed-smoke",
    label: "Installed smoke",
    detail: "/Applications/Mac AI Switchboard.app passes beta smoke.",
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
      detail: "cargo and rustup must be available so release:report can prove backend validation is runnable.",
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
    title: "Smoke Evidence",
    items: [
{
id: "static-preflight",
label: "Run smoke preflight",
detail: "Run npm run smoke:preflight and keep dist/smoke-preflight-summary.md with release evidence.",
},
    {
      id: "dmg-install",
      label: "Install signed DMG",
      detail: "Install the signed and notarized DMG into /Applications before the final smoke run.",
    },
    {
      id: "beta-smoke",
      label: "Run beta smoke test",
detail: "Follow docs/beta-smoke-test.md against installed app including planned connector evidence, Repo Intelligence recipes, and per-tool agent handoffs.",
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
