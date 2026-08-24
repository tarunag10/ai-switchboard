#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
HELPER_ROOT="${REPO_ROOT}/src-tauri/codex-probe-helper-app"
HELPER_TARGET_DIR="${HELPER_ROOT}/target"
STAGING_ROOT="${REPO_ROOT}/src-tauri/target/codex-probe-helper-bundle"
STAGED_APP="${STAGING_ROOT}/AI Switchboard Codex Probe.app"
INFO_PLIST="${HELPER_ROOT}/Info.plist"
ENTITLEMENTS="${HELPER_ROOT}/Entitlements.plist"
TARGET_TRIPLE="${TAURI_ENV_TARGET_TRIPLE:-}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "The Codex probe helper can only be prepared for a macOS bundle on macOS." >&2
  exit 1
fi

if [[ -z "${TARGET_TRIPLE}" ]]; then
  TARGET_TRIPLE="$(rustc -vV | awk '/^host:/ { print $2 }')"
fi

case "${TARGET_TRIPLE}" in
  aarch64-apple-darwin|x86_64-apple-darwin|universal-apple-darwin) ;;
  *)
    echo "Unsupported Codex probe helper target: ${TARGET_TRIPLE}" >&2
    exit 1
    ;;
esac

plutil -lint "${INFO_PLIST}" "${ENTITLEMENTS}" >/dev/null
mkdir -p "${STAGING_ROOT}"
TEMP_ROOT="$(mktemp -d "${STAGING_ROOT}/.codex-probe-helper.XXXXXX")"
trap 'rm -rf "${TEMP_ROOT}"' EXIT
TEMP_APP="${TEMP_ROOT}/AI Switchboard Codex Probe.app"
TEMP_EXECUTABLE="${TEMP_APP}/Contents/MacOS/ai-switchboard-codex-probe"
mkdir -p "${TEMP_APP}/Contents/MacOS"

build_target() {
  local target="$1"
  CARGO_TARGET_DIR="${HELPER_TARGET_DIR}" cargo build \
    --locked \
    --release \
    --manifest-path "${HELPER_ROOT}/Cargo.toml" \
    --target "${target}"
}

if [[ "${TARGET_TRIPLE}" == "universal-apple-darwin" ]]; then
  build_target aarch64-apple-darwin
  build_target x86_64-apple-darwin
  lipo -create \
    "${HELPER_TARGET_DIR}/aarch64-apple-darwin/release/ai-switchboard-codex-probe" \
    "${HELPER_TARGET_DIR}/x86_64-apple-darwin/release/ai-switchboard-codex-probe" \
    -output "${TEMP_EXECUTABLE}"
  chmod 0755 "${TEMP_EXECUTABLE}"
else
  build_target "${TARGET_TRIPLE}"
  install -m 0755 \
    "${HELPER_TARGET_DIR}/${TARGET_TRIPLE}/release/ai-switchboard-codex-probe" \
    "${TEMP_EXECUTABLE}"
fi

install -m 0644 "${INFO_PLIST}" "${TEMP_APP}/Contents/Info.plist"

SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
CODESIGN_ARGS=(
  --force
  --sign "${SIGNING_IDENTITY}"
  --entitlements "${ENTITLEMENTS}"
  --options runtime
)

if [[ -n "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN:-}" ]]; then
  if [[ ! -f "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN}" ]]; then
    echo "Configured signing keychain does not exist: ${AI_SWITCHBOARD_CODESIGN_KEYCHAIN}" >&2
    exit 1
  fi
  CODESIGN_ARGS+=(--keychain "${AI_SWITCHBOARD_CODESIGN_KEYCHAIN}")
fi

if [[ "${SIGNING_IDENTITY}" != "-" ]]; then
  CODESIGN_ARGS+=(--timestamp)
fi

codesign "${CODESIGN_ARGS[@]}" "${TEMP_APP}"
"${SCRIPT_DIR}/verify-codex-probe-helper-app.sh" "${TEMP_APP}" "${TARGET_TRIPLE}"

if [[ -e "${STAGED_APP}" || -L "${STAGED_APP}" ]]; then
  rm -rf "${STAGED_APP}"
fi
mv "${TEMP_APP}" "${STAGED_APP}"

echo "Prepared nested Codex probe helper at ${STAGED_APP}"
