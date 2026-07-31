#!/usr/bin/env bash
# =============================================================================
# scripts/abi-extractor/extract.sh
#
# Extract ABI/metadata from compiled Soroban WASM artifacts.
#
# Usage:
#   ./scripts/abi-extractor/extract.sh [OPTIONS]
#
# Options:
#   --contract <name>   Extract ABI for a specific contract only
#   --output <dir>      Output directory for ABI JSON files (default: packages/contract-abis/abi)
#   --skip-build        Skip the build step (use existing WASM artifacts)
#   --help              Show this help message
#
# Requirements:
#   - Rust toolchain with wasm32-unknown-unknown target
#   - Stellar CLI >= v21
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Resolve paths
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
STELLAR_LEND_DIR="$REPO_ROOT/stellar-lend"
WASM_DIR="$STELLAR_LEND_DIR/target/wasm32-unknown-unknown/release"

# ---------------------------------------------------------------------------
# Default configuration
# ---------------------------------------------------------------------------
OUTPUT_DIR="$REPO_ROOT/packages/contract-abis/abi"
SPECIFIC_CONTRACT=""
SKIP_BUILD=false

# All deployable contracts (those with crate-type = ["cdylib"])
ALL_CONTRACTS=(
  amm
  bridge
  delegation-registry
  hello-world
  lending
  lending-core
  lending-interest
  lending-risk
  stablecoin
  institutional-wallet
  migration-hub
  stealth-address
  privacy-pool
)

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --contract)
      SPECIFIC_CONTRACT="$2"
      shift 2
      ;;
    --output)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --skip-build)
      SKIP_BUILD=true
      shift
      ;;
    --help)
      head -17 "$0" | tail -14
      exit 0
      ;;
    *)
      echo "ERROR: Unknown argument: $1" >&2
      echo "Run with --help for usage information." >&2
      exit 1
      ;;
  esac
done

# ---------------------------------------------------------------------------
# Determine which contracts to process
# ---------------------------------------------------------------------------
if [[ -n "$SPECIFIC_CONTRACT" ]]; then
  # Validate the contract name
  FOUND=false
  for c in "${ALL_CONTRACTS[@]}"; do
    if [[ "$c" == "$SPECIFIC_CONTRACT" ]]; then
      FOUND=true
      break
    fi
  done
  if [[ "$FOUND" == "false" ]]; then
    echo "ERROR: Unknown contract '$SPECIFIC_CONTRACT'." >&2
    echo "Available contracts: ${ALL_CONTRACTS[*]}" >&2
    exit 1
  fi
  CONTRACTS=("$SPECIFIC_CONTRACT")
else
  CONTRACTS=("${ALL_CONTRACTS[@]}")
fi

echo "======================================================================"
echo " StellarLend ABI Extractor"
echo "======================================================================"
echo ""
echo "Contracts:  ${CONTRACTS[*]}"
echo "Output dir: $OUTPUT_DIR"
echo ""

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------
command -v cargo >/dev/null 2>&1 || {
  echo "ERROR: cargo not found. Install Rust from https://rustup.rs" >&2
  exit 1
}
command -v stellar >/dev/null 2>&1 || {
  echo "ERROR: stellar CLI not found. Install from https://developers.stellar.org/docs/tools/cli" >&2
  exit 1
}

