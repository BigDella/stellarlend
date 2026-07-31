#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# healthcheck.sh -- Verify all StellarLend devnet components are ready
#
# Checks:
#   - Soroban devnet node (RPC + Horizon)
#   - PostgreSQL database
#   - Redis cache
#   - API service
#   - Oracle service
#
# Exit codes:
#   0 -- all healthy
#   1 -- one or more services unhealthy
#
# Usage:
#   ./healthcheck.sh            # check all services
#   ./healthcheck.sh --json     # output JSON report
#   ./healthcheck.sh --wait     # block until all healthy (timeout 120s)
# ---------------------------------------------------------------------------
set -euo pipefail

SOROBAN_RPC_URL="${SOROBAN_RPC_URL:-http://localhost:8000/soroban/rpc}"
HORIZON_URL="${HORIZON_URL:-http://localhost:8000}"
API_URL="${API_URL:-http://localhost:3000}"
ORACLE_URL="${ORACLE_URL:-http://localhost:4000}"
POSTGRES_HOST="${POSTGRES_HOST:-localhost}"
POSTGRES_PORT="${POSTGRES_PORT:-5432}"
REDIS_HOST="${REDIS_HOST:-localhost}"
REDIS_PORT="${REDIS_PORT:-6379}"
HEALTH_TIMEOUT="${HEALTH_TIMEOUT:-120}"

OUTPUT_JSON=false
WAIT_MODE=false

for arg in "$@"; do
  case "${arg}" in
    --json) OUTPUT_JSON=true ;;
    --wait) WAIT_MODE=true ;;
  esac
done

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Track overall status
ALL_HEALTHY=true
declare -A STATUSES

# ---------------------------------------------------------------------------
# Check functions
# ---------------------------------------------------------------------------
check_soroban_rpc() {
  if curl -sf "${HORIZON_URL}/health" > /dev/null 2>&1; then
    STATUSES[soroban_rpc]="healthy"
  else
    STATUSES[soroban_rpc]="unhealthy"
    ALL_HEALTHY=false
  fi
}

check_postgres() {
  if pg_isready -h "${POSTGRES_HOST}" -p "${POSTGRES_PORT}" -U stellarlend > /dev/null 2>&1; then
    STATUSES[postgres]="healthy"
  elif docker exec stellarlend-devnet-postgres pg_isready -U stellarlend > /dev/null 2>&1; then
    STATUSES[postgres]="healthy"
  else
    STATUSES[postgres]="unhealthy"
    ALL_HEALTHY=false
  fi
}

check_redis() {
  if redis-cli -h "${REDIS_HOST}" -p "${REDIS_PORT}" ping > /dev/null 2>&1; then
    STATUSES[redis]="healthy"
  elif docker exec stellarlend-devnet-redis redis-cli ping > /dev/null 2>&1; then
    STATUSES[redis]="healthy"
  else
    STATUSES[redis]="unhealthy"
    ALL_HEALTHY=false
  fi
}

check_api() {
  if curl -sf "${API_URL}/api/health" > /dev/null 2>&1; then
    STATUSES[api]="healthy"
  else
    STATUSES[api]="unhealthy"
    ALL_HEALTHY=false
  fi
}

check_oracle() {
  if curl -sf "${ORACLE_URL}/health" > /dev/null 2>&1; then
    STATUSES[oracle]="healthy"
  else
    STATUSES[oracle]="unhealthy"
    ALL_HEALTHY=false
  fi
}

# ---------------------------------------------------------------------------
# Run all checks
# ---------------------------------------------------------------------------
run_checks() {
  ALL_HEALTHY=true
  check_soroban_rpc
  check_postgres
  check_redis
  check_api
  check_oracle
}

# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------
print_status() {
  local name="$1"
  local status="${STATUSES[$name]}"
  if [ "${status}" = "healthy" ]; then
    echo -e "  ${GREEN}[OK]${NC}  ${name}"
  else
    echo -e "  ${RED}[FAIL]${NC} ${name}"
  fi
}

print_report() {
  echo ""
  echo "StellarLend Devnet Health Check"
  echo "==============================="
  print_status "soroban_rpc"
  print_status "postgres"
  print_status "redis"
  print_status "api"
  print_status "oracle"
  echo ""
  if ${ALL_HEALTHY}; then
    echo -e "${GREEN}All services are healthy.${NC}"
  else
    echo -e "${RED}One or more services are unhealthy.${NC}"
  fi
  echo ""
}

print_json() {
  local overall="healthy"
  ${ALL_HEALTHY} || overall="unhealthy"
  cat <<ENDJSON
{
  "status": "${overall}",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "services": {
    "soroban_rpc": "${STATUSES[soroban_rpc]}",
    "postgres": "${STATUSES[postgres]}",
    "redis": "${STATUSES[redis]}",
    "api": "${STATUSES[api]}",
    "oracle": "${STATUSES[oracle]}"
  }
}
ENDJSON
}

# ---------------------------------------------------------------------------
# Wait mode
# ---------------------------------------------------------------------------
wait_until_healthy() {
  local elapsed=0
  echo "Waiting for all services to become healthy (timeout: ${HEALTH_TIMEOUT}s)..."
  while [ "${elapsed}" -lt "${HEALTH_TIMEOUT}" ]; do
    run_checks
    if ${ALL_HEALTHY}; then
      echo -e "${GREEN}All services healthy after ${elapsed}s.${NC}"
      return 0
    fi
    sleep 3
    elapsed=$((elapsed + 3))
    if [ $((elapsed % 15)) -eq 0 ]; then
      echo -e "${YELLOW}  Still waiting... (${elapsed}s / ${HEALTH_TIMEOUT}s)${NC}"
      for svc in soroban_rpc postgres redis api oracle; do
        [ "${STATUSES[$svc]}" = "unhealthy" ] && echo "    - ${svc} not ready"
      done
    fi
  done
  echo -e "${RED}Timed out after ${HEALTH_TIMEOUT}s.${NC}"
  return 1
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
if ${WAIT_MODE}; then
  wait_until_healthy
  exit $?
fi

run_checks

if ${OUTPUT_JSON}; then
  print_json
else
  print_report
fi

${ALL_HEALTHY} && exit 0 || exit 1
