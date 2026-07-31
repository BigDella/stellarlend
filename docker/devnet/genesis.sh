#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# genesis.sh -- Create and fund initial accounts on the local Soroban devnet
#
# This script is called by init-devnet.sh after the node is healthy.
# It reads account definitions from seed-data/accounts.json and creates
# each account via friendbot (local network) or soroban keys generate.
#
# Environment variables (set by docker-compose):
#   SOROBAN_RPC_URL       -- e.g. http://soroban-devnet:8000/soroban/rpc
#   HORIZON_URL           -- e.g. http://soroban-devnet:8000
#   FRIENDBOT_URL         -- e.g. http://soroban-devnet:8000/friendbot
#   NETWORK_PASSPHRASE    -- "Standalone Network ; February 2017"
# ---------------------------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACCOUNTS_FILE="${SCRIPT_DIR}/seed-data/accounts.json"
DEPLOY_DIR="${DEPLOY_DIR:-/deploy}"

SOROBAN_RPC_URL="${SOROBAN_RPC_URL:-http://soroban-devnet:8000/soroban/rpc}"
HORIZON_URL="${HORIZON_URL:-http://soroban-devnet:8000}"
FRIENDBOT_URL="${FRIENDBOT_URL:-http://soroban-devnet:8000/friendbot}"
NETWORK_PASSPHRASE="${NETWORK_PASSPHRASE:-Standalone Network ; February 2017}"

log() { echo "[genesis] $(date '+%H:%M:%S') $*"; }
err() { echo "[genesis] ERROR: $*" >&2; }

# ---------------------------------------------------------------------------
# Create a single account via soroban keys generate + friendbot funding
# Args: alias
# ---------------------------------------------------------------------------
create_account() {
  local alias="$1"
  log "Creating account: ${alias}"

  # Generate a new keypair for this identity using stellar/soroban CLI
  if stellar keys generate "${alias}" \
       --rpc-url "${SOROBAN_RPC_URL}" \
       --network-passphrase "${NETWORK_PASSPHRASE}" \
       --fund 2>/dev/null; then
    log "  Keypair generated for ${alias}"
  elif soroban keys generate "${alias}" \
       --rpc-url "${SOROBAN_RPC_URL}" \
       --network-passphrase "${NETWORK_PASSPHRASE}" \
       --fund 2>/dev/null; then
    log "  Keypair generated for ${alias} (soroban CLI)"
  else
    # Fallback: use friendbot directly
    log "  CLI key generation unavailable, using friendbot fallback"
    local tmp_keypair
    tmp_keypair=$(stellar keys generate "${alias}" --no-fund 2>/dev/null || soroban keys generate "${alias}" --no-fund 2>/dev/null || true)
    local pub_key
    pub_key=$(stellar keys address "${alias}" 2>/dev/null || soroban keys address "${alias}" 2>/dev/null || echo "")
    if [ -n "${pub_key}" ]; then
      curl -sf "${FRIENDBOT_URL}?addr=${pub_key}" > /dev/null 2>&1 || true
      log "  Funded ${alias} via friendbot: ${pub_key}"
    else
      err "Failed to create account ${alias}"
      return 1
    fi
  fi

  # Retrieve and display the public key
  local address
  address=$(stellar keys address "${alias}" 2>/dev/null || soroban keys address "${alias}" 2>/dev/null || echo "unknown")
  log "  Address: ${address}"

  # Save to deploy directory for later reference
  mkdir -p "${DEPLOY_DIR}/accounts"
  echo "${address}" > "${DEPLOY_DIR}/accounts/${alias}.address"
}

# ---------------------------------------------------------------------------
# Fund an existing account with additional XLM via friendbot
# Args: alias, amount (amount is informational; friendbot gives a fixed sum)
# ---------------------------------------------------------------------------
fund_account() {
  local alias="$1"
  local target_xlm="${2:-10000}"

  local address
  address=$(stellar keys address "${alias}" 2>/dev/null || soroban keys address "${alias}" 2>/dev/null || echo "")
  if [ -z "${address}" ]; then
    err "Cannot fund ${alias}: address not found"
    return 1
  fi

  # On local network friendbot can be called multiple times to add funds
  local rounds=$(( target_xlm / 10000 ))
  [ "${rounds}" -lt 1 ] && rounds=1

  log "Funding ${alias} (${address}) -- ${rounds} friendbot round(s)"
  for i in $(seq 1 "${rounds}"); do
    curl -sf "${FRIENDBOT_URL}?addr=${address}" > /dev/null 2>&1 || true
  done
  log "  Funded ${alias} with ~${target_xlm} XLM"
}

# ---------------------------------------------------------------------------
# Create custom test tokens (wrapped assets)
# ---------------------------------------------------------------------------
create_test_tokens() {
  log "Creating test tokens (USDC, slUSD mock assets)..."

  local admin_addr
  admin_addr=$(stellar keys address "admin" 2>/dev/null || soroban keys address "admin" 2>/dev/null || echo "")

  if [ -z "${admin_addr}" ]; then
    err "Admin account not found, skipping token creation"
    return 0
  fi

  # Deploy a SAC (Stellar Asset Contract) for test USDC
  for token_code in USDC slUSD; do
    log "  Wrapping ${token_code} as Stellar asset..."
    if stellar contract asset deploy \
         --asset "${token_code}:${admin_addr}" \
         --rpc-url "${SOROBAN_RPC_URL}" \
         --network-passphrase "${NETWORK_PASSPHRASE}" \
         --source-account "admin" 2>/dev/null; then
      log "  ${token_code} asset contract deployed"
    elif soroban contract asset deploy \
         --asset "${token_code}:${admin_addr}" \
         --rpc-url "${SOROBAN_RPC_URL}" \
         --network-passphrase "${NETWORK_PASSPHRASE}" \
         --source-account "admin" 2>/dev/null; then
      log "  ${token_code} asset contract deployed (soroban CLI)"
    else
      log "  WARN: Could not deploy ${token_code} asset contract (non-fatal)"
    fi
  done
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  log "=== Genesis: Creating accounts on local devnet ==="
  log "RPC URL:     ${SOROBAN_RPC_URL}"
  log "Horizon URL: ${HORIZON_URL}"
  log "Friendbot:   ${FRIENDBOT_URL}"
  log ""

  # Create accounts in dependency order: admin first, then service accounts,
  # then test users.
  local accounts=("admin" "liquidator" "user1" "user2" "user3" "user4" "user5")

  for acct in "${accounts[@]}"; do
    create_account "${acct}" || true
  done

  log ""
  log "--- Funding accounts ---"

  # Fund admin with extra XLM (needs gas for contract deployments)
  fund_account "admin" 100000
  fund_account "liquidator" 50000

  for i in 1 2 3 4 5; do
    fund_account "user${i}" 10000
  done

  log ""
  log "--- Creating test tokens ---"
  create_test_tokens

  log ""
  log "=== Genesis complete ==="

  # Summary
  log ""
  log "Account summary:"
  for acct in "${accounts[@]}"; do
    local addr
    addr=$(stellar keys address "${acct}" 2>/dev/null || soroban keys address "${acct}" 2>/dev/null || echo "n/a")
    log "  ${acct}: ${addr}"
  done
}

main "$@"
