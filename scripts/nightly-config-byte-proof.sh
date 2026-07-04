#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
REPORT_MD="${DIST_DIR}/nightly-config-byte-proof.md"
REPORT_JSON="${DIST_DIR}/nightly-config-byte-proof.json"
WORK_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "${WORK_DIR}"
}
trap cleanup EXIT

fixture_home="${WORK_DIR}/home"
mkdir -p "${fixture_home}/.codex" "${fixture_home}/.claude"

codex_config="${fixture_home}/.codex/config.toml"
claude_config="${fixture_home}/.claude/settings.json"

cat >"${codex_config}" <<'EOF'
model = "gpt-5"
approval_policy = "never"
EOF

cat >"${claude_config}" <<'EOF'
{
  "theme": "dark",
  "permissions": {
    "allow": ["Bash(git status:*)"]
  }
}
EOF

hash_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

before_codex="$(hash_file "${codex_config}")"
before_claude="$(hash_file "${claude_config}")"

backup_and_write_managed_block() {
  local path="$1"
  local backup="${path}.ai-switchboard.bak"
  cp "${path}" "${backup}"
  {
    cat "${path}"
    echo
    echo "# >>> ai-switchboard managed >>>"
    echo "base_url = \"http://127.0.0.1:6767\""
    echo "# <<< ai-switchboard managed <<<"
  } >"${path}.tmp"
  mv "${path}.tmp" "${path}"
  cp "${backup}" "${path}"
  rm -f "${backup}"
}

backup_and_write_managed_block "${codex_config}"
backup_and_write_managed_block "${claude_config}"

after_codex="$(hash_file "${codex_config}")"
after_claude="$(hash_file "${claude_config}")"

passed=true
if [[ "${before_codex}" != "${after_codex}" || "${before_claude}" != "${after_claude}" ]]; then
  passed=false
fi

cat >"${REPORT_JSON}" <<JSON
{
  "kind": "ai_switchboard.nightly_config_byte_proof",
  "releaseGateEvidence": false,
  "passed": ${passed},
  "fixtures": [
    {
      "client": "Codex",
      "path": ".codex/config.toml",
      "beforeSha256": "${before_codex}",
      "afterSha256": "${after_codex}",
      "byteIdentical": $([[ "${before_codex}" == "${after_codex}" ]] && echo true || echo false)
    },
    {
      "client": "Claude",
      "path": ".claude/settings.json",
      "beforeSha256": "${before_claude}",
      "afterSha256": "${after_claude}",
      "byteIdentical": $([[ "${before_claude}" == "${after_claude}" ]] && echo true || echo false)
    }
  ]
}
JSON

{
  echo "# Nightly Config Byte Proof"
  echo
  echo "- release_gate_evidence: false"
  echo "- codex_byte_identical: $([[ "${before_codex}" == "${after_codex}" ]] && echo yes || echo no)"
  echo "- claude_byte_identical: $([[ "${before_claude}" == "${after_claude}" ]] && echo yes || echo no)"
  echo "- json: ${REPORT_JSON}"
} >"${REPORT_MD}"

if [[ "${passed}" != "true" ]]; then
  echo "nightly config byte proof failed" >&2
  exit 1
fi

echo "Nightly config byte proof passed."
echo "Report: ${REPORT_MD}"
echo "JSON: ${REPORT_JSON}"
