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

RAW_DMG="src-tauri/target/release/bundle/dmg/Mac AI Switchboard_${APP_VERSION}_${DMG_ARCH}.dmg"
LOCAL_DIR="dist/release-artifacts"
LOCAL_DMG="${LOCAL_DIR}/Mac-AI-Switchboard_${APP_VERSION}-local-unsigned-${DMG_ARCH}.dmg"
APP_DEST="/Applications/Mac AI Switchboard.app"
MOUNT_POINT="/Volumes/Mac AI Switchboard"

echo "Building local unsigned/ad-hoc DMG..."
CI=true npx tauri build --bundles dmg --ci

if [[ ! -f "${RAW_DMG}" ]]; then
  echo "Expected DMG not found: ${RAW_DMG}" >&2
  echo "Available DMGs:" >&2
  find src-tauri/target/release/bundle/dmg -maxdepth 1 -name "*.dmg" -print >&2
  exit 1
fi

mkdir -p "${LOCAL_DIR}"
cp -f "${RAW_DMG}" "${LOCAL_DMG}"
shasum -a 256 "${LOCAL_DMG}" | tee "${LOCAL_DMG}.sha256"
hdiutil verify "${LOCAL_DMG}"

if mount | grep -q "on ${MOUNT_POINT} "; then
  hdiutil detach "${MOUNT_POINT}" >/dev/null
fi

hdiutil attach "${LOCAL_DMG}" -nobrowse -readonly
trap 'hdiutil detach "${MOUNT_POINT}" >/dev/null 2>&1 || true' EXIT

if [[ ! -d "${MOUNT_POINT}/Mac AI Switchboard.app" ]]; then
  echo "Mounted DMG does not contain Mac AI Switchboard.app." >&2
  exit 1
fi

if pgrep -f "${APP_DEST}/Contents/MacOS/headroom-desktop" >/dev/null 2>&1; then
  osascript -e 'tell application id "com.tarunagarwal.mac-ai-switchboard" to quit' >/dev/null 2>&1 || true
  sleep 2
fi
pkill -f "${APP_DEST}/Contents/MacOS/headroom-desktop" >/dev/null 2>&1 || true

rm -rf "${APP_DEST}"
ditto "${MOUNT_POINT}/Mac AI Switchboard.app" "${APP_DEST}"
codesign --force --deep --sign - "${APP_DEST}"
codesign --verify --deep --strict --verbose=2 "${APP_DEST}"

npm run smoke:installed:local

echo "Local app installed at ${APP_DEST}"
echo "Local DMG copied to ${LOCAL_DMG}"
