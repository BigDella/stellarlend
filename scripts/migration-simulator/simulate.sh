#!/usr/bin/env bash
# simulate.sh — Run the migration against a local Quickstart fork and validate invariants.
#
# Usage: ./scripts/migration-simulator/simulate.sh [options]
#
# Workflow:
#   1. Start local Quickstart sandbox (or reuse existing)
#   2. Seed the fork with the snapshot state
#   3. Invoke migration operations
#   4. Validate post-migration invariants
#   5. Simulate failure + rollback paths
#   6. Write a simulation result JSON consumed by report.ts
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONFIG="$REPO_ROOT/environments/migration-sandbox/sandbox.config.json"

SNAPSHOT_FILE="${MIGRATION_SNAPSHOT_FILE:-}"
LENDING_CONTRACT_ID="${LENDING_CONTRACT_ID:-}"
ADMIN_SECRET_KEY="${ADMIN_SECRET_KEY:-}"
SANDBOX_NETWORK="${STELLAR_NETWORK:-local}"
RPC_URL="${STELLAR_RPC_URL:-http://localhost:8000/soroban/rpc}"
RESULTS_DIR="${RESULTS_DIR:-$REPO_ROOT/.migration-sandbox/results}"
LABEL="${LABEL:-$(date -u +%Y%m%dT%H%M%SZ)}"
SKIP_SANDBOX_START=false

usage() {
  cat <<EOF
Usage: simulate.sh [options]

Options:
  --snapshot-file <path>       Snapshot JSON from snapshot.sh (required)
  --lending-contract-id <id>   Deployed contract ID on sandbox (required)
  --admin-secret <key>         Admin secret key for sandbox invocations (required)
  --rpc-url <url>              Soroban RPC URL (default: http://localhost:8000/soroban/rpc)
  --results-dir <path>         Directory to write simulation results
  --label <label>              Result label (default: ISO timestamp)
  --skip-sandbox-start         Assume sandbox is already running
  --help                       Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --snapshot-file)        SNAPSHOT_FILE="$2"; shift 2 ;;
    --lending-contract-id)  LENDING_CONTRACT_ID="$2"; shift 2 ;;
    --admin-secret)         ADMIN_SECRET_KEY="$2"; shift 2 ;;
    --rpc-url)              RPC_URL="$2"; shift 2 ;;
    --results-dir)          RESULTS_DIR="$2"; shift 2 ;;
    --label)                LABEL="$2"; shift 2 ;;
    --skip-sandbox-start)   SKIP_SANDBOX_START=true; shift ;;
    --help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

command -v stellar >/dev/null 2>&1 || { echo "ERROR: stellar CLI not found" >&2; exit 1; }
command -v node    >/dev/null 2>&1 || { echo "ERROR: node not found" >&2; exit 1; }

[[ -n "$SNAPSHOT_FILE"      ]] || { echo "ERROR: --snapshot-file is required" >&2; exit 1; }
[[ -n "$LENDING_CONTRACT_ID" ]] || { echo "ERROR: --lending-contract-id is required" >&2; exit 1; }
[[ -n "$ADMIN_SECRET_KEY"   ]] || { echo "ERROR: --admin-secret is required" >&2; exit 1; }
[[ -f "$SNAPSHOT_FILE"      ]] || { echo "ERROR: snapshot file not found: $SNAPSHOT_FILE" >&2; exit 1; }

mkdir -p "$RESULTS_DIR"
RESULT_FILE="$RESULTS_DIR/simulate-${LABEL}.json"

# ── 1. Start sandbox ─────────────────────────────────────────────────────────
if ! $SKIP_SANDBOX_START; then
  echo ">>> Starting local Quickstart sandbox"
  stellar container stop local >/dev/null 2>&1 || true
  stellar container start local --limits testnet
  echo ">>> Waiting for RPC to become available"
  for i in $(seq 1 30); do
    curl -sf "$RPC_URL" >/dev/null 2>&1 && break || sleep 2
    [[ $i -eq 30 ]] && { echo "ERROR: RPC not available after 60s" >&2; exit 1; }
  done
