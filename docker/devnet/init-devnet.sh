#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# init-devnet.sh -- Bootstrap the StellarLend local Soroban devnet
#
# Sequence:
#   1. Wait for the Soroban devnet node to be healthy
#   2. Run genesis.sh to create & fund accounts
#   3. Build and deploy all 16 contracts
#   4. Initialize contracts with default parameters
#   5. Set up oracle price feeds with mock data
#   6. Seed sample positions, loans, and liquidation data
#
# Environment variables (set by docker-compose):
#   SOROBAN_RPC_URL, HORIZON_URL, FRIENDBOT_URL,
#   NETWORK_PASSPHRASE, API_URL, ORACLE_URL
# ---------------------------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="${DEPLOY_DIR:-/deploy}"
CONTRACTS_DIR="${CONTRACTS_DIR:-/contracts}"

SOROBAN_RPC_URL="${SOROBAN_RPC_URL:-http://soroban-devnet:8000/soroban/rpc}"
HORIZON_URL="${HORIZON_URL:-http://soroban-devnet:8000}"
FRIENDBOT_URL="${FRIENDBOT_URL:-http://soroban-devnet:8000/friendbot}"
NETWORK_PASSPHRASE="${NETWORK_PASSPHRASE:-Standalone Network ; February 2017}"
API_URL="${API_URL:-http://api:3000}"
ORACLE_URL="${ORACLE_URL:-http://oracle:4000}"
HEALTH_TIMEOUT="${HEALTH_TIMEOUT:-120}"

SEED_DIR="${SCRIPT_DIR}/seed-data"
CONTRACTS_CONFIG="${SEED_DIR}/contracts.json"

log() { echo "[init-devnet] $(date '+%H:%M:%S') $*"; }
err() { echo "[init-devnet] ERROR: $*" >&2; }

# ---------------------------------------------------------------------------
# Step 1: Wait for the Soroban devnet node
# ---------------------------------------------------------------------------
wait_for_devnet() {
  log "Waiting for Soroban devnet to be ready (timeout: ${HEALTH_TIMEOUT}s)..."
  local elapsed=0
  while [ "${elapsed}" -lt "${HEALTH_TIMEOUT}" ]; do
    if curl -sf "${HORIZON_URL}/health" > /dev/null 2>&1; then
      log "Soroban devnet is healthy!"
      return 0
    fi
    sleep 2
    elapsed=$((elapsed + 2))
    if [ $((elapsed % 10)) -eq 0 ]; then
      log "  Still waiting... (${elapsed}s / ${HEALTH_TIMEOUT}s)"
    fi
  done
  err "Soroban devnet did not become healthy within ${HEALTH_TIMEOUT}s"
  return 1
}

# ---------------------------------------------------------------------------
# Step 2: Run genesis (account creation & funding)
# ---------------------------------------------------------------------------
run_genesis() {
  log "=== Step 2: Running genesis ==="
  if [ -f "${SCRIPT_DIR}/genesis.sh" ]; then
    bash "${SCRIPT_DIR}/genesis.sh"
  else
    err "genesis.sh not found at ${SCRIPT_DIR}/genesis.sh"
    return 1
  fi
}

# ---------------------------------------------------------------------------
# Step 3: Build and deploy contracts
# ---------------------------------------------------------------------------
deploy_contracts() {
  log "=== Step 3: Deploying contracts ==="

  if [ ! -f "${CONTRACTS_CONFIG}" ]; then
    err "contracts.json not found at ${CONTRACTS_CONFIG}"
    return 1
  fi

  local admin_addr
  admin_addr=$(stellar keys address "admin" 2>/dev/null || soroban keys address "admin" 2>/dev/null || echo "")
  if [ -z "${admin_addr}" ]; then
    err "Admin account not available"
    return 1
  fi

  mkdir -p "${DEPLOY_DIR}/contracts"

  # Parse the deploy order from contracts.json
  # Using a simple approach compatible with busybox/alpine
  local deploy_order
  if command -v jq &> /dev/null; then
    deploy_order=$(jq -r '.deploy_order[]' "${CONTRACTS_CONFIG}")
  else
    # Fallback: hardcoded order matching contracts.json
    deploy_order="common lending-types lending-interest lending-risk lending-core lending stablecoin amm bridge delegation-registry institutional-wallet migration-hub stealth-address privacy-pool test-utils hello-world"
  fi

  for contract_name in ${deploy_order}; do
    deploy_single_contract "${contract_name}" "${admin_addr}"
  done

  log "All contracts deployed."
}

