import assert from "node:assert/strict";
import crypto from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const root = process.cwd();
const checker = path.join(root, "scripts/check-phase6-release-hardening.mjs");

function run(evidenceDir) {
  const output = path.join(evidenceDir, "audit.json");
  const result = spawnSync(process.execPath, [checker], {
    cwd: root,
    encoding: "utf8",
    env: {
      ...process.env,
      P6_RELEASE_EVIDENCE_DIR: evidenceDir,
      P6_RELEASE_AUDIT_OUTPUT: output,
    },
  });
  return { result, report: JSON.parse(fs.readFileSync(output, "utf8")) };
}

test("missing external artifacts remain blocked despite local contracts", () => {
  const evidenceDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-p6-"));
  const { result, report } = run(evidenceDir);
  assert.notEqual(result.status, 0);
  assert.equal(report.releaseProofReady, false);
  assert.equal(report.externalEvidence.signedInstalledReboot.valid, false);
  assert.equal(report.externalEvidence.installedOperations.valid, false);
  assert.match(report.claimsBoundary, /do not prove/);
});

test("local-only or incomplete artifacts cannot become release proof", () => {
  const evidenceDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-p6-"));
  fs.writeFileSync(
    path.join(evidenceDir, "reboot-level-installed-proof-summary.json"),
    JSON.stringify({
      schemaVersion: 1,
      kind: "mac_ai_switchboard.reboot_level_installed_proof",
      releaseGateEvidence: false,
      proofReady: true,
      trust: { ready: true },
      rebootMarker: {
        matchesCurrentBoot: true,
        installedAppTrustVerified: true,
      },
      destructive: false,
    }),
  );
  fs.writeFileSync(
    path.join(evidenceDir, "phase6-installed-operations-proof.json"),
    JSON.stringify({
      schemaVersion: 1,
      kind: "ai_switchboard.phase6_installed_operations_proof",
      releaseGateEvidence: false,
      contentOrSecretsRecorded: false,
      bootTimeUnixSeconds: 1,
      installedAppArtifactSha256: "a".repeat(64),
      operations: Object.fromEntries(
        [
          "doctor",
          "rollback",
          "uninstallCleanup",
          "updaterRecovery",
          "launchAtLogin",
          "legacyStorageMigration",
        ].map((id) => [
          id,
          {
            verified: true,
            evidenceArtifact: `${id}.json`,
            evidenceSha256: "b".repeat(64),
          },
        ]),
      ),
    }),
  );
  const { result, report } = run(evidenceDir);
  assert.notEqual(result.status, 0);
  assert.equal(report.releaseProofReady, false);
  assert.equal(report.externalEvidence.signedInstalledReboot.valid, false);
  assert.equal(report.externalEvidence.installedOperations.valid, false);
});

test("complete external artifacts require in-scope files with matching digests", () => {
  const evidenceDir = fs.mkdtempSync(path.join(os.tmpdir(), "switchboard-p6-"));
  fs.writeFileSync(
    path.join(evidenceDir, "reboot-level-installed-proof-summary.json"),
    JSON.stringify({
      schemaVersion: 1,
      kind: "mac_ai_switchboard.reboot_level_installed_proof",
      releaseGateEvidence: true,
      proofReady: true,
      trust: { ready: true },
      rebootMarker: {
        matchesCurrentBoot: true,
        installedAppTrustVerified: true,
      },
      destructive: false,
    }),
  );
  const operations = {};
  for (const id of [
    "doctor",
    "rollback",
    "uninstallCleanup",
    "updaterRecovery",
    "launchAtLogin",
    "legacyStorageMigration",
  ]) {
    const evidenceArtifact = `${id}.json`;
    const body = JSON.stringify({ id, verified: true });
    fs.writeFileSync(path.join(evidenceDir, evidenceArtifact), body);
    operations[id] = {
      verified: true,
      evidenceArtifact,
      evidenceSha256: crypto.createHash("sha256").update(body).digest("hex"),
    };
  }
  fs.writeFileSync(
    path.join(evidenceDir, "phase6-installed-operations-proof.json"),
    JSON.stringify({
      schemaVersion: 1,
      kind: "ai_switchboard.phase6_installed_operations_proof",
      releaseGateEvidence: true,
      contentOrSecretsRecorded: false,
      bootTimeUnixSeconds: 1,
      installedAppArtifactSha256: "a".repeat(64),
      operations,
    }),
  );

  const { result, report } = run(evidenceDir);
  assert.equal(result.status, 0);
  assert.equal(report.releaseProofReady, true);
  assert.equal(report.externalEvidence.signedInstalledReboot.valid, true);
  assert.equal(report.externalEvidence.installedOperations.valid, true);
});