fi

# ── 2. Seed fork with snapshot ────────────────────────────────────────────────
echo ">>> Seeding sandbox from snapshot: $SNAPSHOT_FILE"
STELLAR_ARGS=(--id "$LENDING_CONTRACT_ID" --source "$ADMIN_SECRET_KEY" --network "$SANDBOX_NETWORK" --rpc-url "$RPC_URL")

seed_param() {
  # Applies a single admin parameter to the sandbox contract if the snapshot has it.
  local fn="$1" param_flag="$2" param_value="$3"
  if [[ "$param_value" != "null" && -n "$param_value" ]]; then
    stellar contract invoke "${STELLAR_ARGS[@]}" -- "$fn" "$param_flag" "$param_value" 2>/dev/null \
      && echo "  seeded $fn=$param_value" \
      || echo "  WARN: could not seed $fn (continuing)"
  fi
}

# Extract snapshot fields
MIN_COLLATERAL_RATIO="$(node -e "const s=require('$SNAPSHOT_FILE'); console.log((s.state.protocolParams && s.state.protocolParams.min_collateral_ratio) || 'null')")"
BASE_RATE="$(node -e "const s=require('$SNAPSHOT_FILE'); console.log((s.state.protocolParams && s.state.protocolParams.base_rate) || 'null')")"
RESERVE_FACTOR="$(node -e "const s=require('$SNAPSHOT_FILE'); console.log((s.state.protocolParams && s.state.protocolParams.reserve_factor) || 'null')")"

seed_param set_min_collateral_ratio --ratio "$MIN_COLLATERAL_RATIO"
seed_param set_base_rate             --rate  "$BASE_RATE"
seed_param set_reserve_factor        --factor "$RESERVE_FACTOR"

# ── 3. Run migration operations ───────────────────────────────────────────────
echo ">>> Running migration operations on fork"
MIGRATION_START="$(date -u +%s%3N)"
MIGRATION_OK=true
MIGRATION_ERROR=""

run_migration_step() {
  local step_name="$1"; shift
  echo "  step: $step_name"
  if ! stellar contract invoke "${STELLAR_ARGS[@]}" -- "$@" 2>&1; then
    MIGRATION_OK=false
    MIGRATION_ERROR="$step_name failed"
    return 1
  fi
}

# Collect gas for each migration step using simulate (--sim-only)
declare -A STEP_CPU_INSNS=()
simulate_step() {
  local step_name="$1"; shift
  local out
  out="$(stellar contract invoke "${STELLAR_ARGS[@]}" --sim-only -- "$@" 2>&1 || true)"
  local cpu
  cpu="$(echo "$out" | grep -oP 'cpu_insns:\s*\K[0-9]+' | head -1 || echo 0)"
  STEP_CPU_INSNS["$step_name"]="${cpu:-0}"
}

# Example migration steps — apply protocol parameter updates from snapshot
simulate_step "set_min_collateral_ratio" set_min_collateral_ratio --ratio "${MIN_COLLATERAL_RATIO:-150}"
simulate_step "set_base_rate"            set_base_rate             --rate  "${BASE_RATE:-100}"

run_migration_step "set_min_collateral_ratio" set_min_collateral_ratio --ratio "${MIN_COLLATERAL_RATIO:-150}" || true
run_migration_step "set_base_rate"            set_base_rate             --rate  "${BASE_RATE:-100}" || true

MIGRATION_END="$(date -u +%s%3N)"
MIGRATION_DURATION_MS=$(( MIGRATION_END - MIGRATION_START ))

# ── 4. Validate invariants ────────────────────────────────────────────────────
echo ">>> Validating post-migration invariants"
INVARIANTS_PASSED=true
INVARIANT_ERRORS=()

check_invariant() {
  local name="$1" fn="$2"
  local result
  result="$(stellar contract invoke "${STELLAR_ARGS[@]}" -- "$fn" 2>/dev/null || echo "error")"
  if [[ "$result" == "error" || "$result" == "null" ]]; then
    INVARIANTS_PASSED=false
    INVARIANT_ERRORS+=("$name: query failed")
    echo "  FAIL: $name"
  else
    echo "  PASS: $name = $result"
  fi
}