deploy_single_contract() {
  local name="$1"
  local admin_addr="$2"

  # Convert contract-name to wasm filename (hyphens -> underscores)
  local wasm_name
  wasm_name=$(echo "${name}" | tr '-' '_')
  local wasm_path="${CONTRACTS_DIR}/target/wasm32-unknown-unknown/release/${wasm_name}.wasm"

  log "Deploying ${name}..."

  # Check if WASM exists; if not, try to build
  if [ ! -f "${wasm_path}" ]; then
    log "  WASM not found at ${wasm_path}, attempting build..."
    if [ -d "${CONTRACTS_DIR}/${name}" ]; then
      (cd "${CONTRACTS_DIR}" && \
        stellar contract build --manifest-path "${name}/Cargo.toml" 2>/dev/null || \
        soroban contract build --manifest-path "${name}/Cargo.toml" 2>/dev/null || \
        cargo build --manifest-path "${name}/Cargo.toml" --target wasm32-unknown-unknown --release 2>/dev/null || \
        true)
    fi
  fi

  if [ ! -f "${wasm_path}" ]; then
    log "  WARN: Cannot build/find WASM for ${name}, skipping deployment"
    echo "SKIPPED" > "${DEPLOY_DIR}/contracts/${name}.status"
    return 0
  fi

  # Deploy the contract
  local contract_id
  contract_id=$(stellar contract deploy \
    --wasm "${wasm_path}" \
    --rpc-url "${SOROBAN_RPC_URL}" \
    --network-passphrase "${NETWORK_PASSPHRASE}" \
    --source-account "admin" 2>/dev/null || \
  soroban contract deploy \
    --wasm "${wasm_path}" \
    --rpc-url "${SOROBAN_RPC_URL}" \
    --network-passphrase "${NETWORK_PASSPHRASE}" \
    --source-account "admin" 2>/dev/null || echo "")

  if [ -z "${contract_id}" ]; then
    log "  WARN: Deployment failed for ${name}"
    echo "FAILED" > "${DEPLOY_DIR}/contracts/${name}.status"
    return 0
  fi

  log "  Deployed ${name}: ${contract_id}"
  echo "${contract_id}" > "${DEPLOY_DIR}/contracts/${name}.id"
  echo "DEPLOYED" > "${DEPLOY_DIR}/contracts/${name}.status"
}

