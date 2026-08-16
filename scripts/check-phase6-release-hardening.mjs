#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

const root = process.cwd();
const evidenceDir = path.resolve(
  root,
  process.env.P6_RELEASE_EVIDENCE_DIR || "dist",
);
const outputPath = path.resolve(
  root,
  process.env.P6_RELEASE_AUDIT_OUTPUT ||
    "dist/phase6-release-hardening-audit.json",
);

function read(relative) {
  try {
    return fs.readFileSync(path.join(root, relative), "utf8");
  } catch {
    return null;
  }
}

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
}

const localContracts = [
  {
    id: "signed-installed-and-reboot-workflow",
    files: [
      "scripts/arm-reboot-level-installed-proof.mjs",
      "scripts/record-reboot-level-installed-proof-marker.mjs",
      "scripts/reboot-level-installed-proof-summary.mjs",
      "scripts/check-reboot-level-installed-proof-summary.mjs",
      "scripts/reboot-level-installed-proof.node-test.mjs",
    ],
    markers: ["recordedAfterManualReboot", "codesign", "stapler", "matchesCurrentBoot"],
  },
  {
    id: "doctor-rollback-uninstall-local-workflows",
    files: [
      "scripts/local-doctor-repair-validation-summary.mjs",
      "scripts/local-rollback-validation-summary.mjs",
      "scripts/local-uninstall-validation-summary.mjs",
    ],
    markers: ["releaseGateEvidence: false", "destructive: false"],
  },
  {
    id: "updater-retry-recovery",
    files: ["src-tauri/src/app_update_commands.rs", "src-tauri/src/lib.rs"],
    markers: [
      "failed update remains retryable",
      "install_pending_update_failure_restores_retryable_pending_update",
      "install_pending_update_retries_same_update_after_transient_failure",
      "if pending.is_none()",
    ],
  },
  {
    id: "launch-at-login-lifecycle",
    files: ["src-tauri/src/lib.rs", "src-tauri/src/dedicated_cleanup_rollback.rs"],
    markers: [
      "set_autostart_enabled",
      "dedicated_cleanup_rollback_removes_managed_launch_agents_only",
      "Tauri autostart disable was requested",
    ],
  },
  {
    id: "legacy-storage-migration",
    files: ["src-tauri/src/storage.rs"],
    markers: [
      "migration_copies_legacy_storage_and_preserves_legacy",
      "migration_skips_when_new_storage_exists",
      "migration_failure_leaves_legacy_storage_intact",
      "copy-preserve-legacy",
    ],
  },
];

function inspectLocalContract(contract) {
  const bodies = contract.files.map((file) => ({ file, body: read(file) }));
  const missingFiles = bodies.filter(({ body }) => body === null).map(({ file }) => file);
  const combined = bodies.map(({ body }) => body || "").join("\n");
  const missingMarkers = contract.markers.filter((marker) => !combined.includes(marker));
  return {
    id: contract.id,
    contractPresent: missingFiles.length === 0 && missingMarkers.length === 0,
    files: contract.files,
    missingFiles,
    missingMarkers,
    evidenceClass: "implementation-contract-only",
    note: "Presence is not execution proof and is never signed/reboot release evidence.",
  };
}

const rebootProofPath = path.join(
  evidenceDir,
  "reboot-level-installed-proof-summary.json",
);
const operationsProofPath = path.join(
  evidenceDir,
  "phase6-installed-operations-proof.json",
);
const rebootProof = readJson(rebootProofPath);
const operationsProof = readJson(operationsProofPath);

const rebootProofValid = Boolean(
  rebootProof?.schemaVersion === 1 &&
    rebootProof?.kind === "mac_ai_switchboard.reboot_level_installed_proof" &&
    rebootProof?.releaseGateEvidence === true &&
    rebootProof?.proofReady === true &&
    rebootProof?.trust?.ready === true &&
    rebootProof?.rebootMarker?.matchesCurrentBoot === true &&
    rebootProof?.rebootMarker?.installedAppTrustVerified === true &&
    rebootProof?.destructive === false,
);

const requiredOperations = [
  "doctor",
  "rollback",
  "uninstallCleanup",
  "updaterRecovery",
  "launchAtLogin",
  "legacyStorageMigration",
];
function operationEvidenceValid(operation) {
  if (
    operation?.verified !== true ||
    typeof operation?.evidenceArtifact !== "string" ||
    typeof operation?.evidenceSha256 !== "string" ||
    !/^[a-f0-9]{64}$/.test(operation.evidenceSha256)
  ) {
    return false;
  }
  const artifactPath = path.resolve(evidenceDir, operation.evidenceArtifact);
  const insideEvidenceDir =
    artifactPath === evidenceDir ||
    artifactPath.startsWith(`${evidenceDir}${path.sep}`);
  if (!insideEvidenceDir || !fs.statSync(artifactPath, { throwIfNoEntry: false })?.isFile()) {
    return false;
  }
  const actual = crypto
    .createHash("sha256")
    .update(fs.readFileSync(artifactPath))
    .digest("hex");
  return actual === operation.evidenceSha256;
}
const operationsProofValid = Boolean(
  operationsProof?.schemaVersion === 1 &&
    operationsProof?.kind === "ai_switchboard.phase6_installed_operations_proof" &&
    operationsProof?.releaseGateEvidence === true &&
    operationsProof?.contentOrSecretsRecorded === false &&
    Number.isInteger(operationsProof?.bootTimeUnixSeconds) &&
    typeof operationsProof?.installedAppArtifactSha256 === "string" &&
    /^[a-f0-9]{64}$/.test(operationsProof.installedAppArtifactSha256) &&
    requiredOperations.every((id) =>
      operationEvidenceValid(operationsProof?.operations?.[id]),
    ),
);

const localResults = localContracts.map(inspectLocalContract);
const blockers = [
  ...localResults
    .filter((result) => !result.contractPresent)
    .map((result) => `implementation contract missing: ${result.id}`),
  rebootProofValid
    ? null
    : `current signed/notarized post-reboot proof missing or blocked: ${rebootProofPath}`,
  operationsProofValid
    ? null
    : `installed Doctor/rollback/uninstall/updater/login/migration proof missing or blocked: ${operationsProofPath}`,
].filter(Boolean);

const report = {
  schemaVersion: 1,
  kind: "ai_switchboard.phase6_release_hardening_audit",
  generatedAt: new Date().toISOString(),
  implementationContractsPresent: localResults.every(
    (result) => result.contractPresent,
  ),
  localContracts: localResults,
  externalEvidence: {
    signedInstalledReboot: {
      path: rebootProofPath,
      present: fs.existsSync(rebootProofPath),
      valid: rebootProofValid,
    },
    installedOperations: {
      path: operationsProofPath,
      present: fs.existsSync(operationsProofPath),
      valid: operationsProofValid,
      requiredOperations,
    },
  },
  releaseProofReady: blockers.length === 0,
  blockers,
  claimsBoundary:
    "Source and local-test contracts do not prove a signed/notarized installed app, a physical reboot, or installed-app operations. Release proof requires both external artifacts.",
};

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Phase 6 release-hardening audit written: ${outputPath}`);
console.log(`Release proof ready: ${report.releaseProofReady ? "yes" : "no"}`);
if (blockers.length) {
  for (const blocker of blockers) console.error(`BLOCKED: ${blocker}`);
  process.exitCode = 1;
}
