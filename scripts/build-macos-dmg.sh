#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SIGNING_TEMP_BASE="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
SIGNING_ENV_FILE=""
TEMP_NOTARIZATION_KEY_PATH=""
IMPORTED_SIGNING_KEYCHAIN=0

cleanup() {
  local status=$?
  trap - EXIT

  if [[ "${IMPORTED_SIGNING_KEYCHAIN}" == "1" ]]; then
    if ! AI_SWITCHBOARD_SIGNING_TEMP_BASE="${SIGNING_TEMP_BASE}" \
      "${REPO_ROOT}/scripts/cleanup-macos-signing-keychain.sh"; then
      echo "Failed to remove the temporary signing keychain." >&2
      status=1
    fi
  fi
  if [[ -n "${SIGNING_ENV_FILE}" ]]; then
    rm -f "${SIGNING_ENV_FILE}"
  fi
  if [[ -n "${TEMP_NOTARIZATION_KEY_PATH}" ]]; then
    rm -f "${TEMP_NOTARIZATION_KEY_PATH}"
  fi

  exit "${status}"
}
trap cleanup EXIT

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script only runs on macOS." >&2
  exit 1
fi

load_env_file() {
  local path="$1"
  if [[ -f "${path}" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "${path}"
    set +a
  fi
}

load_env_file "${REPO_ROOT}/.env"
load_env_file "${REPO_ROOT}/.env.local"

require_env() {
  local key="$1"
  if [[ -z "${!key:-}" ]]; then
    echo "Missing required environment variable: ${key}" >&2
    exit 1
  fi
}

load_env_value_from_file() {
  local key="$1"
  local value="${!key:-}"

  if [[ -n "${value}" && -f "${value}" ]]; then
    export "${key}=$(<"${value}")"
  fi
}

prepare_notarization() {
  if [[ -n "${APPLE_API_KEY_PATH:-}" ]]; then
    require_env APPLE_API_KEY
    require_env APPLE_API_ISSUER
    return 0
  fi

  if [[ -n "${APPLE_API_PRIVATE_KEY_P8:-}" ]]; then
    require_env APPLE_API_KEY
    require_env APPLE_API_ISSUER

    TEMP_NOTARIZATION_KEY_PATH="$(mktemp "${TMPDIR:-/tmp}/headroom-authkey.XXXXXX.p8")"
    printf '%s' "${APPLE_API_PRIVATE_KEY_P8}" > "${TEMP_NOTARIZATION_KEY_PATH}"
    export APPLE_API_KEY_PATH="${TEMP_NOTARIZATION_KEY_PATH}"
    return 0
  fi

  if [[ -n "${APPLE_ID:-}" || -n "${APPLE_PASSWORD:-}" || -n "${APPLE_TEAM_ID:-}" ]]; then
    require_env APPLE_ID
    require_env APPLE_PASSWORD
    require_env APPLE_TEAM_ID
    return 0
  fi

  echo "Configure notarization with either APPLE_API_* variables or APPLE_ID/APPLE_PASSWORD/APPLE_TEAM_ID." >&2
  exit 1
}

import_signing_certificate_if_needed() {
  if [[ -z "${APPLE_CERTIFICATE:-}" && -z "${APPLE_CERTIFICATE_PASSWORD:-}" ]]; then
    return 0
  fi

  require_env APPLE_CERTIFICATE
  require_env APPLE_CERTIFICATE_PASSWORD

  local existing_identities
  local matching_identity_line
  local existing_identity_sha1
  existing_identities="$(security find-identity -v -p codesigning)"
  matching_identity_line="$(grep -F -m1 -- "${APPLE_SIGNING_IDENTITY}" <<<"${existing_identities}" || true)"
  existing_identity_sha1="$(awk '{ print $2 }' <<<"${matching_identity_line}")"
  if [[ "${existing_identity_sha1}" =~ ^[0-9A-Fa-f]{40}$ ]]; then
    export APPLE_SIGNING_IDENTITY="${existing_identity_sha1}"
    unset APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD
    echo "Using the existing matching macOS signing identity; skipped duplicate PKCS#12 import."
    return 0
  fi

  SIGNING_ENV_FILE="$(mktemp "${SIGNING_TEMP_BASE}/ai-switchboard-signing-env.XXXXXX")"
  AI_SWITCHBOARD_SIGNING_TEMP_BASE="${SIGNING_TEMP_BASE}" \
    AI_SWITCHBOARD_SIGNING_ENV_FILE="${SIGNING_ENV_FILE}" \
    "${REPO_ROOT}/scripts/import-macos-signing-certificate.sh"

  while IFS='=' read -r key value; do
    case "${key}" in
      AI_SWITCHBOARD_CODESIGN_KEYCHAIN|AI_SWITCHBOARD_SIGNING_KEYCHAIN_ROOT|AI_SWITCHBOARD_SIGNING_TEMP_BASE|APPLE_SIGNING_IDENTITY)
        printf -v "${key}" '%s' "${value}"
        export "${key}"
        ;;
      *)
        echo "Unexpected signing environment key: ${key}" >&2
        exit 1
        ;;
    esac
  done <"${SIGNING_ENV_FILE}"

  IMPORTED_SIGNING_KEYCHAIN=1
  unset APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD
}

