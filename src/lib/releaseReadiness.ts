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

export type ReleaseReadinessStatusTone = "ready" | "blocked" | "local-only";

export interface ReleaseReadinessStatusRow {
  id: string;
  label: string;
  statusLabel: string;
  tone: ReleaseReadinessStatusTone;
  source: string;
  detail: string;
}

export interface ReleaseReadinessReportSnapshot {
  generatedAt?: string;
  status: "ready" | "blocked" | string;
  backendValidation?: {
    ready?: boolean;
    message?: string;
  };
  staticSmokePreflight?: {
    ready?: boolean;
    message?: string;
  };
  installedSmoke?: {
    ready?: boolean;
    installedAppPresent?: boolean;
    evidenceReady?: boolean;
    missingEvidence?: string[];
    message?: string;
  };
  shareableDmgGate?: {
    ready?: boolean;
    signedAndNotarized?: boolean;
    updaterFeedReady?: boolean;
    staticSmokePreflightReady?: boolean;
    installedAppSmokeReady?: boolean;
    message?: string;
  };
  releaseEnv?: {
    ok?: boolean;
    blockers?: Array<{ label?: string }>;
    warnings?: Array<{ label?: string }>;
  };
}

function yesNo(value: boolean | undefined) {
  return value === true ? "yes" : "no";
}

function labels(items: Array<{ label?: string }> | undefined) {
  return items?.map((item) => item.label).filter(Boolean).join(", ") || "none";
}

export const releaseReadinessCommand = "npm run release:ready";

export const releaseReadinessStatusRows: ReleaseReadinessStatusRow[] = [
  {
    id: "frontend-build",
    label: "Frontend build",
    statusLabel: "Scripted",
    tone: "ready",
    source: "npm run build",
    detail: "TypeScript and Vite production build are the frontend gate.",
  },
  {
    id: "desktop-tests",
    label: "Desktop tests",
    statusLabel: "Required",
    tone: "blocked",
    source: "npm run fmt:desktop && npm run test:desktop",
    detail:
      "Rust formatting and desktop tests must run locally or in CI before public release.",
  },
  {
    id: "local-dmg",
    label: "Local DMG",
    statusLabel: "Local only",
    tone: "local-only",
    source: "npm run build:mac:local-install",
    detail:
      "Ad-hoc local install evidence is useful for testing but does not prove signed release readiness.",
  },
  {
    id: "installed-smoke",
    label: "Installed smoke",
    statusLabel: "Evidence required",
    tone: "blocked",
    source: "npm run smoke:installed -- --confirm",
    detail:
      "Installed-app smoke evidence must be recorded after running the beta checklist against the installed app.",
  },
  {
    id: "signing-env",
    label: "Signing environment",
    statusLabel: "Blocked until secrets",
    tone: "blocked",
    source: "npm run release:env",
    detail:
      "Developer ID and updater signing secrets are release blockers when missing, not app failures.",
  },
  {
    id: "notarization-env",
    label: "Notarization",
    statusLabel: "Blocked until credentials",
    tone: "blocked",
    source: "npm run release:env",
    detail:
      "Apple notarization credentials are required before sharing a public signed DMG.",
  },
  {
    id: "updater-config",
    label: "Updater configuration",
    statusLabel: "Blocked until feed",
    tone: "blocked",
    source: "HEADROOM_UPDATER_PUBLIC_KEY + HEADROOM_UPDATER_ENDPOINTS",
    detail:
      "Updater public key and feed endpoints must be configured for release builds.",
  },
  {
    id: "final-gate",
    label: "Final release gate",
    statusLabel: "Run report",
    tone: "blocked",
    source: "npm run release:ready -- --strict",
    detail:
      "The strict release readiness report is the source of truth before sharing a DMG.",
  },
];

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

export function releaseReadinessStatusCounts(
  rows: ReleaseReadinessStatusRow[] = releaseReadinessStatusRows,
) {
  return rows.reduce(
    (counts, row) => ({
      ...counts,
      [row.tone]: counts[row.tone] + 1,
    }),
    { ready: 0, blocked: 0, "local-only": 0 } satisfies Record<
      ReleaseReadinessStatusTone,
      number
    >,
  );
}

function statusTone(ready: boolean): ReleaseReadinessStatusTone {
  return ready ? "ready" : "blocked";
}

function statusLabel(ready: boolean) {
  return ready ? "Ready" : "Blocked";
}

