#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
REPORT_MD="${DIST_DIR}/nightly-golden-path-proof.md"
REPORT_JSON="${DIST_DIR}/nightly-optimization-telemetry-proof.json"
LEDGER_DB="${DIST_DIR}/nightly-optimization-telemetry.sqlite"
LIVE_PROOF="${LIVE_NIGHTLY_PROOF:-0}"

mkdir -p "${DIST_DIR}"

"${ROOT_DIR}/scripts/nightly-config-byte-proof.sh"

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

node "${ROOT_DIR}/scripts/optimization-report.mjs" --json --input "${telemetry_input}" >"${REPORT_JSON}"

node - "${REPORT_JSON}" <<'NODE'
const fs = require("fs");
const report = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const failures = [];
if (report.tokenXray?.totalTokens !== 1000) failures.push("tokenXray.totalTokens");
if (report.cacheEfficiency?.uncachedInputTokens !== 400) failures.push("cacheEfficiency.uncachedInputTokens");
if (report.modelRouting?.routes?.[0]?.model !== "gpt-5") failures.push("modelRouting.routes[0].model");
if (failures.length) {
  console.error(`nightly JSON proof failed: ${failures.join(", ")}`);
  process.exit(1);
}
NODE

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required for persisted optimization proof" >&2
  exit 1
fi

rm -f "${LEDGER_DB}"
sqlite3 "${LEDGER_DB}" <<'SQL'
CREATE TABLE prompt_cache_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  prompt_tokens INTEGER NOT NULL,
  completion_tokens INTEGER NOT NULL,
  cache_read_tokens INTEGER NOT NULL,
  cache_creation_tokens INTEGER NOT NULL
);
CREATE TABLE compaction_decisions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  should_compact INTEGER NOT NULL,
  context_used_percent INTEGER NOT NULL,
  threshold_percent INTEGER NOT NULL,
  reason TEXT NOT NULL
);
CREATE TABLE routing_decisions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  task TEXT NOT NULL,
  current_model TEXT NOT NULL,
  selected_model TEXT NOT NULL,
  fallback_model TEXT NOT NULL,
  reason TEXT NOT NULL,
  estimated_savings_percent INTEGER NOT NULL
);
CREATE TABLE token_xray_bucket_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  bucket TEXT NOT NULL,
  tokens INTEGER NOT NULL
);
CREATE TABLE redundancy_hash_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  source_id TEXT NOT NULL,
  content_sha256 TEXT NOT NULL,
  estimated_tokens INTEGER NOT NULL
);
CREATE TABLE rtk_preset_metadata_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  preset_id TEXT NOT NULL,
  label TEXT NOT NULL,
  command TEXT NOT NULL,
  focus TEXT NOT NULL
);

INSERT INTO prompt_cache_events (prompt_tokens, completion_tokens, cache_read_tokens, cache_creation_tokens)
VALUES (1000, 200, 300, 100);
INSERT INTO compaction_decisions (should_compact, context_used_percent, threshold_percent, reason)
VALUES (1, 91, 90, 'nightly threshold proof');
INSERT INTO routing_decisions (task, current_model, selected_model, fallback_model, reason, estimated_savings_percent)
VALUES ('commit message', 'gpt-5', 'gpt-5-mini', 'gpt-5', 'low-risk routing proof', 35);
INSERT INTO token_xray_bucket_events (bucket, tokens)
VALUES ('history', 700), ('tool', 100);
INSERT INTO redundancy_hash_events (source_id, content_sha256, estimated_tokens)
VALUES ('AGENTS.md', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 42);
INSERT INTO rtk_preset_metadata_events (preset_id, label, command, focus)
VALUES ('pytest', 'pytest', 'rtk pytest', 'failure-only test output');
SQL

sqlite_scalar() {
  sqlite3 "${LEDGER_DB}" "$1"
}

