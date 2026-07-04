#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
REPORT_MD="${DIST_DIR}/nightly-golden-path-proof.md"
REPORT_JSON="${DIST_DIR}/nightly-optimization-telemetry-proof.json"
LIVE_PROOF="${LIVE_NIGHTLY_PROOF:-0}"

mkdir -p "${DIST_DIR}"

telemetry_input="$(mktemp)"
trap 'rm -f "${telemetry_input}"' EXIT

cat >"${telemetry_input}" <<'JSON'
{
  "tokenXray": {
    "totalTokens": 1000,
    "promptTokens": 700,
    "completionTokens": 200,
    "toolTokens": 100
  },
  "cache": {
    "readTokens": 300,
    "writeTokens": 100,
    "uncachedInputTokens": 400
  },
  "summary": {
    "tokensSaved": 120,
    "summaryTokens": 80,
    "fallbackCount": 0
  },
  "modelRouting": {
    "routes": [
      {
        "provider": "openai",
        "model": "gpt-5",
        "requests": 2,
        "fallbacks": 0
      }
    ]
  }
}
JSON

node "${ROOT_DIR}/scripts/optimization-report.mjs" --input "${telemetry_input}" --json >"${REPORT_JSON}"

node --input-type=module - "${REPORT_JSON}" <<'NODE'
import fs from "node:fs";

const report = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const failures = [];

if (report.tokenXray?.totalTokens !== 1000) failures.push("tokenXray.totalTokens");
if (report.cacheEfficiency?.uncachedInputTokens !== 400) failures.push("cacheEfficiency.uncachedInputTokens");
if (report.modelRouting?.routes?.[0]?.model !== "gpt-5") failures.push("modelRouting.routes[0].model");

if (failures.length) {
  console.error(`nightly telemetry proof failed: ${failures.join(", ")}`);
  process.exit(1);
}
NODE

{
  echo "# Nightly Golden Path Proof"
  echo
  echo "- generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "- live proof requested: ${LIVE_PROOF}"
  echo "- optimization telemetry replay: passed"
  echo "- persisted proof JSON: ${REPORT_JSON}"
  echo
  echo "## Current Scaffold"
  echo
  echo "This nightly proof is intentionally non-invasive. It replays metrics-only optimization telemetry through the repo-owned report parser and persists an assertion artifact."
  echo
  echo "## Remaining Live Prerequisites"
  echo
  echo "- Clean install: one canonical installed app path shared by local install and installed-smoke scripts."
  echo "- Enable/replay: a CI-safe command that uses an isolated config home or writes a restorable backup."
  echo "- Assert persisted runtime telemetry: a live app command or fixture that writes optimization telemetry outside process memory."
  echo "- Uninstall/config restore: a repo-owned command that removes the app and verifies config restoration without touching unrelated user files."
} >"${REPORT_MD}"

if [[ "${LIVE_PROOF}" == "1" || "${LIVE_PROOF}" == "true" ]]; then
  {
    echo
    echo "Live proof was requested, but the full install/enable/replay/uninstall restore path is not yet non-invasive."
    echo "Refusing to mutate the runner until the prerequisites above exist."
  } >>"${REPORT_MD}"
  exit 1
fi

echo "Nightly golden-path scaffold passed."
echo "Report: ${REPORT_MD}"
echo "Telemetry JSON: ${REPORT_JSON}"
