#!/usr/bin/env bash
# snapshot.sh — Fork mainnet state into the migration sandbox.
#
# Usage: ./scripts/migration-simulator/snapshot.sh [options]
#
# Reads contract state from the SOURCE_NETWORK via Soroban RPC and writes it
# as a JSON snapshot that simulate.sh can replay against a local Quickstart fork.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONFIG="$REPO_ROOT/environments/migration-sandbox/sandbox.config.json"

SOURCE_NETWORK="${SOURCE_NETWORK:-pubnet}"
SNAPSHOT_DIR="${SNAPSHOT_DIR:-$REPO_ROOT/.migration-sandbox/snapshots}"
LENDING_CONTRACT_ID="${LENDING_CONTRACT_ID:-}"
MIGRATION_HUB_CONTRACT_ID="${MIGRATION_HUB_CONTRACT_ID:-}"
LABEL="${LABEL:-$(date -u +%Y%m%dT%H%M%SZ)}"

usage() {
  cat <<EOF
Usage: snapshot.sh [options]

Options:
  --source-network <pubnet|testnet|futurenet>  Network to snapshot (default: pubnet)
  --snapshot-dir <path>                        Directory to write snapshot (default: .migration-sandbox/snapshots)
  --lending-contract-id <id>                   Lending contract ID to snapshot
  --migration-hub-id <id>                      MigrationHub contract ID to snapshot
  --label <label>                              Snapshot label/tag (default: ISO timestamp)
  --help                                       Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-network)      SOURCE_NETWORK="$2"; shift 2 ;;
    --snapshot-dir)        SNAPSHOT_DIR="$2"; shift 2 ;;
    --lending-contract-id) LENDING_CONTRACT_ID="$2"; shift 2 ;;
    --migration-hub-id)    MIGRATION_HUB_CONTRACT_ID="$2"; shift 2 ;;
    --label)               LABEL="$2"; shift 2 ;;
    --help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

command -v stellar >/dev/null 2>&1 || { echo "ERROR: stellar CLI not found" >&2; exit 1; }
command -v node    >/dev/null 2>&1 || { echo "ERROR: node not found" >&2; exit 1; }

if [[ -z "$LENDING_CONTRACT_ID" ]]; then
  echo "ERROR: --lending-contract-id is required (or set LENDING_CONTRACT_ID)" >&2
  exit 1
fi

mkdir -p "$SNAPSHOT_DIR"
SNAPSHOT_FILE="$SNAPSHOT_DIR/snapshot-${LABEL}.json"

echo ">>> Snapshotting $SOURCE_NETWORK state for contract $LENDING_CONTRACT_ID"

# Query key protocol state from the source network
query_contract() {
  local contract_id="$1" fn="$2"
  stellar contract invoke \
    --id "$contract_id" \
    --network "$SOURCE_NETWORK" \
    -- "$fn" 2>/dev/null || echo "null"
}

PROTOCOL_PARAMS="$(query_contract "$LENDING_CONTRACT_ID" get_protocol_params)"
RISK_CONFIG="$(query_contract "$LENDING_CONTRACT_ID" get_risk_config)"
SYSTEM_STATS="$(query_contract "$LENDING_CONTRACT_ID" get_system_stats)"

MIGRATION_HUB_SECTION="null"
if [[ -n "$MIGRATION_HUB_CONTRACT_ID" ]]; then
  MIGRATION_HUB_SECTION="$(query_contract "$MIGRATION_HUB_CONTRACT_ID" get_analytics 2>/dev/null || echo 'null')"
fi

# Write snapshot JSON
node - <<NODE
const fs = require('fs');
const snapshot = {
  label: ${LABEL@Q},
  timestamp: new Date().toISOString(),
  sourceNetwork: ${SOURCE_NETWORK@Q},
  lendingContractId: ${LENDING_CONTRACT_ID@Q},
  migrationHubContractId: ${MIGRATION_HUB_CONTRACT_ID:+"$MIGRATION_HUB_CONTRACT_ID"} || null,
  state: {
    protocolParams: parseOrNull(${PROTOCOL_PARAMS@Q}),
    riskConfig:     parseOrNull(${RISK_CONFIG@Q}),
    systemStats:    parseOrNull(${SYSTEM_STATS@Q}),
    migrationHub:   parseOrNull(${MIGRATION_HUB_SECTION@Q}),
  },
};
function parseOrNull(s) { try { return JSON.parse(s); } catch { return s === 'null' ? null : s; } }
fs.writeFileSync(${SNAPSHOT_FILE@Q}, JSON.stringify(snapshot, null, 2));
console.log('Snapshot written to ${SNAPSHOT_FILE}');
NODE

echo ">>> Snapshot complete: $SNAPSHOT_FILE"
echo "MIGRATION_SNAPSHOT_FILE=$SNAPSHOT_FILE"
