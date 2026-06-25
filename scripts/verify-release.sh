set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

echo "Checking semantic color tokens..."
npm run check:colors

echo "Building production frontend..."
npm run build

echo "Running frontend coverage..."
npm run test:coverage

echo "Running desktop tests..."
# Prefer nextest: its slow-timeout/terminate-after (.config/nextest.toml) kills a
# hung test in minutes instead of letting it stall the whole release job. Fall
# back to plain cargo test where nextest isn't installed (local dev machines).
if command -v cargo-nextest >/dev/null 2>&1; then
  cargo nextest run --manifest-path src-tauri/Cargo.toml
else
  npm run test:desktop
fi

echo "Release checks passed."
