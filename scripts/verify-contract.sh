#!/usr/bin/env bash
# =============================================================================
# scripts/verify-contract.sh – Verify deployed Soroban contracts against source code
#
# Usage:
#   ./scripts/verify-contract.sh --contract-id <id> --source <path> [--network testnet|mainnet|futurenet] [--build]
#
# Options:
#   --contract-id <id>    Contract ID to verify (required)
#   --source <path>       Path to contract source directory containing Cargo.toml (required)
#   --network <net>       Target network: testnet | mainnet | futurenet (default: testnet)
#   --build               Rebuild the contract before verification
#   --help                Print this help and exit
#
# Requirements:
#   - Rust toolchain with wasm32-unknown-unknown target
#   - Stellar CLI ≥ v21 (https://developers.stellar.org/docs/tools/cli)
#   - Contract must be deployed on the specified network
#
# The script verifies that the deployed contract bytecode matches the source code
# by recompiling with the same settings and comparing.
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Resolve repository root
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
STELLAR_LEND_DIR="$REPO_ROOT/stellar-lend"

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
NETWORK="${STELLAR_NETWORK:-testnet}"
CONTRACT_ID=""
SOURCE_PATH=""
DO_BUILD=false

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --contract-id) CONTRACT_ID="$2"; shift 2 ;;
    --source) SOURCE_PATH="$2"; shift 2 ;;
    --network) NETWORK="$2"; shift 2 ;;
    --build) DO_BUILD=true; shift ;;
    --help)
      sed -n '2,25p' "$0"   # print the header comment
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

# ---------------------------------------------------------------------------
# Validate required arguments
# ---------------------------------------------------------------------------
if [[ -z "$CONTRACT_ID" ]]; then
  echo "ERROR: --contract-id is required" >&2
  exit 1
fi

if [[ -z "$SOURCE_PATH" ]]; then
  echo "ERROR: --source is required" >&2
  exit 1
fi

if [[ ! -d "$SOURCE_PATH" ]]; then
  echo "ERROR: Source path does not exist: $SOURCE_PATH" >&2
  exit 1
fi

if [[ ! -f "$SOURCE_PATH/Cargo.toml" ]]; then
  echo "ERROR: Source path must contain Cargo.toml: $SOURCE_PATH" >&2
  exit 1
fi

echo "======================================================================"
echo " StellarLend contract verification"
echo " Contract ID    : $CONTRACT_ID"
echo " Source path    : $SOURCE_PATH"
echo " Network        : $NETWORK"
echo "======================================================================"

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------
command -v stellar >/dev/null 2>&1 || { echo "ERROR: stellar CLI not found. Install from https://developers.stellar.org/docs/tools/cli" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust from https://rustup.rs" >&2; exit 1; }

# Ensure wasm target is present
rustup target add wasm32-unknown-unknown --quiet

# ---------------------------------------------------------------------------
# Optional build step
# ---------------------------------------------------------------------------
if $DO_BUILD; then
  echo ""
  echo ">>> Building contract for verification"
  (cd "$SOURCE_PATH" && stellar contract build --verbose)
fi

# ---------------------------------------------------------------------------
# Locate WASM file
# ---------------------------------------------------------------------------
# Find the WASM file (prefer optimized version)
WASM_FILE=$(find "$SOURCE_PATH/target/wasm32-unknown-unknown/release" -name "*.optimized.wasm" | head -1)

if [[ -z "$WASM_FILE" ]]; then
  WASM_FILE=$(find "$SOURCE_PATH/target/wasm32-unknown-unknown/release" -name "*.wasm" | head -1)
fi

if [[ -z "$WASM_FILE" || ! -f "$WASM_FILE" ]]; then
  echo "ERROR: WASM file not found in $SOURCE_PATH/target/wasm32-unknown-unknown/release" >&2
  echo "       Run with --build flag or build the contract first." >&2
  exit 1
fi

echo ""
echo ">>> Verifying contract"
echo "    Contract ID: $CONTRACT_ID"
echo "    WASM file  : $WASM_FILE"
echo "    Network    : $NETWORK"

# ---------------------------------------------------------------------------
# Run verification
# ---------------------------------------------------------------------------
rpc_args=()
if [[ -n "${STELLAR_RPC_URL:-}" ]]; then
  rpc_args=(--rpc-url "$STELLAR_RPC_URL")
fi

if stellar contract verify \
  --id "$CONTRACT_ID" \
  --wasm "$WASM_FILE" \
  --network "$NETWORK" \
  "${rpc_args[@]+"${rpc_args[@]}"}"; then

  echo ""
  echo "======================================================================"
  echo " ✅ VERIFICATION SUCCESSFUL"
  echo " Contract $CONTRACT_ID matches source code"
  echo "======================================================================"
else
  echo ""
  echo "======================================================================"
  echo " ❌ VERIFICATION FAILED"
  echo " Contract $CONTRACT_ID does not match source code"
  echo "======================================================================"
  exit 1
fi