# ---------------------------------------------------------------------------
# Step 4: Initialize contracts with default parameters
# ---------------------------------------------------------------------------
initialize_contracts() {
  log "=== Step 4: Initializing contracts ==="

  local admin_addr
  admin_addr=$(stellar keys address "admin" 2>/dev/null || soroban keys address "admin" 2>/dev/null || echo "")

  for contract_file in "${DEPLOY_DIR}"/contracts/*.id; do
    [ -f "${contract_file}" ] || continue
    local name
    name=$(basename "${contract_file}" .id)
    local contract_id
    contract_id=$(cat "${contract_file}")

    initialize_single_contract "${name}" "${contract_id}" "${admin_addr}"
  done

  log "Contract initialization complete."
}

initialize_single_contract() {
  local name="$1"
  local contract_id="$2"
  local admin_addr="$3"

  log "Initializing ${name} (${contract_id})..."

  case "${name}" in
    lending-interest)
      invoke_contract "${contract_id}" "initialize" \
        --arg "${admin_addr}" --arg 200 --arg 500 --arg 2000 --arg 8000
      ;;
    lending-risk)
      invoke_contract "${contract_id}" "initialize" \
        --arg "${admin_addr}" --arg 15000 --arg 12500 --arg 500
      ;;
    lending-core)
      local interest_id risk_id
      interest_id=$(cat "${DEPLOY_DIR}/contracts/lending-interest.id" 2>/dev/null || echo "")
      risk_id=$(cat "${DEPLOY_DIR}/contracts/lending-risk.id" 2>/dev/null || echo "")
      if [ -n "${interest_id}" ] && [ -n "${risk_id}" ]; then
        invoke_contract "${contract_id}" "initialize" \
          --arg "${admin_addr}" --arg "${interest_id}" --arg "${risk_id}"
      else
        log "  WARN: Dependencies not found for lending-core, skipping init"
      fi
      ;;
    lending)
      local core_id
      core_id=$(cat "${DEPLOY_DIR}/contracts/lending-core.id" 2>/dev/null || echo "")
      if [ -n "${core_id}" ]; then
        invoke_contract "${contract_id}" "initialize" \
          --arg "${admin_addr}" --arg "${core_id}" --arg "mock-oracle" --arg 9 --arg 100
      else
        log "  WARN: lending-core not found, skipping lending init"
      fi
      ;;
    stablecoin)
      invoke_contract "${contract_id}" "initialize" \
        --arg "${admin_addr}" --arg "StellarLend USD" --arg "slUSD" --arg 7
      ;;
    amm)
      invoke_contract "${contract_id}" "initialize" \
        --arg "${admin_addr}" --arg 30 --arg 1667
      ;;
    bridge)
      invoke_contract "${contract_id}" "initialize" \
        --arg "${admin_addr}" --arg 1 --arg 50
      ;;
    delegation-registry)
      invoke_contract "${contract_id}" "initialize" --arg "${admin_addr}"
      ;;
    institutional-wallet)
      invoke_contract "${contract_id}" "initialize" \
        --arg "${admin_addr}" --arg 2 --arg 1000000000000
      ;;
    migration-hub)
      invoke_contract "${contract_id}" "initialize" --arg "${admin_addr}"
      ;;
    stealth-address)
      invoke_contract "${contract_id}" "initialize" --arg "${admin_addr}"
      ;;
    privacy-pool)
      invoke_contract "${contract_id}" "initialize" --arg "${admin_addr}" --arg 10
      ;;
    hello-world)
      invoke_contract "${contract_id}" "hello" --arg "StellarLend"
      ;;
    common|lending-types|test-utils)
      log "  No initialization needed for ${name}"
      ;;
    *)
      log "  WARN: Unknown contract ${name}, skipping init"
      ;;
  esac
}

invoke_contract() {
  local contract_id="$1"
  local fn_name="$2"
  shift 2

  # Build the argument list
  local args=()
  while [ $# -gt 0 ]; do
    case "$1" in
      --arg) args+=("--arg" "$2"); shift 2 ;;
      *) shift ;;
    esac
  done

  stellar contract invoke \
    --id "${contract_id}" \
    --rpc-url "${SOROBAN_RPC_URL}" \
    --network-passphrase "${NETWORK_PASSPHRASE}" \
    --source-account "admin" \
    -- "${fn_name}" "${args[@]}" 2>/dev/null || \
  soroban contract invoke \
    --id "${contract_id}" \
    --rpc-url "${SOROBAN_RPC_URL}" \
    --network-passphrase "${NETWORK_PASSPHRASE}" \
    --source-account "admin" \
    -- "${fn_name}" "${args[@]}" 2>/dev/null || \
  log "  WARN: Failed to invoke ${fn_name} on ${contract_id}"
}

# ---------------------------------------------------------------------------
# Step 5: Set up oracle price feeds
# ---------------------------------------------------------------------------
setup_oracle_feeds() {
  log "=== Step 5: Setting up oracle price feeds ==="

  local oracle_feeds="${SEED_DIR}/oracle-feeds.json"
  if [ ! -f "${oracle_feeds}" ]; then
    log "  WARN: oracle-feeds.json not found, skipping"
    return 0
  fi

  # Wait for oracle service to be available
  local attempts=0
  while [ "${attempts}" -lt 30 ]; do
    if curl -sf "${ORACLE_URL}/health" > /dev/null 2>&1; then
      log "Oracle service is ready"
      break
    fi
    sleep 2
    attempts=$((attempts + 1))
  done

  if [ "${attempts}" -ge 30 ]; then
    log "  WARN: Oracle service not available, skipping feed setup"
    return 0
  fi

  # Push mock price feeds to the oracle service
  if command -v jq &> /dev/null; then
    local feeds
    feeds=$(jq -r '.feeds | keys[]' "${oracle_feeds}")
    for feed in ${feeds}; do
      local price base quote
      price=$(jq -r ".feeds.\"${feed}\".price" "${oracle_feeds}")
      base=$(jq -r ".feeds.\"${feed}\".base" "${oracle_feeds}")
      quote=$(jq -r ".feeds.\"${feed}\".quote" "${oracle_feeds}")

      log "  Setting price feed: ${base}/${quote} = ${price}"
      curl -sf -X POST "${ORACLE_URL}/feeds" \
        -H "Content-Type: application/json" \
        -d "{\"base\":\"${base}\",\"quote\":\"${quote}\",\"price\":\"${price}\"}" \
        > /dev/null 2>&1 || \
      log "    WARN: Could not set ${feed} feed (oracle may not support POST /feeds)"
    done
  else
    # Without jq, POST the entire feeds file
    curl -sf -X POST "${ORACLE_URL}/feeds/bulk" \
      -H "Content-Type: application/json" \
      -d @"${oracle_feeds}" \
      > /dev/null 2>&1 || \
    log "  WARN: Bulk feed upload failed (non-fatal)"
  fi

  log "Oracle price feed setup complete."
}

# ---------------------------------------------------------------------------
# Step 6: Seed sample data
# ---------------------------------------------------------------------------
seed_sample_data() {
  log "=== Step 6: Seeding sample data ==="

  local positions_file="${SEED_DIR}/sample-positions.json"
  if [ ! -f "${positions_file}" ]; then
    log "  WARN: sample-positions.json not found, skipping"
    return 0
  fi

  # Wait for the API to be available
  local attempts=0
  while [ "${attempts}" -lt 30 ]; do
    if curl -sf "${API_URL}/api/health" > /dev/null 2>&1; then
      log "API service is ready"
      break
    fi
    sleep 2
    attempts=$((attempts + 1))
  done

  if [ "${attempts}" -ge 30 ]; then
    log "  WARN: API service not available, skipping data seeding"
    return 0
  fi

  # Seed deposits
  log "  Seeding sample deposits..."
  if command -v jq &> /dev/null; then
    local deposits
    deposits=$(jq -c '.positions.deposits[]' "${positions_file}" 2>/dev/null)
    echo "${deposits}" | while IFS= read -r deposit; do
      curl -sf -X POST "${API_URL}/api/seed/deposit" \
        -H "Content-Type: application/json" \
        -d "${deposit}" > /dev/null 2>&1 || true
    done
  fi

  # Seed active loans
  log "  Seeding active loans..."
  if command -v jq &> /dev/null; then
    local loans
    loans=$(jq -c '.positions.active_loans[]' "${positions_file}" 2>/dev/null)
    echo "${loans}" | while IFS= read -r loan; do
      curl -sf -X POST "${API_URL}/api/seed/loan" \
        -H "Content-Type: application/json" \
        -d "${loan}" > /dev/null 2>&1 || true
    done
  fi

  # Seed pending liquidations
  log "  Seeding pending liquidations..."
  if command -v jq &> /dev/null; then
    local liquidations
    liquidations=$(jq -c '.positions.pending_liquidations[]' "${positions_file}" 2>/dev/null)
    echo "${liquidations}" | while IFS= read -r liq; do
      curl -sf -X POST "${API_URL}/api/seed/liquidation" \
        -H "Content-Type: application/json" \
        -d "${liq}" > /dev/null 2>&1 || true
    done
  fi

  log "Sample data seeding complete."
  log "  Alternatively, POST the full file to ${API_URL}/api/seed/bulk"
}

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
print_summary() {
  log ""
  log "============================================================"
  log "  StellarLend Devnet Initialization Complete"
  log "============================================================"
  log ""
  log "Services:"
  log "  Soroban RPC:  ${SOROBAN_RPC_URL}"
  log "  Horizon:      ${HORIZON_URL}"
  log "  Friendbot:    ${FRIENDBOT_URL}"
  log "  API:          ${API_URL}"
  log "  Oracle:       ${ORACLE_URL}"
  log ""
  log "Accounts:"
  for acct in admin liquidator user1 user2 user3 user4 user5; do
    local addr
    addr=$(stellar keys address "${acct}" 2>/dev/null || soroban keys address "${acct}" 2>/dev/null || echo "n/a")
    log "  ${acct}: ${addr}"
  done
  log ""
  log "Deployed contracts:"
  for id_file in "${DEPLOY_DIR}"/contracts/*.id; do
    [ -f "${id_file}" ] || continue
    local cname
    cname=$(basename "${id_file}" .id)
    log "  ${cname}: $(cat "${id_file}")"
  done
  log ""
  log "Contract IDs saved to: ${DEPLOY_DIR}/contracts/"
  log "============================================================"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  log "Starting StellarLend devnet initialization..."
  log ""

  # Check for marker to avoid re-initialization
  if [ -f "${DEPLOY_DIR}/.initialized" ]; then
    log "Devnet already initialized (marker found at ${DEPLOY_DIR}/.initialized)"
    log "To re-initialize, run reset-devnet.sh or remove the marker file."
    print_summary
    return 0
  fi

  # Step 1
  wait_for_devnet

  # Step 2
  run_genesis

  # Step 3
  deploy_contracts

  # Step 4
  initialize_contracts

  # Step 5
  setup_oracle_feeds

  # Step 6
  seed_sample_data

  # Mark as initialized
  mkdir -p "${DEPLOY_DIR}"
  date -u > "${DEPLOY_DIR}/.initialized"

  print_summary
}

main "$@"
