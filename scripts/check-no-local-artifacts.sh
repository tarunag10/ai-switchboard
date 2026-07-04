#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

blocked_patterns=(
  "*.db"
  "*.db-wal"
  "*.db-shm"
  "*.sqlite"
  "*.sqlite3"
  "*.sqlite-wal"
  "*.sqlite-shm"
  "*mac-ai-switchboard-audit.md"
  ".env.local"
  ".DS_Store"
  "*.log"
  "console-errors.md"
  "mobile-*.png"
  "graphify-out/**"
)

is_allowed_fixture() {
  case "$1" in
    fixtures/*|tests/fixtures/*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

artifacts=()
while IFS= read -r -d "" path; do
  if ! is_allowed_fixture "${path}"; then
    artifacts+=("${path}")
  fi
done < <(git ls-files -z -- "${blocked_patterns[@]}")

if (( ${#artifacts[@]} > 0 )); then
  {
    echo "Tracked local/generated artifacts are not allowed:"
    printf '  %s\n' "${artifacts[@]}"
    echo
    echo "Move generated proof into docs/dist evidence, ignore it, or put test fixtures under fixtures/."
  } >&2
  exit 1
fi

echo "No tracked local/generated artifacts found."
