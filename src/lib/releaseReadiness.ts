import {
  getPlannedConnectorReadinessContract,
  pendingPlannedConnectors,
} from "./plannedConnectors";

export interface ReleaseReadinessItem {
  id: string;
  label: string;
  detail: string;
  command?: string;
  executable?: boolean;
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

export interface ReleaseLocalEvidenceRow {
  id: string;
  label: string;
  passed: boolean;
  statusLabel: string;
  command: string;
  summaryPath: string;
  detail: string;
}

export interface ReleaseReadinessEvidenceSummary {
  totalRows: number;
  readyRows: number;
  blockedRows: number;
  localOnlyRows: number;
  publicGateReady: boolean;
  reportLoaded: boolean;
  copy: string;
}

export interface ReleaseReadinessNextAction {
  rowId: string;
  label: string;
  command: string;
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
    requiredEvidence?: string[];
    missingEvidence?: string[];
    evidenceReady?: boolean;
  };
  installedSmoke?: {
    ready?: boolean;
    installedAppPresent?: boolean;
    evidenceReady?: boolean;
    missingEvidence?: string[];
    message?: string;
  };
  localValidation?: {
    ready?: boolean;
    releaseGateEvidence?: boolean;
    message?: string;
    localInstalled?: {
      summaryPresent?: boolean;
      jsonPresent?: boolean;
      passed?: boolean;
      appPresent?: boolean;
      metadataMatches?: boolean;
      dmgVerified?: boolean;
      codesignVerified?: boolean;
      runtimeHealthChecked?: boolean;
      appListenerReady?: boolean;
      engineProxyReady?: boolean;
      requiredCommand?: string;
      summaryPath?: string;
    };
    modeRelaunch?: {
      summaryPresent?: boolean;
      jsonPresent?: boolean;
      passed?: boolean;
      modeCount?: number;
      offModeProxyDown?: boolean;
      rtkModeProxyDown?: boolean;
      restored?: boolean;
      requiredCommand?: string;
      summaryPath?: string;
    };
    rollback?: {
      summaryPresent?: boolean;
      jsonPresent?: boolean;
      passed?: boolean;
      stepCount?: number;
      relaunchSurvivalEvidence?: string | null;
      requiredCommand?: string;
      summaryPath?: string;
    };
    doctorRepair?: {
      summaryPresent?: boolean;
      jsonPresent?: boolean;
      passed?: boolean;
      stepCount?: number;
      requiredCommand?: string;
      summaryPath?: string;
    };
    uninstall?: {
      summaryPresent?: boolean;
      jsonPresent?: boolean;
      passed?: boolean;
      destructive?: boolean | null;
      stepCount?: number;
      requiredCommand?: string;
      summaryPath?: string;
    };
    repoIntelligence?: {
      summaryPresent?: boolean;
      jsonPresent?: boolean;
      passed?: boolean;
      readOnly?: boolean | null;
      modifiesRepository?: boolean | null;
      stepCount?: number;
      requiredCommand?: string;
      summaryPath?: string;
    };
    repoMemoryMcp?: {
      summaryPresent?: boolean;
      jsonPresent?: boolean;
      passed?: boolean;
      readOnly?: boolean | null;
      modifiesRepository?: boolean | null;
      relaunchSurvivalEvidence?: string | null;
      connectorBridgeRecipesVerified?: boolean | null;
      toolCount?: number;
      expectedToolsPresent?: boolean | null;
      requiredCommand?: string;
      summaryPath?: string;
    };
    localOnlyNetwork?: {
      summaryPresent?: boolean;
      jsonPresent?: boolean;
      passed?: boolean;
      localOnly?: boolean | null;
      appOwnedRemoteCallsBlocked?: boolean | null;
      coverage?: {
        guardSurfaces?: number;
        appOwnedRemoteServiceSurfaces?: number;
        providerTrafficSurfaces?: number;
        managedDownloadSurfaces?: number;
      } | null;
      stepCount?: number;
      requiredCommand?: string;
      summaryPath?: string;
    };
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
  return (
    items
      ?.map((item) => item.label)
      .filter(Boolean)
      .join(", ") || "none"
  );
}

function labelsMatching(
  items: Array<{ label?: string }> | undefined,
  pattern: RegExp,
) {
  const matches = items?.filter(
    (item) => item.label && pattern.test(item.label),
  );
  return labels(matches);
}

function hasConnectorConfigPlanEvidence(
  report: ReleaseReadinessReportSnapshot,
) {
  const evidence = report.staticSmokePreflight?.requiredEvidence ?? [];
  return (
    report.staticSmokePreflight?.ready === true &&
    evidence.includes("Managed connector config creation plan") &&
    evidence.includes("Connector readiness payload in agent handoffs")
  );
}

function formatConnectorReadinessSummary() {
  const contracts = pendingPlannedConnectors.map(
    getPlannedConnectorReadinessContract,
  );
  const automationReady = contracts.filter(
    (contract) => contract.automationEnabled,
  ).length;
  const nextBlocked = new Map<string, number>();
  for (const contract of contracts) {
    const stage =
      contract.stages.find((item) => item.id === contract.nextBlockedStage)
        ?.label ?? "None";
    nextBlocked.set(stage, (nextBlocked.get(stage) ?? 0) + 1);
  }

  return [
    "## Managed Connector Readiness",
    `- Managed connectors: ${contracts.length}`,
    `- Automation ready: ${automationReady}`,
    `- Next blocked gates: ${[...nextBlocked.entries()]
      .map(([label, count]) => `${label} (${count})`)
      .join(", ")}`,
    ...contracts
      .slice(0, 6)
      .map(
        (contract) =>
          `- ${contract.connectorName}: ${contract.setupPhase}, next gate ${
            contract.stages.find(
              (item) => item.id === contract.nextBlockedStage,
            )?.label ?? "None"
          }`,
      ),
    "- Full per-tool dossiers are available from Doctor's connector dossier copy action.",
  ].join("\n");
}

export const releaseReadinessCommand = "npm run release:ready";
export const localReleaseEvidenceCommand = "npm run evidence:local";

export const localReleaseEvidenceCommandIds = [
  "desktop-validation",
  "static-preflight",
  "local-dmg-build-install",
  "local-installed-smoke",
  "local-mode-relaunch-smoke",
  "rollback-center-validation",
  "doctor-repair-validation",
  "uninstall-validation",
  "repo-intelligence-validation",
  "repo-memory-mcp-validation",
  "local-only-network-validation",
  "release-report",
] as const;

export function formatLocalReleaseEvidenceSequenceCopy() {
  return [
    `Run local evidence: ${localReleaseEvidenceCommand}`,
    "This matches the in-app Run local evidence button and runs these steps in order:",
    "1. Desktop validation",
    "2. Static smoke preflight",
    "3. Local DMG build/install",
    "4. Local installed smoke",
    "5. Local Off/RTK relaunch smoke",
    "6. Rollback Center validation",
    "7. Doctor repair validation",
    "8. Uninstall dry-run validation",
    "9. Repo Intelligence validation",
    "10. Repo Memory MCP validation",
    "11. Local-only network validation",
    "12. Refresh release readiness report",
    "Summary: dist/local-evidence-summary.md",
    "Boundary: this local unsigned/ad-hoc evidence does not run signing, notarization, updater publication, or the strict public-release gate.",
  ].join("\n");
}

export function formatReleaseReadinessCommandCopy() {
  return [
    "Mac AI Switchboard release readiness command",
    `Refresh report: ${releaseReadinessCommand}`,
    "Strict public-release gate: npm run release:ready -- --strict",
    "Report source after running: dist/release-readiness-report.json",
    `One-command local evidence: ${localReleaseEvidenceCommand}`,
    "Local-only install evidence: npm run build:mac:local-install",
    "App shortcut: Run local evidence executes the same sequence as npm run evidence:local: desktop validation, smoke preflight, local DMG build/install, local installed smoke, local Off/RTK relaunch smoke, Rollback Center validation, Doctor repair validation, uninstall dry-run validation, Repo Intelligence validation, Repo Memory MCP validation, local-only network validation, and a release report refresh.",
    "Boundary: local unsigned/ad-hoc install evidence never replaces signed DMG install, notarization, updater feed, or installed smoke confirmation.",
  ].join("\n");
}

export function formatReleaseReadinessSourceLabel(
  reportPath: string | null | undefined,
) {
  return reportPath
    ? `Loaded report: ${reportPath}`
    : `No release report loaded. Run ${releaseReadinessCommand} to create dist/release-readiness-report.json; checklist defaults are guidance, not release proof.`;
}

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
    id: "local-doctor-rollback",
    label: "Local safety evidence",
    statusLabel: "Local only",
    tone: "local-only",
    source: localReleaseEvidenceCommand,
    detail:
      "The one-click local evidence flow runs desktop validation, preflight, local install/relaunch, Rollback Center, Doctor repair, uninstall dry-run, Repo Intelligence, Repo Memory MCP, local-only network validation, and a release report refresh; it does not replace signed installed-app smoke.",
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
    id: "connector-config-plan",
    label: "Connector config plan",
    statusLabel: "Evidence required",
    tone: "blocked",
    source: "npm run smoke:preflight",
    detail:
      "Managed connector smoke evidence must include the gated config creation plan and connector readiness payload before any future config writes.",
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
    detail:
      "HEADROOM_UPDATER_PUBLIC_KEY and HEADROOM_UPDATER_ENDPOINTS are set.",
  },
  {
    id: "static-smoke-preflight",
    label: "Static smoke preflight",
    detail:
      "smoke:preflight passes and writes dist/smoke-preflight-summary.md with managed connector safety evidence and managed connector readiness evidence.",
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
        id: "desktop-validation",
        label: "Run desktop validation",
        detail:
          "Run Rust formatting and desktop tests from the app before release evidence is refreshed.",
        command: "npm run fmt:desktop && npm run test:desktop",
        executable: true,
      },
      {
        id: "static-preflight",
        label: "Run smoke preflight",
        detail:
          "Run npm run smoke:preflight and keep dist/smoke-preflight-summary.md as release evidence, including managed connector evidence, managed connector automation gates, and native config gates for connector-specific writes.",
        command: "npm run smoke:preflight",
        executable: true,
      },
      {
        id: "dmg-install",
        label: "Install signed DMG",
        detail:
          "Install the signed and notarized DMG into /Applications before the final smoke run.",
        command: "npm run build:mac:dmg",
      },
      {
        id: "local-dmg-build-install",
        label: "Build/install local DMG",
        detail:
          "Build, install, ad-hoc sign, and verify the local unsigned DMG from the app; this produces local-only evidence and does not prove public release readiness.",
        command: "npm run build:mac:local-install",
        executable: true,
      },
      {
        id: "local-installed-smoke",
        label: "Record local install smoke",
        detail:
          "Record local unsigned/ad-hoc install evidence after a local DMG install; this remains local-only and does not prove public release readiness.",
        command: "npm run smoke:installed:local",
        executable: true,
      },
      {
        id: "local-mode-relaunch-smoke",
        label: "Record local mode relaunch smoke",
        detail:
          "Relaunch the installed local app in saved Off and RTK-only modes and confirm proxy listeners stay down; this remains local-only and does not prove public release readiness.",
        command: "npm run smoke:mode-relaunch:local -- --confirm",
        executable: true,
      },
      {
        id: "rollback-center-validation",
        label: "Validate Rollback Center",
        detail:
          "Run focused Rollback Center frontend and backend checks from the app evidence flow, then write local summary evidence.",
        command: "npm run smoke:rollback:local",
        executable: true,
      },
      {
        id: "doctor-repair-validation",
        label: "Validate Doctor repairs",
        detail:
          "Run focused Doctor UI/copy checks and the backend Off/RTK guard check, then write local summary evidence.",
        command: "npm run smoke:doctor-repair:local",
        executable: true,
      },
      {
        id: "uninstall-validation",
        label: "Validate uninstall dry-run",
        detail:
          "Run frontend disclosure and backend target-inventory checks for the uninstall dry-run without deleting files.",
        command: "npm run smoke:uninstall:local",
        executable: true,
      },
      {
        id: "repo-intelligence-validation",
        label: "Validate Repo Intelligence",
        detail:
          "Run Repo Intelligence pack/safety tests and backend read-only API tests without mutating the repository.",
        command: "npm run smoke:repo-intelligence:local",
        executable: true,
      },
      {
        id: "repo-memory-mcp-validation",
        label: "Validate Repo Memory MCP",
        detail:
          "Run the app-managed repo-memory MCP stdio smoke as its own local evidence row, including read-only tools, bounded pack response, seeded secret exclusion, and connector bridge recipe proof.",
        command: "npm run smoke:repo-memory-mcp:local",
        executable: true,
      },
      {
        id: "beta-smoke",
        label: "Run beta smoke test",
        detail:
          "Follow docs/beta-smoke-test.md against the installed app, including managed connector evidence, native config gates as a manual workflow, Repo Intelligence recipes, per-tool agent handoffs, and connector readiness payload; then run npm run smoke:installed to write dist/installed-smoke-summary.md.",
        command: "open docs/beta-smoke-test.md",
      },
      {
        id: "release-report",
        label: "Archive readiness report",
        detail:
          "Refresh dist/release-readiness-report.md and .json with the latest local evidence for handoff; this does not run the strict public-release gate.",
        command: "npm run release:report",
        executable: true,
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

export function releaseReadinessEvidenceSummary(
  rows: ReleaseReadinessStatusRow[] = releaseReadinessStatusRows,
  report: ReleaseReadinessReportSnapshot | null | undefined = null,
): ReleaseReadinessEvidenceSummary {
  const counts = releaseReadinessStatusCounts(rows);
  const publicGateReady =
    report?.status === "ready" && report.shareableDmgGate?.ready === true;
  const reportLoaded = Boolean(report);

  return {
    totalRows: rows.length,
    readyRows: counts.ready,
    blockedRows: counts.blocked,
    localOnlyRows: counts["local-only"],
    publicGateReady,
    reportLoaded,
    copy: reportLoaded
      ? `${counts.ready}/${rows.length} scripted release checks ready; ${counts.blocked} blocked; ${counts["local-only"]} local-only; public gate ${publicGateReady ? "ready" : "blocked"}.`
      : `${counts.ready}/${rows.length} checklist defaults ready; ${counts.blocked} blocked; ${counts["local-only"]} local-only; no release report loaded.`,
  };
}

export function releaseReadinessNextAction(
  rows: ReleaseReadinessStatusRow[] = releaseReadinessStatusRows,
): ReleaseReadinessNextAction | null {
  const blocked = rows.find((row) => row.tone === "blocked");
  if (!blocked) {
    return null;
  }

  return {
    rowId: blocked.id,
    label: blocked.label,
    command: blocked.source,
    detail: blocked.detail,
  };
}

export function formatReleaseReadinessNextAction(
  action: ReleaseReadinessNextAction | null,
) {
  if (!action) {
    return "Next release action: no blocked scripted rows. Run the strict release gate before sharing a DMG.";
  }

  return [
    `Next release action: ${action.label}`,
    `Command: ${action.command}`,
    `Why: ${action.detail}`,
  ].join("\n");
}

function statusTone(ready: boolean): ReleaseReadinessStatusTone {
  return ready ? "ready" : "blocked";
}

function statusLabel(ready: boolean) {
  return ready ? "Ready" : "Blocked";
}

function localEvidenceDetail(
  label: string,
  count: number | undefined,
  summaryPresent: boolean | undefined,
  extra: string | null = null,
  unit = "step",
) {
  const safeCount = count ?? 0;
  const parts = [
    `${label} ${safeCount} ${unit}${safeCount === 1 ? "" : "s"}.`,
    summaryPresent ? "Summary present." : "Summary missing.",
  ];
  if (extra) {
    parts.push(extra);
  }
  return parts.join(" ");
}

export function releaseLocalEvidenceRowsFromReport(
  report: ReleaseReadinessReportSnapshot | null | undefined,
): ReleaseLocalEvidenceRow[] {
  const local = report?.localValidation;
  if (!local) {
    return [];
  }

  return [
    {
      id: "local-installed",
      label: "Local installed app",
      passed: local.localInstalled?.passed === true,
      statusLabel:
        local.localInstalled?.passed === true ? "Passed" : "Missing",
      command:
        local.localInstalled?.requiredCommand ??
        "npm run smoke:installed:local",
      summaryPath:
        local.localInstalled?.summaryPath ??
        "dist/local-installed-smoke-summary.md",
      detail: localEvidenceDetail(
        "Local installed smoke checked",
        undefined,
        local.localInstalled?.summaryPresent,
        local.localInstalled?.runtimeHealthChecked === true
          ? local.localInstalled?.appListenerReady === true &&
            local.localInstalled?.engineProxyReady === true
            ? "App listener and Headroom engine proxy were ready."
            : "Runtime health was checked but one listener was not ready."
          : "Runtime health was not checked because the app was not running.",
      ),
    },
    {
      id: "mode-relaunch",
      label: "Mode relaunch",
      passed: local.modeRelaunch?.passed === true,
      statusLabel: local.modeRelaunch?.passed === true ? "Passed" : "Missing",
      command:
        local.modeRelaunch?.requiredCommand ??
        "npm run smoke:mode-relaunch:local -- --confirm",
      summaryPath:
        local.modeRelaunch?.summaryPath ??
        "dist/local-mode-relaunch-smoke-summary.md",
      detail: localEvidenceDetail(
        "Off and RTK relaunch smoke checked",
        local.modeRelaunch?.modeCount,
        local.modeRelaunch?.summaryPresent,
        local.modeRelaunch?.offModeProxyDown === true &&
          local.modeRelaunch?.rtkModeProxyDown === true &&
        local.modeRelaunch?.restored === true
          ? "Proxy listeners stayed down and config was restored."
          : "Proxy or restore evidence unproven.",
        "mode",
      ),
    },
    {
      id: "rollback",
      label: "Rollback Center",
      passed: local.rollback?.passed === true,
      statusLabel: local.rollback?.passed === true ? "Passed" : "Missing",
      command: local.rollback?.requiredCommand ?? "npm run smoke:rollback:local",
      summaryPath:
        local.rollback?.summaryPath ??
        "dist/local-rollback-validation-summary.md",
      detail: localEvidenceDetail(
        "Rollback validation checked",
        local.rollback?.stepCount,
        local.rollback?.summaryPresent,
      ),
    },
    {
      id: "doctor-repair",
      label: "Doctor repairs",
      passed: local.doctorRepair?.passed === true,
      statusLabel: local.doctorRepair?.passed === true ? "Passed" : "Missing",
      command:
        local.doctorRepair?.requiredCommand ??
        "npm run smoke:doctor-repair:local",
      summaryPath:
        local.doctorRepair?.summaryPath ??
        "dist/local-doctor-repair-validation-summary.md",
      detail: localEvidenceDetail(
        "Doctor repair validation checked",
        local.doctorRepair?.stepCount,
        local.doctorRepair?.summaryPresent,
      ),
    },
    {
      id: "uninstall",
      label: "Uninstall dry-run",
      passed: local.uninstall?.passed === true,
      statusLabel: local.uninstall?.passed === true ? "Passed" : "Missing",
      command:
        local.uninstall?.requiredCommand ?? "npm run smoke:uninstall:local",
      summaryPath:
        local.uninstall?.summaryPath ??
        "dist/local-uninstall-validation-summary.md",
      detail: localEvidenceDetail(
        "Uninstall validation checked",
        local.uninstall?.stepCount,
        local.uninstall?.summaryPresent,
        local.uninstall?.destructive === false
          ? "Dry-run only; no destructive actions."
          : "Destructive status unproven.",
      ),
    },
    {
      id: "repo-intelligence",
      label: "Repo Intelligence",
      passed: local.repoIntelligence?.passed === true,
      statusLabel:
        local.repoIntelligence?.passed === true ? "Passed" : "Missing",
      command:
        local.repoIntelligence?.requiredCommand ??
        "npm run smoke:repo-intelligence:local",
      summaryPath:
        local.repoIntelligence?.summaryPath ??
        "dist/local-repo-intelligence-validation-summary.md",
      detail: localEvidenceDetail(
        "Repo Intelligence validation checked",
        local.repoIntelligence?.stepCount,
        local.repoIntelligence?.summaryPresent,
        local.repoIntelligence?.readOnly === true &&
          local.repoIntelligence?.modifiesRepository === false
          ? "Read-only and non-mutating."
          : "Read-only boundary unproven.",
      ),
    },
    {
      id: "repo-memory-mcp",
      label: "Repo Memory MCP",
      passed: local.repoMemoryMcp?.passed === true,
      statusLabel:
        local.repoMemoryMcp?.passed === true ? "Passed" : "Missing",
      command:
        local.repoMemoryMcp?.requiredCommand ??
        "npm run smoke:repo-memory-mcp:local",
      summaryPath:
        local.repoMemoryMcp?.summaryPath ??
        "dist/local-repo-memory-mcp-validation-summary.md",
      detail: localEvidenceDetail(
        "Repo Memory MCP validation checked",
        local.repoMemoryMcp?.toolCount,
        local.repoMemoryMcp?.summaryPresent,
        local.repoMemoryMcp?.readOnly === true &&
          local.repoMemoryMcp?.modifiesRepository === false &&
          local.repoMemoryMcp?.expectedToolsPresent === true &&
          local.repoMemoryMcp?.connectorBridgeRecipesVerified === true
          ? "Read-only stdio tools, connector bridge recipes, and seeded secret exclusion passed."
          : "MCP read-only or bridge evidence unproven.",
        "tool",
      ),
    },
    {
      id: "local-only-network",
      label: "Local-only network",
      passed: local.localOnlyNetwork?.passed === true,
      statusLabel:
        local.localOnlyNetwork?.passed === true ? "Passed" : "Missing",
      command:
        local.localOnlyNetwork?.requiredCommand ??
        "npm run smoke:local-only:local",
      summaryPath:
        local.localOnlyNetwork?.summaryPath ??
        "dist/local-only-network-validation-summary.md",
      detail: localEvidenceDetail(
        "Local-only network validation checked",
        local.localOnlyNetwork?.stepCount,
        local.localOnlyNetwork?.summaryPresent,
        local.localOnlyNetwork?.localOnly === true &&
          local.localOnlyNetwork?.appOwnedRemoteCallsBlocked === true
          ? `Local-only guards and remote-service scan passed${
              local.localOnlyNetwork.coverage
                ? ` across ${local.localOnlyNetwork.coverage.guardSurfaces ?? "unknown"} guard surfaces, ${local.localOnlyNetwork.coverage.appOwnedRemoteServiceSurfaces ?? "unknown"} app-owned remote-service surfaces, ${local.localOnlyNetwork.coverage.providerTrafficSurfaces ?? "unknown"} provider-traffic surfaces, and ${local.localOnlyNetwork.coverage.managedDownloadSurfaces ?? "unknown"} managed-download surfaces.`
                : "."
            }`
          : "Local-only network boundary unproven.",
      ),
    },
  ];
}

export function releaseReadinessRowsFromReport(
  report: ReleaseReadinessReportSnapshot | null | undefined,
): ReleaseReadinessStatusRow[] {
  if (!report) {
    return releaseReadinessStatusRows;
  }

  const frontendReady =
    report.staticSmokePreflight?.evidenceReady ??
    report.shareableDmgGate?.staticSmokePreflightReady === true;
  const desktopReady = report.backendValidation?.ready === true;
  const installedAppPresent =
    report.installedSmoke?.installedAppPresent === true;
  const installedSmokeReady = report.installedSmoke?.evidenceReady === true;
  const localValidationReady = report.localValidation?.ready === true;
  const rollbackPassed = report.localValidation?.rollback?.passed === true;
  const doctorRepairPassed =
    report.localValidation?.doctorRepair?.passed === true;
  const localInstalledPassed =
    report.localValidation?.localInstalled?.passed === true;
  const uninstallPassed = report.localValidation?.uninstall?.passed === true;
  const repoIntelligencePassed =
    report.localValidation?.repoIntelligence?.passed === true;
  const repoMemoryMcpPassed =
    report.localValidation?.repoMemoryMcp?.passed === true;
  const localOnlyNetworkPassed =
    report.localValidation?.localOnlyNetwork?.passed === true;
  const signingReady = report.shareableDmgGate?.signedAndNotarized === true;
  const notarizationReady = signingReady;
  const updaterReady = report.shareableDmgGate?.updaterFeedReady === true;
  const connectorConfigPlanReady = hasConnectorConfigPlanEvidence(report);
  const finalReady =
    report.status === "ready" && report.shareableDmgGate?.ready === true;
  const missingEvidence = report.installedSmoke?.missingEvidence ?? [];
  const missingStaticEvidence =
    report.staticSmokePreflight?.missingEvidence ?? [];
  const releaseBlockers = report.releaseEnv?.blockers ?? [];
  const publicDmgBlockers = [
    signingReady ? null : "signed/notarized DMG",
    updaterReady ? null : "updater feed",
    frontendReady ? null : "static smoke preflight",
    installedSmokeReady ? null : "public installed-app smoke",
  ].filter(Boolean);

  return [
    {
      ...releaseReadinessStatusRows[0],
      statusLabel: statusLabel(frontendReady),
      tone: statusTone(frontendReady),
      detail: frontendReady
        ? "Static smoke preflight is ready in the release report."
        : missingStaticEvidence.length
          ? `Static smoke evidence missing: ${missingStaticEvidence.join(", ")}.`
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
            missingEvidence.length
              ? missingEvidence.join(", ")
              : "installed smoke summary"
          }.`,
    },
    {
      ...releaseReadinessStatusRows[4],
      statusLabel: localValidationReady ? "Validated locally" : "Local only",
      tone: "local-only",
      detail: localValidationReady
        ? "Local installed app, Rollback Center, Doctor repair, uninstall dry-run, Repo Intelligence, Repo Memory MCP, and local-only network validation summaries are passing; this remains local-only evidence."
        : `Local survival validation missing or stale: installed app ${
            localInstalledPassed ? "passed" : "not passed"
          }, Rollback ${
            rollbackPassed ? "passed" : "not passed"
          }, Doctor repair ${
            doctorRepairPassed ? "passed" : "not passed"
          }, uninstall ${
            uninstallPassed ? "passed" : "not passed"
          }, Repo Intelligence ${
            repoIntelligencePassed ? "passed" : "not passed"
          }, Repo Memory MCP ${
            repoMemoryMcpPassed ? "passed" : "not passed"
          }, local-only network ${
            localOnlyNetworkPassed ? "passed" : "not passed"
          }.`,
    },
    {
      ...releaseReadinessStatusRows[5],
      statusLabel: statusLabel(signingReady),
      tone: statusTone(signingReady),
      detail: signingReady
        ? "Signing and updater secrets are present according to the release report."
        : `Signing remains blocked${
            releaseBlockers.length
              ? `: ${releaseBlockers[0].label ?? "release blocker"}`
              : "."
          }`,
    },
    {
      ...releaseReadinessStatusRows[6],
      statusLabel: statusLabel(notarizationReady),
      tone: statusTone(notarizationReady),
      detail: notarizationReady
        ? "Notarization credentials are ready for signed release."
        : "Notarization credentials are still missing or unproven.",
    },
    {
      ...releaseReadinessStatusRows[7],
      statusLabel: statusLabel(updaterReady),
      tone: statusTone(updaterReady),
      detail: updaterReady
        ? "Updater public key and feed endpoint are configured."
        : "Updater feed configuration is missing or incomplete.",
    },
    {
      ...releaseReadinessStatusRows[8],
      statusLabel: statusLabel(connectorConfigPlanReady),
      tone: statusTone(connectorConfigPlanReady),
      detail: connectorConfigPlanReady
        ? "Static smoke evidence includes the managed connector config creation plan and connector readiness payload."
        : "Static smoke evidence must include the managed connector config creation plan and connector readiness payload.",
    },
    {
      ...releaseReadinessStatusRows[9],
      statusLabel: finalReady ? "Ready" : "Blocked",
      tone: statusTone(finalReady),
      detail:
        publicDmgBlockers.length > 0
          ? `Public DMG blocked until ${publicDmgBlockers.join(", ")} evidence is ready.`
          : (report.shareableDmgGate?.message ??
            "The strict release readiness report is still the source of truth."),
    },
  ];
}

export function formatReleaseReadinessReportSnapshot(
  report: ReleaseReadinessReportSnapshot,
  reportPath: string,
) {
  const statusRows = releaseReadinessRowsFromReport(report);
  const evidenceSummary = releaseReadinessEvidenceSummary(statusRows, report);
  const nextAction = releaseReadinessNextAction(statusRows);
  const localInstalled = report.installedSmoke?.installedAppPresent === true;
  const publicReady =
    report.shareableDmgGate?.ready === true && report.status === "ready";
  const localValidation = report.localValidation;
  const missingLocalEvidence = [
    localValidation?.localInstalled?.passed ? null : "local installed smoke",
    localValidation?.modeRelaunch?.passed ? null : "Off/RTK relaunch smoke",
    localValidation?.rollback?.passed ? null : "Rollback Center validation",
    localValidation?.rollback?.relaunchSurvivalEvidence
      ? null
      : "Rollback Center relaunch survival evidence",
    localValidation?.doctorRepair?.passed ? null : "Doctor repair validation",
    localValidation?.uninstall?.passed ? null : "uninstall dry-run validation",
    localValidation?.repoIntelligence?.passed ? null : "Repo Intelligence validation",
    localValidation?.repoMemoryMcp?.passed ? null : "Repo Memory MCP validation",
    localValidation?.localOnlyNetwork?.passed ? null : "local-only network validation",
  ].filter(Boolean);

  return [
    "# Mac AI Switchboard Release Readiness",
    "",
    `Source: ${reportPath}`,
    `Generated: ${report.generatedAt ?? "unknown"}`,
    `Status: ${report.status}`,
    `Evidence summary: ${evidenceSummary.copy}`,
    formatReleaseReadinessNextAction(nextAction),
    "",
    "## Commands",
    `- Refresh report: ${releaseReadinessCommand}`,
    "- Strict public-release gate: npm run release:ready -- --strict",
    "- Local-only install evidence: npm run build:mac:local-install",
    "",
    "## Gates",
    `- Backend validation ready: ${yesNo(report.backendValidation?.ready)}`,
    `- Static smoke preflight ready: ${yesNo(report.staticSmokePreflight?.ready)}`,
    `- Connector config plan evidence: ${yesNo(hasConnectorConfigPlanEvidence(report))}`,
    `- Installed app present: ${yesNo(report.installedSmoke?.installedAppPresent)}`,
    `- Installed smoke ready: ${yesNo(report.installedSmoke?.ready ?? report.installedSmoke?.evidenceReady)}`,
    `- Local safety validation ready: ${yesNo(report.localValidation?.ready)}`,
    `- Signed and notarized: ${yesNo(report.shareableDmgGate?.signedAndNotarized)}`,
    `- Updater feed ready: ${yesNo(report.shareableDmgGate?.updaterFeedReady)}`,
    `- Shareable DMG ready: ${yesNo(report.shareableDmgGate?.ready)}`,
    "",
    "## Evidence Boundary",
    `- Public signed/notarized release proof: ${publicReady ? "ready" : "not ready"}`,
    `- Local unsigned/ad-hoc install evidence: ${localInstalled ? "present, local-only" : "not present"}`,
    "- Local unsigned/ad-hoc evidence never replaces signed DMG install, notarization, updater feed, or installed smoke confirmation.",
    `- Sharing status: ${publicReady ? "shareable" : "blocked until strict release gate is ready"}`,
    "",
    "## Status Rows",
    ...statusRows.map(
      (row) =>
        `- ${row.label}: ${row.statusLabel} (${row.tone}) via ${row.source}. ${row.detail}`,
    ),
    "",
    formatConnectorReadinessSummary(),
    "",
    "## Blockers",
    `- Environment blockers: ${labels(report.releaseEnv?.blockers)}`,
    `- Environment warnings: ${labels(report.releaseEnv?.warnings)}`,
    `- Signing blockers: ${labelsMatching(
      report.releaseEnv?.blockers,
      /APPLE_SIGNING|TAURI_SIGNING|Developer ID|signing/i,
    )}`,
    `- Notarization blockers: ${labelsMatching(
      report.releaseEnv?.blockers,
      /notarization|APPLE_API|APPLE_ID|App Store Connect/i,
    )}`,
    `- Updater blockers: ${labelsMatching(
      report.releaseEnv?.blockers,
      /HEADROOM_UPDATER|updater/i,
    )}`,
    "- Missing signing, notarization, or updater secrets are release blockers, not app failures.",
    `- Missing installed smoke evidence: ${
      report.installedSmoke?.missingEvidence?.join(", ") || "none"
    }`,
    `- Missing local evidence: ${
      missingLocalEvidence.length ? missingLocalEvidence.join(", ") : "none"
    }`,
    "",
    "## Next Step",
    report.shareableDmgGate?.message ??
      "Run npm run release:ready -- --strict before sharing a DMG.",
  ].join("\n");
}
