#!/usr/bin/env bash

set -euo pipefail
umask 077

require_env() {
  local key="$1"
  if [[ -z "${!key:-}" ]]; then
    echo "Missing required environment variable: ${key}" >&2
    exit 1
  fi
}

require_env APPLE_CERTIFICATE
require_env APPLE_CERTIFICATE_PASSWORD
require_env APPLE_SIGNING_IDENTITY

SIGNING_TEMP_BASE="${RUNNER_TEMP:-${AI_SWITCHBOARD_SIGNING_TEMP_BASE:-}}"
SIGNING_ENV_FILE="${GITHUB_ENV:-${AI_SWITCHBOARD_SIGNING_ENV_FILE:-}}"
if [[ -z "${SIGNING_TEMP_BASE}" || -z "${SIGNING_ENV_FILE}" ]]; then
  echo "Signing import requires a temporary base and an environment output file." >&2
  exit 1
fi
if [[ ! -d "${SIGNING_TEMP_BASE}" || -L "${SIGNING_TEMP_BASE}" ]]; then
  echo "Signing temporary base must be an existing non-symlink directory." >&2
  exit 1
fi
if [[ -L "${SIGNING_ENV_FILE}" || -d "${SIGNING_ENV_FILE}" ]]; then
  echo "Signing environment output must be a regular file path, never a symlink or directory." >&2
  exit 1
fi
SIGNING_ENV_PARENT="$(dirname "${SIGNING_ENV_FILE}")"
if [[ ! -d "${SIGNING_ENV_PARENT}" ]]; then
  echo "Signing environment output directory does not exist." >&2
  exit 1
fi

SIGNING_ROOT="$(mktemp -d "${SIGNING_TEMP_BASE}/ai-switchboard-signing.XXXXXX")"
CERTIFICATE_PATH="${SIGNING_ROOT}/certificate.p12"
KEYCHAIN_PASSWORD="$(uuidgen)"
DEFAULT_KEYCHAIN="$(security default-keychain -d user | tr -d '"')"
DEFAULT_KEYCHAIN="${DEFAULT_KEYCHAIN#"${DEFAULT_KEYCHAIN%%[![:space:]]*}"}"
DEFAULT_KEYCHAIN="${DEFAULT_KEYCHAIN%"${DEFAULT_KEYCHAIN##*[![:space:]]}"}"
KEYCHAIN_DIRECTORY="$(dirname "${DEFAULT_KEYCHAIN}")"
if [[ ! -d "${KEYCHAIN_DIRECTORY}" || -L "${KEYCHAIN_DIRECTORY}" ]]; then
  echo "Could not resolve a safe user keychain directory." >&2
  rm -rf "${SIGNING_ROOT}"
  exit 1
fi
KEYCHAIN_PATH="${KEYCHAIN_DIRECTORY}/ai-switchboard-signing-$(uuidgen).keychain-db"

cleanup_on_failure() {
  local status=$?
  if [[ ${status} -ne 0 ]]; then
    security delete-keychain "${KEYCHAIN_PATH}" >/dev/null 2>&1 || true
    rm -f "${KEYCHAIN_PATH}"
    rm -rf "${SIGNING_ROOT}"
  fi
  exit "${status}"
}
trap cleanup_on_failure EXIT

printf '%s' "${APPLE_CERTIFICATE}" | openssl base64 -d -A -out "${CERTIFICATE_PATH}"
security create-keychain -p "${KEYCHAIN_PASSWORD}" "${KEYCHAIN_PATH}"
security set-keychain-settings -lut 21600 "${KEYCHAIN_PATH}"
security unlock-keychain -p "${KEYCHAIN_PASSWORD}" "${KEYCHAIN_PATH}"
security import "${CERTIFICATE_PATH}" \
  -k "${KEYCHAIN_PATH}" \
  -P "${APPLE_CERTIFICATE_PASSWORD}" \
  -f pkcs12 \
  -t agg \
  -T /usr/bin/codesign \
  -T /usr/bin/pkgbuild \
  -T /usr/bin/productbuild
security set-key-partition-list \
  -S apple-tool:,apple:,codesign: \
  -s \
  -k "${KEYCHAIN_PASSWORD}" \
  "${KEYCHAIN_PATH}" >/dev/null

existing_keychains=()
while IFS= read -r keychain; do
  keychain="${keychain//\"/}"
  keychain="${keychain#"${keychain%%[![:space:]]*}"}"
  keychain="${keychain%"${keychain##*[![:space:]]}"}"
  if [[ -n "${keychain}" && "${keychain}" != "${KEYCHAIN_PATH}" ]]; then
    existing_keychains+=("${keychain}")
  fi
done < <(security list-keychains -d user)
security list-keychains -d user -s "${KEYCHAIN_PATH}" "${existing_keychains[@]}"

IMPORTED_IDENTITIES="$(security find-identity -v -p codesigning "${KEYCHAIN_PATH}")"
MATCHING_IDENTITY_LINE="$(grep -F -m1 -- "${APPLE_SIGNING_IDENTITY}" <<<"${IMPORTED_IDENTITIES}" || true)"
IMPORTED_IDENTITY_SHA1="$(awk '{ print $2 }' <<<"${MATCHING_IDENTITY_LINE}")"
if [[ ! "${IMPORTED_IDENTITY_SHA1}" =~ ^[0-9A-Fa-f]{40}$ ]]; then
  echo "The imported keychain does not contain APPLE_SIGNING_IDENTITY." >&2
  exit 1
fi

{
  echo "AI_SWITCHBOARD_CODESIGN_KEYCHAIN=${KEYCHAIN_PATH}"
  echo "AI_SWITCHBOARD_SIGNING_KEYCHAIN_ROOT=${SIGNING_ROOT}"
  echo "AI_SWITCHBOARD_SIGNING_TEMP_BASE=${SIGNING_TEMP_BASE}"
  echo "APPLE_SIGNING_IDENTITY=${IMPORTED_IDENTITY_SHA1}"
} >>"${SIGNING_ENV_FILE}"

trap - EXIT
echo "Imported the macOS signing certificate into an ephemeral AI Switchboard keychain."
