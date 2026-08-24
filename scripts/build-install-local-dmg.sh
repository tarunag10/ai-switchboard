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

APP_NAME_CANDIDATES=("AI Switchboard" "AI Switchboard for Mac" "Mac AI Switchboard" "Mac Switchboard" Switchboard)

LOCAL_DIR="dist/release-artifacts"
LOCAL_DMG="${LOCAL_DIR}/Mac-AI-Switchboard_${APP_VERSION}-local-unsigned-${DMG_ARCH}.dmg"
APP_DEST="${MAC_AI_SWITCHBOARD_LOCAL_APP_DEST:-/Applications/AI Switchboard.app}"
LEGACY_APP_DEST="/Applications/AI Switchboard for Mac.app"
HELPER_APP_RELATIVE="Contents/Helpers/AI Switchboard Codex Probe.app"
HELPER_ENTITLEMENTS="${REPO_ROOT}/src-tauri/codex-probe-helper-app/Entitlements.plist"
PARENT_ENTITLEMENTS="${REPO_ROOT}/src-tauri/Entitlements.plist"

validate_app_destination() {
  local destination="$1"
  local parent
  local name
  parent="$(dirname "${destination}")"
  name="$(basename "${destination}")"

  if [[ "${parent}" != "/Applications" || "${name}" != *.app || "${name}" == ".app" ]]; then
    echo "Local app destination must be a named .app directly under /Applications." >&2
    exit 1
  fi
  if [[ "$(cd "${parent}" && pwd -P)" != "/Applications" ]]; then
    echo "Local app destination parent does not resolve to /Applications." >&2
    exit 1
  fi
  if [[ -L "${destination}" ]]; then
    echo "Refusing to replace a symlinked local app destination: ${destination}" >&2
    exit 1
  fi
  if [[ -e "${destination}" && ! -d "${destination}" ]]; then
    echo "Refusing to replace a non-directory local app destination: ${destination}" >&2
    exit 1
  fi
  if [[ -d "${destination}" ]]; then
    local existing_info="${destination}/Contents/Info.plist"
    local existing_identifier=""
    if [[ -f "${existing_info}" && ! -L "${existing_info}" ]]; then
      existing_identifier="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "${existing_info}" 2>/dev/null || true)"
    fi
    if [[ "${existing_identifier}" != "com.tarunagarwal.mac-ai-switchboard" ]]; then
      echo "Refusing to replace an app not owned by AI Switchboard: ${destination}" >&2
      exit 1
    fi
  fi
}

validate_app_destination "${APP_DEST}"

echo "Building local unsigned/ad-hoc DMG..."
CI=true npx tauri build --bundles dmg --ci

RAW_DMG="$(node scripts/release-artifact-cli.mjs "src-tauri/target/release/bundle/dmg" "${APP_VERSION}")"

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

# Revalidate the live destination after the lengthy build/mount/process-stop
# sequence and immediately before the destructive replacement.
validate_app_destination "${APP_DEST}"
rm -rf -- "${APP_DEST:?}"
ditto "${DMG_APP}" "${APP_DEST}"
HELPER_APP="${APP_DEST}/${HELPER_APP_RELATIVE}"
if [[ ! -d "${HELPER_APP}" ]]; then
  echo "Installed bundle is missing the nested Codex probe helper: ${HELPER_APP}" >&2
  exit 1
fi

# Sign nested code first with its narrower sandbox-only entitlement. Signing the
# parent with --deep would overwrite that boundary with the parent's settings.
codesign --force --sign - \
  --entitlements "${HELPER_ENTITLEMENTS}" \
  --options runtime \
  "${HELPER_APP}"
"${REPO_ROOT}/scripts/verify-codex-probe-helper-app.sh" "${HELPER_APP}"
codesign --force --sign - \
  --entitlements "${PARENT_ENTITLEMENTS}" \
  --options runtime \
  "${APP_DEST}"
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
