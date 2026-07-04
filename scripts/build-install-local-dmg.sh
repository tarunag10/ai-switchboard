#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script only runs on macOS." >&2
  exit 1
fi

cd "${REPO_ROOT}"

APP_VERSION="$(node -p "require('./package.json').version")"
ARCH_NAME="$(uname -m)"
case "${ARCH_NAME}" in
  arm64) DMG_ARCH="aarch64" ;;
  x86_64) DMG_ARCH="x64" ;;
  *) DMG_ARCH="${ARCH_NAME}" ;;
esac

APP_NAME_CANDIDATES=("AI Switchboard for Mac" "AI Switchboard" "Mac AI Switchboard" "Mac Switchboard" Switchboard)
RAW_DMG=""
for app_name in "${APP_NAME_CANDIDATES[@]}"; do
  candidate="src-tauri/target/release/bundle/dmg/${app_name}_${APP_VERSION}_${DMG_ARCH}.dmg"
  if [[ -f "${candidate}" ]]; then
    RAW_DMG="${candidate}"
    break
  fi
done
if [[ -z "${RAW_DMG}" ]]; then
  RAW_DMG="$(find src-tauri/target/release/bundle/dmg -maxdepth 1 -type f -name "*_${APP_VERSION}_${DMG_ARCH}.dmg" -print -quit 2>/dev/null || true)"
fi
LOCAL_DIR="dist/release-artifacts"
LOCAL_DMG="${LOCAL_DIR}/Mac-AI-Switchboard_${APP_VERSION}-local-unsigned-${DMG_ARCH}.dmg"
APP_DEST="${MAC_AI_SWITCHBOARD_LOCAL_APP_DEST:-/Applications/AI Switchboard for Mac.app}"
LEGACY_APP_DEST="/Applications/Mac AI Switchboard.app"

echo "Building local unsigned/ad-hoc DMG..."
CI=true npx tauri build --bundles dmg --ci

if [[ -z "${RAW_DMG}" ]]; then
  echo "Expected DMG not found for any app name: ${APP_NAME_CANDIDATES[*]}" >&2
  echo "Available DMGs:" >&2
  find src-tauri/target/release/bundle/dmg -maxdepth 1 -name "*.dmg" -print >&2
  exit 1
fi

mkdir -p "${LOCAL_DIR}"
cp -f "${RAW_DMG}" "${LOCAL_DMG}"
shasum -a 256 "${LOCAL_DMG}" | tee "${LOCAL_DMG}.sha256"
hdiutil verify "${LOCAL_DMG}"

for app_name in "${APP_NAME_CANDIDATES[@]}"; do
  if mount | grep -q "on /Volumes/${app_name} "; then
    hdiutil detach "/Volumes/${app_name}" >/dev/null
  fi
done

hdiutil attach "${LOCAL_DMG}" -nobrowse -readonly
MOUNT_POINT=""
DMG_APP=""
for app_name in "${APP_NAME_CANDIDATES[@]}"; do
  candidate_mount="/Volumes/${app_name}"
  candidate_app="${candidate_mount}/${app_name}.app"
  if [[ -d "${candidate_app}" ]]; then
    MOUNT_POINT="${candidate_mount}"
    DMG_APP="${candidate_app}"
    break
  fi
done
trap 'if [[ -n "${MOUNT_POINT}" ]]; then hdiutil detach "${MOUNT_POINT}" >/dev/null 2>&1 || true; fi' EXIT

if [[ -z "${DMG_APP}" ]]; then
  echo "Mounted DMG does not contain a compatible Switchboard app bundle." >&2
  exit 1
fi

if pgrep -f "${APP_DEST}/Contents/MacOS/mac-ai-switchboard" >/dev/null 2>&1 || \
  pgrep -f "${LEGACY_APP_DEST}/Contents/MacOS/mac-ai-switchboard" >/dev/null 2>&1; then
  osascript -e 'tell application id "com.tarunagarwal.mac-ai-switchboard" to quit' >/dev/null 2>&1 || true
  sleep 2
fi
pkill -f "${APP_DEST}/Contents/MacOS/mac-ai-switchboard" >/dev/null 2>&1 || true
pkill -f "${LEGACY_APP_DEST}/Contents/MacOS/mac-ai-switchboard" >/dev/null 2>&1 || true

rm -rf "${APP_DEST}"
ditto "${DMG_APP}" "${APP_DEST}"
codesign --force --deep --sign - "${APP_DEST}"
codesign --verify --deep --strict --verbose=2 "${APP_DEST}"

npm run smoke:installed:local

if [[ "${MAC_AI_SWITCHBOARD_SKIP_OPEN:-0}" != "1" ]]; then
  open "${APP_DEST}"
  echo "Opened ${APP_DEST}"
else
  echo "Skipped opening ${APP_DEST} because MAC_AI_SWITCHBOARD_SKIP_OPEN=1"
fi

echo "Local app installed at ${APP_DEST}"
echo "Local DMG copied to ${LOCAL_DMG}"
