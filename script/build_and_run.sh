#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="Mac AI Switchboard"
BUNDLE_ID="com.tarunagarwal.mac-ai-switchboard"
PROCESS_PATTERN="(^|/)target/.*/mac-ai-switchboard|Mac AI Switchboard.app/Contents/MacOS/mac-ai-switchboard"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script only runs on macOS." >&2
  exit 1
fi

stop_existing_app() {
  osascript -e "tell application id \"${BUNDLE_ID}\" to quit" >/dev/null 2>&1 || true
  sleep 1
  pkill -f "${PROCESS_PATTERN}" >/dev/null 2>&1 || true
}

run_dev_app() {
  npm run tauri -- dev
}

stream_process_logs() {
  /usr/bin/log stream --info --style compact --predicate "process == \"mac-ai-switchboard\" OR process == \"${APP_NAME}\""
}

stream_telemetry_logs() {
  /usr/bin/log stream --info --style compact --predicate "subsystem == \"${BUNDLE_ID}\""
}

verify_launch() {
  local pid=""
  for _ in {1..45}; do
    pid="$(pgrep -f "${PROCESS_PATTERN}" | head -n 1 || true)"
    if [[ -n "${pid}" ]]; then
      echo "${APP_NAME} is running with pid ${pid}."
      return 0
    fi
    sleep 1
  done

  echo "${APP_NAME} did not start within 45 seconds." >&2
  return 1
}

case "${MODE}" in
  run)
    stop_existing_app
    run_dev_app
    ;;
  --debug|debug)
    stop_existing_app
    RUST_BACKTRACE=1 run_dev_app
    ;;
  --logs|logs)
    stop_existing_app
    run_dev_app &
    verify_launch
    stream_process_logs
    ;;
  --telemetry|telemetry)
    stop_existing_app
    run_dev_app &
    verify_launch
    stream_telemetry_logs
    ;;
  --verify|verify)
    stop_existing_app
    run_dev_app &
    dev_pid=$!
    if ! verify_launch; then
      kill "${dev_pid}" >/dev/null 2>&1 || true
      exit 1
    fi
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