check_invariant "protocol_params_readable"  get_protocol_params
check_invariant "risk_config_readable"      get_risk_config
check_invariant "system_stats_readable"     get_system_stats

# Numeric bounds: min_collateral_ratio must be >= 100
POST_MCR="$(stellar contract invoke "${STELLAR_ARGS[@]}" -- get_protocol_params 2>/dev/null \
  | node -e "let d=''; process.stdin.on('data',c=>d+=c); process.stdin.on('end',()=>{ try{const p=JSON.parse(d); console.log(p.min_collateral_ratio||0);}catch{console.log(0);} })" \
  || echo 0)"
if (( POST_MCR < 100 )); then
  INVARIANTS_PASSED=false
  INVARIANT_ERRORS+=("min_collateral_ratio below minimum: $POST_MCR")
fi

# ── 5. Failure simulation ─────────────────────────────────────────────────────
echo ">>> Simulating failure + rollback"
ROLLBACK_OK=false
ROLLBACK_ERROR=""

# Attempt an invalid operation to confirm the contract rejects it
if stellar contract invoke "${STELLAR_ARGS[@]}" -- set_min_collateral_ratio --ratio 0 2>/dev/null; then
  ROLLBACK_ERROR="Expected rejection of ratio=0, but it succeeded"
else
  ROLLBACK_OK=true
  echo "  Rollback check passed: invalid operation correctly rejected"
fi

# ── 6. Write result JSON ──────────────────────────────────────────────────────
TOTAL_CPU_INSNS=0
for v in "${STEP_CPU_INSNS[@]}"; do TOTAL_CPU_INSNS=$(( TOTAL_CPU_INSNS + v )); done

node - <<NODE
const fs = require('fs');
const stepCpu = $(printf '%s\n' "${!STEP_CPU_INSNS[@]}" | node -e "
  const keys=[];let d='';process.stdin.on('data',c=>d+=c);process.stdin.on('end',()=>{
    const steps={};
    d.trim().split('\n').filter(Boolean).forEach(k=>{
      steps[k]=${STEP_CPU_INSNS[$k]:-0};
    });
    console.log(JSON.stringify(steps));
  });
" 2>/dev/null || echo '{}');

const result = {
  label:           ${LABEL@Q},
  timestamp:       new Date().toISOString(),
  snapshotFile:    ${SNAPSHOT_FILE@Q},
  contractId:      ${LENDING_CONTRACT_ID@Q},
  migrationOk:     $( [[ $MIGRATION_OK == true ]] && echo true || echo false ),
  migrationError:  ${MIGRATION_ERROR@Q},
  migrationDurationMs: $MIGRATION_DURATION_MS,
  totalCpuInsns:   $TOTAL_CPU_INSNS,
  stepCpuInsns:    stepCpu,
  invariantsPassed: $( [[ $INVARIANTS_PASSED == true ]] && echo true || echo false ),
  invariantErrors:  $(printf '%s\n' "${INVARIANT_ERRORS[@]+"${INVARIANT_ERRORS[@]}"}" | node -e "let d='';process.stdin.on('data',c=>d+=c);process.stdin.on('end',()=>{ const lines=d.trim().split('\n').filter(Boolean); console.log(JSON.stringify(lines)); })" 2>/dev/null || echo '[]'),
  rollbackOk:      $( [[ $ROLLBACK_OK == true ]] && echo true || echo false ),
  rollbackError:   ${ROLLBACK_ERROR@Q},
};
fs.writeFileSync(${RESULT_FILE@Q}, JSON.stringify(result, null, 2));
console.log('Simulation result written to ${RESULT_FILE}');
NODE

echo ">>> Simulation complete: $RESULT_FILE"
echo "MIGRATION_RESULT_FILE=$RESULT_FILE"

if ! $MIGRATION_OK || ! $INVARIANTS_PASSED; then
  echo "ERROR: Simulation failed. Check $RESULT_FILE for details." >&2
  exit 1
fi