export function releaseReadinessRowsFromReport(
  report: ReleaseReadinessReportSnapshot | null | undefined,
): ReleaseReadinessStatusRow[] {
  if (!report) {
    return releaseReadinessStatusRows;
  }

  const frontendReady = report.shareableDmgGate?.staticSmokePreflightReady === true;
  const desktopReady = report.backendValidation?.ready === true;
  const installedAppPresent = report.installedSmoke?.installedAppPresent === true;
  const installedSmokeReady = report.installedSmoke?.evidenceReady === true;
  const signingReady = report.shareableDmgGate?.signedAndNotarized === true;
  const notarizationReady = signingReady;
  const updaterReady = report.shareableDmgGate?.updaterFeedReady === true;
  const finalReady = report.status === "ready" && report.shareableDmgGate?.ready === true;
  const missingEvidence = report.installedSmoke?.missingEvidence ?? [];
  const releaseBlockers = report.releaseEnv?.blockers ?? [];

  return [
    {
      ...releaseReadinessStatusRows[0],
      statusLabel: statusLabel(frontendReady),
      tone: statusTone(frontendReady),
      detail: frontendReady
        ? "Static smoke preflight is ready in the release report."
        : "Run the frontend build and smoke preflight before publishing.",
    },
    {
      ...releaseReadinessStatusRows[1],
      statusLabel: statusLabel(desktopReady),
      tone: statusTone(desktopReady),
      detail: desktopReady
        ? "Rust formatting and desktop validation are runnable for this release."
        : "Rust backend validation is still blocked in the release report.",
    },
    {
      ...releaseReadinessStatusRows[2],
      statusLabel: installedAppPresent ? "Installed locally" : "Local only",
      tone: "local-only",
      detail: installedAppPresent
        ? "A local installed app exists, but local evidence is separate from signed release readiness."
        : releaseReadinessStatusRows[2].detail,
    },
    {
      ...releaseReadinessStatusRows[3],
      statusLabel: statusLabel(installedSmokeReady),
      tone: statusTone(installedSmokeReady),
      detail: installedSmokeReady
        ? "Installed-app smoke evidence is complete for the current checklist."
        : `Installed smoke evidence missing: ${
            missingEvidence.length ? missingEvidence.join(", ") : "installed smoke summary"
          }.`,
    },
    {
      ...releaseReadinessStatusRows[4],
      statusLabel: statusLabel(signingReady),
      tone: statusTone(signingReady),
      detail: signingReady
        ? "Signing and updater secrets are present according to the release report."
        : `Signing remains blocked${
            releaseBlockers.length ? `: ${releaseBlockers[0].label ?? "release blocker"}` : "."
          }`,
    },
    {
      ...releaseReadinessStatusRows[5],
      statusLabel: statusLabel(notarizationReady),
      tone: statusTone(notarizationReady),
      detail: notarizationReady
        ? "Notarization credentials are ready for signed release."
        : "Notarization credentials are still missing or unproven.",
    },
    {
      ...releaseReadinessStatusRows[6],
      statusLabel: statusLabel(updaterReady),
      tone: statusTone(updaterReady),
      detail: updaterReady
        ? "Updater public key and feed endpoint are configured."
        : "Updater feed configuration is missing or incomplete.",
    },
    {
      ...releaseReadinessStatusRows[7],
      statusLabel: finalReady ? "Ready" : "Blocked",
      tone: statusTone(finalReady),
      detail:
        report.shareableDmgGate?.message ??
        "The strict release readiness report is still the source of truth.",
    },
  ];
}

export function formatReleaseReadinessReportSnapshot(
  report: ReleaseReadinessReportSnapshot,
  reportPath: string,
) {
  return [
    "# Mac AI Switchboard Release Readiness",
    "",
    `Source: ${reportPath}`,
    `Generated: ${report.generatedAt ?? "unknown"}`,
    `Status: ${report.status}`,
    "",
    "## Gates",
    `- Backend validation ready: ${yesNo(report.backendValidation?.ready)}`,
    `- Static smoke preflight ready: ${yesNo(report.staticSmokePreflight?.ready)}`,
    `- Installed app present: ${yesNo(report.installedSmoke?.installedAppPresent)}`,
    `- Installed smoke ready: ${yesNo(report.installedSmoke?.ready ?? report.installedSmoke?.evidenceReady)}`,
    `- Signed and notarized: ${yesNo(report.shareableDmgGate?.signedAndNotarized)}`,
    `- Updater feed ready: ${yesNo(report.shareableDmgGate?.updaterFeedReady)}`,
    `- Shareable DMG ready: ${yesNo(report.shareableDmgGate?.ready)}`,
    "",
    "## Blockers",
    `- Environment blockers: ${labels(report.releaseEnv?.blockers)}`,
    `- Environment warnings: ${labels(report.releaseEnv?.warnings)}`,
    `- Missing installed smoke evidence: ${
      report.installedSmoke?.missingEvidence?.join(", ") || "none"
    }`,
    "",
    "## Next Step",
    report.shareableDmgGate?.message ??
      "Run npm run release:ready -- --strict before sharing a DMG.",
  ].join("\n");
}