require_env APPLE_SIGNING_IDENTITY
require_env TAURI_SIGNING_PRIVATE_KEY
require_env TAURI_SIGNING_PRIVATE_KEY_PASSWORD

load_env_value_from_file TAURI_SIGNING_PRIVATE_KEY
load_env_value_from_file HEADROOM_UPDATER_PUBLIC_KEY

import_signing_certificate_if_needed
prepare_notarization

if [[ -z "${HEADROOM_UPDATER_PUBLIC_KEY:-}" || -z "${HEADROOM_UPDATER_ENDPOINTS:-}" ]]; then
  echo "Warning: HEADROOM_UPDATER_PUBLIC_KEY or HEADROOM_UPDATER_ENDPOINTS is missing." >&2
  echo "The DMG will still build, but in-app update checks will be disabled in that app build." >&2
fi

export CI="${CI:-true}"

cd "${REPO_ROOT}"
./scripts/verify-release.sh

if [[ -n "${TARGET:-}" ]]; then
  npx tauri build --bundles dmg --ci --target "${TARGET}"
else
  npx tauri build --bundles dmg --ci
fi

echo "Checking built local-free application privacy..."
npm run check:local-build-privacy

rename_built_dmg() {
  local version="$1"
  local bundle_dir="${REPO_ROOT}/src-tauri/target"

  if [[ -n "${TARGET:-}" ]]; then
    bundle_dir="${bundle_dir}/${TARGET}"
  fi

  bundle_dir="${bundle_dir}/release/bundle/dmg"

  if [[ ! -d "${bundle_dir}" ]]; then
    echo "Expected DMG output directory not found: ${bundle_dir}" >&2
    exit 1
  fi

  shopt -s nullglob
  local dmgs=("${bundle_dir}"/*.dmg)
  shopt -u nullglob

  if [[ ${#dmgs[@]} -eq 0 ]]; then
    echo "No DMG artifact found in ${bundle_dir}." >&2
    exit 1
  fi

  local desired_path="${bundle_dir}/Mac-AI-Switchboard_${version}.dmg"
  local source_path=""

  for candidate in "${dmgs[@]}"; do
    if [[ "${candidate}" != "${desired_path}" ]]; then
      source_path="${candidate}"
      break
    fi
  done

  if [[ -z "${source_path}" ]]; then
    source_path="${desired_path}"
  fi

  if [[ "${source_path}" != "${desired_path}" ]]; then
    mv -f "${source_path}" "${desired_path}"

    if [[ -f "${source_path}.sig" ]]; then
      mv -f "${source_path}.sig" "${desired_path}.sig"
    fi
  fi

  echo "DMG ready at ${desired_path}"
}

APP_VERSION="$(node -p "require('./package.json').version")"
rename_built_dmg "${APP_VERSION}"
