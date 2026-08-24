#!/usr/bin/env bash

set -euo pipefail

if [[ -z "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN:-}" || -z "${AI_SWITCHBOARD_SIGNING_KEYCHAIN_ROOT:-}" ]]; then
  echo "No AI Switchboard signing keychain is configured; nothing to clean."
  exit 0
fi

SIGNING_TEMP_BASE="${RUNNER_TEMP:-${AI_SWITCHBOARD_SIGNING_TEMP_BASE:-}}"
if [[ -z "${SIGNING_TEMP_BASE}" ]]; then
  echo "A signing temporary base is required to validate the cleanup target." >&2
  exit 1
fi

case "${AI_SWITCHBOARD_SIGNING_KEYCHAIN_ROOT}" in
  "${SIGNING_TEMP_BASE}"/ai-switchboard-signing.*) ;;
  *)
    echo "Refusing to clean an unexpected signing root: ${AI_SWITCHBOARD_SIGNING_KEYCHAIN_ROOT}" >&2
    exit 1
    ;;
esac

DEFAULT_KEYCHAIN="$(security default-keychain -d user | tr -d '"')"
DEFAULT_KEYCHAIN="${DEFAULT_KEYCHAIN#"${DEFAULT_KEYCHAIN%%[![:space:]]*}"}"
DEFAULT_KEYCHAIN="${DEFAULT_KEYCHAIN%"${DEFAULT_KEYCHAIN##*[![:space:]]}"}"
EXPECTED_KEYCHAIN_DIRECTORY="$(dirname "${DEFAULT_KEYCHAIN}")"
if [[ ! -d "${EXPECTED_KEYCHAIN_DIRECTORY}" || -L "${EXPECTED_KEYCHAIN_DIRECTORY}" ]]; then
  echo "Could not resolve a safe user keychain directory for cleanup." >&2
  exit 1
fi
KEYCHAIN_DIRECTORY="$(dirname "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN}")"
KEYCHAIN_NAME="$(basename "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN}")"
if [[ "${KEYCHAIN_DIRECTORY}" != "${EXPECTED_KEYCHAIN_DIRECTORY}" ||
  "${KEYCHAIN_NAME}" != ai-switchboard-signing-*.keychain-db ]]; then
  echo "Refusing to clean a signing keychain outside the validated user keychain directory." >&2
  exit 1
fi

remaining_keychains=()
while IFS= read -r keychain; do
  keychain="${keychain//\"/}"
  keychain="${keychain#"${keychain%%[![:space:]]*}"}"
  keychain="${keychain%"${keychain##*[![:space:]]}"}"
  if [[ -n "${keychain}" && "${keychain}" != "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN}" ]]; then
    remaining_keychains+=("${keychain}")
  fi
done < <(security list-keychains -d user)

if [[ ${#remaining_keychains[@]} -gt 0 ]]; then
  security list-keychains -d user -s "${remaining_keychains[@]}"
fi
if [[ -e "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN}" ]]; then
  security delete-keychain "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN}" >/dev/null 2>&1 || \
    rm -f "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN}"
fi
rm -rf "${AI_SWITCHBOARD_SIGNING_KEYCHAIN_ROOT}"

echo "Removed the ephemeral AI Switchboard signing keychain and certificate material."
