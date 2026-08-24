#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEFAULT_APP="${REPO_ROOT}/src-tauri/target/codex-probe-helper-bundle/AI Switchboard Codex Probe.app"
HELPER_APP="${1:-${DEFAULT_APP}}"
EXPECTED_TARGET="${2:-${TAURI_ENV_TARGET_TRIPLE:-}}"
PARENT_APP="${3:-}"
HELPER_EXECUTABLE="${HELPER_APP}/Contents/MacOS/ai-switchboard-codex-probe"
INFO_PLIST="${HELPER_APP}/Contents/Info.plist"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Codex probe helper bundle verification only runs on macOS." >&2
  exit 1
fi

if [[ ! -d "${HELPER_APP}" || -L "${HELPER_APP}" ]]; then
  echo "Expected a non-symlink helper app at ${HELPER_APP}." >&2
  exit 1
fi

if [[ ! -f "${HELPER_EXECUTABLE}" || -L "${HELPER_EXECUTABLE}" || ! -x "${HELPER_EXECUTABLE}" ]]; then
  echo "Expected an executable, non-symlink helper binary at ${HELPER_EXECUTABLE}." >&2
  exit 1
fi

if [[ ! -f "${INFO_PLIST}" || -L "${INFO_PLIST}" ]]; then
  echo "Expected a non-symlink Info.plist at ${INFO_PLIST}." >&2
  exit 1
fi

FIRST_SYMLINK="$(find "${HELPER_APP}" -type l -print -quit)"
if [[ -n "${FIRST_SYMLINK}" ]]; then
  echo "The helper app must not contain symlinks." >&2
  exit 1
fi

plutil -lint "${INFO_PLIST}" >/dev/null

plist_value() {
  /usr/libexec/PlistBuddy -c "Print :$1" "${INFO_PLIST}"
}

[[ "$(plist_value CFBundleExecutable)" == "ai-switchboard-codex-probe" ]]
[[ "$(plist_value CFBundleIdentifier)" == "com.tarunagarwal.mac-ai-switchboard.codex-probe" ]]
[[ "$(plist_value CFBundlePackageType)" == "APPL" ]]
[[ "$(plist_value LSBackgroundOnly)" == "true" ]]
[[ "$(plist_value LSMinimumSystemVersion)" == "14.0" ]]

EXECUTABLE_KIND="$(file "${HELPER_EXECUTABLE}")"
if ! grep -q "Mach-O" <<<"${EXECUTABLE_KIND}"; then
  echo "The helper executable is not Mach-O." >&2
  exit 1
fi

actual_arches="$(lipo -archs "${HELPER_EXECUTABLE}")"
case "${EXPECTED_TARGET}" in
  "") ;;
  aarch64-apple-darwin)
    [[ "${actual_arches}" == "arm64" ]]
    ;;
  x86_64-apple-darwin)
    [[ "${actual_arches}" == "x86_64" ]]
    ;;
  universal-apple-darwin)
    [[ " ${actual_arches} " == *" arm64 "* && " ${actual_arches} " == *" x86_64 "* ]]
    ;;
  *)
    echo "Unsupported expected helper target: ${EXPECTED_TARGET}" >&2
    exit 1
    ;;
esac

while IFS= read -r dependency; do
  [[ -z "${dependency}" ]] && continue
  if [[ "${dependency}" != /usr/lib/libSystem.B.dylib* ]]; then
    echo "Unexpected helper dependency: ${dependency}" >&2
    exit 1
  fi
done < <(otool -L "${HELPER_EXECUTABLE}" | tail -n +2 | sed 's/^[[:space:]]*//')

LOAD_COMMANDS="$(otool -l "${HELPER_EXECUTABLE}")"
if grep -q "cmd LC_RPATH" <<<"${LOAD_COMMANDS}"; then
  echo "The helper executable must not contain an LC_RPATH load command." >&2
  exit 1
fi

codesign --verify --strict --verbose=2 "${HELPER_APP}"

TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ai-switchboard-helper-verify.XXXXXX")"
trap 'rm -rf "${TEMP_DIR}"' EXIT
SIGNED_ENTITLEMENTS="${TEMP_DIR}/signed-entitlements.plist"
codesign -d --entitlements :- "${HELPER_APP}" >"${SIGNED_ENTITLEMENTS}" 2>/dev/null
plutil -lint "${SIGNED_ENTITLEMENTS}" >/dev/null

node - "${SIGNED_ENTITLEMENTS}" <<'NODE'
const fs = require("node:fs");
const { execFileSync } = require("node:child_process");

const path = process.argv[2];
const json = execFileSync("plutil", ["-convert", "json", "-o", "-", path], {
  encoding: "utf8",
});
const entitlements = JSON.parse(json);
const keys = Object.keys(entitlements);
if (keys.length !== 1 || keys[0] !== "com.apple.security.app-sandbox") {
  throw new Error(`unexpected signed helper entitlements: ${keys.join(", ")}`);
}
if (entitlements["com.apple.security.app-sandbox"] !== true) {
  throw new Error("the helper App Sandbox entitlement is not true");
}
NODE

if [[ -n "${PARENT_APP}" ]]; then
  if [[ ! -d "${PARENT_APP}" || -L "${PARENT_APP}" ]]; then
    echo "Expected a non-symlink parent app at ${PARENT_APP}." >&2
    exit 1
  fi

  codesign --verify --deep --strict --verbose=2 "${PARENT_APP}"
  HELPER_SIGNATURE="$(codesign -dvvv "${HELPER_APP}" 2>&1)"
  PARENT_SIGNATURE="$(codesign -dvvv "${PARENT_APP}" 2>&1)"
  HELPER_TEAM="$(sed -n 's/^TeamIdentifier=//p' <<<"${HELPER_SIGNATURE}")"
  PARENT_TEAM="$(sed -n 's/^TeamIdentifier=//p' <<<"${PARENT_SIGNATURE}")"

  if [[ "${PARENT_SIGNATURE}" == *"Signature=adhoc"* ]]; then
    if [[ "${HELPER_SIGNATURE}" != *"Signature=adhoc"* ]]; then
      echo "Ad-hoc parent and helper signature modes do not match." >&2
      exit 1
    fi
  else
    if [[ -z "${PARENT_TEAM}" || "${PARENT_TEAM}" == "not set" ]]; then
      echo "A non-ad-hoc parent must expose a TeamIdentifier." >&2
      exit 1
    fi
    if [[ "${HELPER_TEAM}" != "${PARENT_TEAM}" ]]; then
      echo "Helper and parent TeamIdentifier values do not match." >&2
      exit 1
    fi

    DESIGNATED_REQUIREMENT="$(codesign -d -r- "${HELPER_APP}" 2>&1)"
    if [[ "${DESIGNATED_REQUIREMENT}" != *'identifier "com.tarunagarwal.mac-ai-switchboard.codex-probe"'* ||
      "${DESIGNATED_REQUIREMENT}" != *"anchor apple generic"* ]]; then
      echo "The Developer ID helper designated requirement is not pinned to its identifier and Apple anchor." >&2
      exit 1
    fi
  fi
fi

echo "Verified sandbox-only Codex probe helper bundle: ${HELPER_APP} (${actual_arches})"
