#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

allowed_prefixes=(
  "fixtures/"
  "tests/fixtures/"
)

forbidden_patterns=(
  "*.db"
  "*.db-wal"
  "*.db-shm"
  "*.sqlite"
  "*.sqlite3"
  "*.sqlite-wal"
  "*.sqlite-shm"
  "*mac-ai-switchboard-audit.md"
  ".env"
  ".env.local"
  ".DS_Store"
  "*.log"
)

is_allowed_fixture() {
  local path="$1"

  for prefix in "${allowed_prefixes[@]}"; do
    if [[ "${path}" == "${prefix}"* ]]; then
      return 0
    fi
  done

  return 1
}

failures=()

for pattern in "${forbidden_patterns[@]}"; do
  while IFS= read -r path; do
    [[ -z "${path}" ]] && continue

    if is_allowed_fixture "${path}"; then
      continue
    fi

    failures+=("${path}")
  done < <(git ls-files -- "${pattern}")
done

if (( ${#failures[@]} > 0 )); then
  printf 'Unexpected local artifact files are tracked:\n' >&2
  printf '  %s\n' "${failures[@]}" >&2
  printf '\nMove fixtures under tests/fixtures/ or remove local artifacts from Git.\n' >&2
  exit 1
fi

echo "No tracked local artifacts found."