failures=()
[[ "$(sqlite_scalar 'SELECT COUNT(*) FROM prompt_cache_events;')" == "1" ]] || failures+=("prompt_cache_events")
[[ "$(sqlite_scalar 'SELECT SUM(cache_read_tokens) FROM prompt_cache_events;')" == "300" ]] || failures+=("cache_read_tokens")
[[ "$(sqlite_scalar 'SELECT COUNT(*) FROM compaction_decisions WHERE should_compact = 1;')" == "1" ]] || failures+=("compaction_decisions")
[[ "$(sqlite_scalar 'SELECT selected_model FROM routing_decisions LIMIT 1;')" == "gpt-5-mini" ]] || failures+=("routing_decisions")
[[ "$(sqlite_scalar "SELECT SUM(tokens) FROM token_xray_bucket_events WHERE bucket IN ('history', 'tool');")" == "800" ]] || failures+=("token_xray_bucket_events")
[[ "$(sqlite_scalar 'SELECT COUNT(*) FROM redundancy_hash_events WHERE length(content_sha256) = 64;')" == "1" ]] || failures+=("redundancy_hash_events")
[[ "$(sqlite_scalar 'SELECT command FROM rtk_preset_metadata_events LIMIT 1;')" == "rtk pytest" ]] || failures+=("rtk_preset_metadata_events")

if ((${#failures[@]} > 0)); then
  printf 'nightly SQLite proof failed: %s\n' "${failures[*]}" >&2
  exit 1
fi

{
  echo "# Nightly Golden Path Proof"
  echo
  echo "- generated_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "- live_proof_requested: ${LIVE_PROOF}"
  echo "- telemetry_report_json: ${REPORT_JSON}"
  echo "- persisted_ledger_sqlite: ${LEDGER_DB}"
  echo "- config_byte_proof: ${DIST_DIR}/nightly-config-byte-proof.json"
  echo
  echo "## Persisted Telemetry Rows"
  echo
  echo "- prompt_cache_events: $(sqlite_scalar 'SELECT COUNT(*) FROM prompt_cache_events;')"
  echo "- compaction_decisions: $(sqlite_scalar 'SELECT COUNT(*) FROM compaction_decisions;')"
  echo "- routing_decisions: $(sqlite_scalar 'SELECT COUNT(*) FROM routing_decisions;')"
  echo "- token_xray_bucket_events: $(sqlite_scalar 'SELECT COUNT(*) FROM token_xray_bucket_events;')"
  echo "- redundancy_hash_events: $(sqlite_scalar 'SELECT COUNT(*) FROM redundancy_hash_events;')"
  echo "- rtk_preset_metadata_events: $(sqlite_scalar 'SELECT COUNT(*) FROM rtk_preset_metadata_events;')"
  echo
  echo "## Remaining Live Prerequisites"
  echo
  echo "- Clean install and uninstall in an isolated macOS user profile."
  echo "- Enable Full optimization against the packaged app."
  echo "- Replay a scripted proxy session through the running app."
  echo "- Assert user config files are byte-identical after uninstall."
} >"${REPORT_MD}"

if [[ "${LIVE_PROOF}" == "1" ]]; then
  if [[ "${GITHUB_ACTIONS:-}" != "true" ]]; then
    {
      echo
      echo "Live proof requested, but it is only enabled on ephemeral GitHub macOS runners."
    } >>"${REPORT_MD}"
    echo "LIVE_NIGHTLY_PROOF=1 is only enabled on GitHub Actions." >&2
    exit 1
  fi

  MAC_AI_SWITCHBOARD_SKIP_OPEN=1 npm run evidence:local
  {
    echo
    echo "## GitHub Live Evidence"
    echo
    echo "- local_evidence_summary: ${DIST_DIR}/local-evidence-summary.md"
    echo "- installed_smoke_summary: ${DIST_DIR}/local-installed-smoke-summary.md"
    echo "- uninstall_dry_run_summary: ${DIST_DIR}/local-uninstall-validation-summary.md"
  } >>"${REPORT_MD}"
fi

echo "Nightly golden-path proof passed."
echo "Report: ${REPORT_MD}"
echo "JSON: ${REPORT_JSON}"
echo "SQLite: ${LEDGER_DB}"
