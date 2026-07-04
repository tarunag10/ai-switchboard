set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

echo "Checking deployment readiness..."
npm run check:deployment

echo "Checking release environment..."
node scripts/check-release-env.mjs --strict

echo "Checking release environment placeholder guard..."
npm run release:env:selftest

echo "Checking installed-app smoke preflight..."
npm run smoke:preflight

echo "Checking repo-memory MCP read-only contract..."
npm run check:repo-memory-mcp

echo "Checking connector registry parity..."
npm run check:connectors

echo "Checking for tracked local artifacts..."
npm run check:local-artifacts

echo "Checking semantic color tokens..."
npm run check:colors

echo "Checking governance docs..."
npm run check:governance

echo "Checking local-only network boundary..."
npm run check:local-only-network

echo "Building production frontend..."
npm run build

echo "Checking local-free build privacy..."
npm run check:local-build-privacy

echo "Running frontend coverage..."
npm run test:frontend

echo "Checking Rust formatting..."
npm run fmt:desktop

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