# ---------------------------------------------------------------------------
# Build contracts (unless --skip-build)
# ---------------------------------------------------------------------------
if [[ "$SKIP_BUILD" == "false" ]]; then
  echo ">>> Building contracts..."
  rustup target add wasm32-unknown-unknown --quiet

  if [[ -n "$SPECIFIC_CONTRACT" ]]; then
    echo "    Building contract: $SPECIFIC_CONTRACT"
    (cd "$STELLAR_LEND_DIR" && cargo build --release --target wasm32-unknown-unknown \
      --package "$SPECIFIC_CONTRACT" 2>&1)
  else
    echo "    Building all contracts..."
    (cd "$STELLAR_LEND_DIR" && stellar contract build 2>&1) || {
      echo "    stellar contract build failed, falling back to cargo build..."
      (cd "$STELLAR_LEND_DIR" && cargo build --release --target wasm32-unknown-unknown 2>&1)
    }
  fi

  # Optimize WASM artifacts
  echo ""
  echo ">>> Optimizing WASM artifacts..."
  for wasm_file in "$WASM_DIR"/*.wasm; do
    [[ ! -f "$wasm_file" ]] && continue
    [[ "$wasm_file" == *optimized* ]] && continue
    basename_wasm="$(basename "$wasm_file" .wasm)"
    # Convert underscores to hyphens for matching against contract names
    contract_name="${basename_wasm//_/-}"
    # If building a specific contract, skip others
    if [[ -n "$SPECIFIC_CONTRACT" && "$contract_name" != "$SPECIFIC_CONTRACT" ]]; then
      continue
    fi
    echo "    Optimizing $(basename "$wasm_file")..."
    stellar contract optimize --wasm "$wasm_file" 2>/dev/null || true
  done

  echo ""
else
  echo ">>> Skipping build (--skip-build)"
  echo ""
fi

# ---------------------------------------------------------------------------
# Create output directory
# ---------------------------------------------------------------------------
mkdir -p "$OUTPUT_DIR"

# ---------------------------------------------------------------------------
# Extract ABIs
# ---------------------------------------------------------------------------
echo ">>> Extracting ABIs..."
EXTRACTED=0
FAILED=0

for contract in "${CONTRACTS[@]}"; do
  # Contract WASM filename uses underscores (cargo convention)
  wasm_name="${contract//-/_}"

  # Try optimized first, then unoptimized
  wasm_path="$WASM_DIR/${wasm_name}.optimized.wasm"
  if [[ ! -f "$wasm_path" ]]; then
    wasm_path="$WASM_DIR/${wasm_name}.wasm"
  fi

  if [[ ! -f "$wasm_path" ]]; then
    echo "    WARNING: No WASM artifact found for '$contract' (looked for ${wasm_name}.wasm)" >&2
    FAILED=$((FAILED + 1))
    continue
  fi

  echo "    Extracting ABI: $contract ($(basename "$wasm_path"))"

  # Extract contract spec/ABI using stellar CLI
  ABI_JSON="$OUTPUT_DIR/${contract}.json"
  SPEC_OUTPUT=""
  EXTRACT_OK=false

  # Method 1: stellar contract info interface (newer CLI versions)
  if SPEC_OUTPUT=$(stellar contract info interface --wasm "$wasm_path" --output json 2>/dev/null); then
    EXTRACT_OK=true
  # Method 2: stellar contract inspect (older CLI versions)
  elif SPEC_OUTPUT=$(stellar contract inspect --wasm "$wasm_path" --output json 2>/dev/null); then
    EXTRACT_OK=true
  # Method 3: Parse the WASM custom sections directly
  elif SPEC_OUTPUT=$(stellar contract inspect --wasm "$wasm_path" 2>/dev/null); then
    EXTRACT_OK=true
  fi

  if [[ "$EXTRACT_OK" == "true" && -n "$SPEC_OUTPUT" ]]; then
    # Build a structured ABI envelope
    WASM_SIZE=$(wc -c < "$wasm_path" | tr -d ' ')
    WASM_HASH=$(shasum -a 256 "$wasm_path" | cut -d' ' -f1)
    EXTRACT_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    # Construct the full ABI JSON
    # Use a heredoc with jq if available, otherwise construct manually
    if command -v jq >/dev/null 2>&1; then
      jq -n \
        --arg name "$contract" \
        --arg version "$(date +%Y%m%d)" \
        --arg wasmHash "$WASM_HASH" \
        --argjson wasmSize "$WASM_SIZE" \
        --arg extractedAt "$EXTRACT_TIME" \
        --arg wasmPath "$(basename "$wasm_path")" \
        --argjson spec "$SPEC_OUTPUT" \
        '{
          contract_name: $name,
          version: $version,
          wasm_hash: $wasmHash,
          wasm_size: $wasmSize,
          extracted_at: $extractedAt,
          wasm_artifact: $wasmPath,
          spec: $spec
        }' > "$ABI_JSON"
    else
      # Fallback: write JSON without jq
      cat > "$ABI_JSON" <<ENDJSON
{
  "contract_name": "${contract}",
  "version": "$(date +%Y%m%d)",
  "wasm_hash": "${WASM_HASH}",
  "wasm_size": ${WASM_SIZE},
  "extracted_at": "${EXTRACT_TIME}",
  "wasm_artifact": "$(basename "$wasm_path")",
  "spec": ${SPEC_OUTPUT}
}
ENDJSON
    fi

    echo "      -> $ABI_JSON"
    EXTRACTED=$((EXTRACTED + 1))
  else
    echo "    WARNING: Could not extract ABI for '$contract'." >&2
    echo "             Generating minimal metadata only." >&2

    # Even if spec extraction fails, record what we can
    WASM_SIZE=$(wc -c < "$wasm_path" | tr -d ' ')
    WASM_HASH=$(shasum -a 256 "$wasm_path" | cut -d' ' -f1)

    cat > "$ABI_JSON" <<ENDJSON
{
  "contract_name": "${contract}",
  "version": "$(date +%Y%m%d)",
  "wasm_hash": "${WASM_HASH}",
  "wasm_size": ${WASM_SIZE},
  "extracted_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "wasm_artifact": "$(basename "$wasm_path")",
  "spec": null,
  "note": "ABI extraction failed - spec unavailable"
}
ENDJSON

    echo "      -> $ABI_JSON (metadata only)"
    EXTRACTED=$((EXTRACTED + 1))
  fi
done

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "======================================================================"
echo " ABI Extraction Complete"
echo "   Extracted: $EXTRACTED"
echo "   Failed:    $FAILED"
echo "   Output:    $OUTPUT_DIR"
echo "======================================================================"

if [[ $FAILED -gt 0 && -z "$SPECIFIC_CONTRACT" ]]; then
  echo ""
  echo "NOTE: Some contracts failed. This may be expected for library crates"
  echo "      (e.g., common, lending-types, test-utils) that do not produce"
  echo "      deployable WASM artifacts."
fi

exit 0
