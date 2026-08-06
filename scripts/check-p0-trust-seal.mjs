#!/usr/bin/env node
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const planDir = path.join(root, "docs/world-class-token-savings");

const requiredPlanFiles = [
  "README.md",
  "IMPLEMENTATION-PLAN.md",
  "slice-status.json",
];

const requiredSignals = [
  {
    label: "proxy session auth module",
    file: "src-tauri/src/proxy_session_auth.rs",
    needles: [
      "ProxySessionAuth",
      "PROXY_SESSION_HEADER",
      "proxy-session-auth.json",
      "get_proxy_session_auth_status",
    ],
  },
  {
    label: "proxy intercept session validation",
    file: "src-tauri/src/proxy_intercept.rs",
    needles: ["validate_request_headers", "ProxySessionValidation"],
  },
  {
    label: "mode inspector verdict",
    file: "src/lib/modeInspectorVerdict.ts",
    needles: ["deriveModeInspectorVerdict", '"aligned"', '"attention"', '"blocked"'],
  },
  {
    label: "mode inspector wiring",
    file: "src/components/SwitchboardPanel.tsx",
    needles: ["deriveModeInspectorVerdict", "Inspector verdict", "proxyAuthStatus"],
  },
  {
    label: "proxy session auth settings card",
    file: "src/components/ProxySessionAuthCard.tsx",
    needles: ["get_proxy_session_auth_status", "set_proxy_session_auth_enforce"],
  },
  {
    label: "reboot proof summary checker",
    file: "scripts/check-reboot-level-installed-proof-summary.mjs",
    needles: ["schemaVersion", "mac_ai_switchboard.reboot_level_installed_proof"],
  },
  {
    label: "world-class plan checker",
    file: "scripts/check-world-class-token-savings-plan.mjs",
    needles: ["proxy session auth module", "mode inspector verdict"],
  },
];

function fail(message) {
  console.error(`P0 trust seal check failed: ${message}`);
  process.exit(1);
}

function run(command) {
  return execSync(command, { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

for (const file of requiredPlanFiles) {
  const absolute = path.join(planDir, file);
  if (!fs.existsSync(absolute)) {
    fail(`missing plan file ${absolute}`);
  }
}

const ledger = JSON.parse(
  fs.readFileSync(path.join(planDir, "slice-status.json"), "utf8"),
);
for (const sliceId of ["P0.2", "P0.3", "P0.4"]) {
  const slice = ledger.phases?.P0?.slices?.find((entry) => entry.id === sliceId);
  if (!slice) {
    fail(`slice-status.json missing ${sliceId}`);
  }
  if (slice.status === "blocked") {
    fail(`${sliceId} must not be blocked for trust-seal automation`);
  }
}

for (const signal of requiredSignals) {
  const absolute = path.join(root, signal.file);
  if (!fs.existsSync(absolute)) {
    fail(`missing ${signal.file}`);
  }
  const contents = fs.readFileSync(absolute, "utf8");
  for (const needle of signal.needles) {
    if (!contents.includes(needle)) {
      fail(`${signal.label} missing needle ${needle} in ${signal.file}`);
    }
  }
}

const localOnlyCheck = run("node scripts/check-local-only-network.mjs");

console.log(
  JSON.stringify(
    {
      ok: true,
      phase: "P0",
      title: "Trust seal",
      localOnlyNetwork: localOnlyCheck.split("\n").at(-1),
      signals: requiredSignals.map((signal) => signal.label),
      planFiles: requiredPlanFiles,
    },
    null,
    2,
  ),
);
