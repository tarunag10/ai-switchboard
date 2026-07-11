#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const armPath =
  process.env.MAC_AI_SWITCHBOARD_REBOOT_ARM_PATH ??
  "dist/reboot-level-installed-proof-arm.json";

function command(command, args) {
  try {
    return execFileSync(command, args, { encoding: "utf8", timeout: 15_000 }).trim();
  } catch {
    return null;
  }
}

function bootSession() {
  const rawBootTime = command("sysctl", ["-n", "kern.boottime"]);
  const match = rawBootTime?.match(/sec = (\d+)/);
  return {
    bootTimeUnixSeconds: match ? Number(match[1]) : null,
    hostUuid: command("sysctl", ["-n", "kern.hostuuid"]),
  };
}

const session = bootSession();
if (!session.bootTimeUnixSeconds) {
  console.error("Cannot arm reboot proof: macOS boot-session identity is unavailable.");
  process.exit(1);
}

const payload = {
  schemaVersion: 1,
  kind: "mac_ai_switchboard.reboot_level_installed_proof_arm",
  armedAt: new Date().toISOString(),
  hostname: os.hostname(),
  nonce: crypto.randomUUID(),
  bootSession: session,
  note:
    "This arm receipt proves only the pre-reboot baseline. Run the marker command after a real reboot; it refuses to write a marker unless this boot session changes and installed-app trust checks pass.",
};

fs.mkdirSync(path.dirname(armPath), { recursive: true });
fs.writeFileSync(armPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(`Reboot proof armed for boot ${session.bootTimeUnixSeconds}: ${armPath}`);
