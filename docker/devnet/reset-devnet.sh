#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# reset-devnet.sh -- Tear down and rebuild the StellarLend local devnet
#
# This script:
#   1. Stops all containers
#   2. Removes volumes (blockchain state, database, cache)
#   3. Restarts everything
#   4. Re-runs initialization
#
# Target: complete reset in under 2 minutes.
# ---------------------------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.yml"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()  { echo -e "${GREEN}[reset]${NC} $(date '+%H:%M:%S') $*"; }
warn() { echo -e "${YELLOW}[reset]${NC} $(date '+%H:%M:%S') WARN: $*"; }
err()  { echo -e "${RED}[reset]${NC} $(date '+%H:%M:%S') ERROR: $*" >&2; }

START_TIME=$(date +%s)

# ---------------------------------------------------------------------------
# Step 1: Stop containers
# ---------------------------------------------------------------------------
log "Step 1/4: Stopping all containers..."
docker compose -f "${COMPOSE_FILE}" down --timeout 10 2>/dev/null || \
docker-compose -f "${COMPOSE_FILE}" down --timeout 10 2>/dev/null || \
warn "No running containers to stop"

# ---------------------------------------------------------------------------
# Step 2: Remove volumes
# ---------------------------------------------------------------------------
log "Step 2/4: Removing volumes..."
docker compose -f "${COMPOSE_FILE}" down -v --timeout 5 2>/dev/null || \
docker-compose -f "${COMPOSE_FILE}" down -v --timeout 5 2>/dev/null || \
warn "Could not remove volumes (they may not exist yet)"

# Also prune any orphaned devnet volumes
docker volume ls -q --filter "name=stellarlend-devnet" 2>/dev/null | \
  xargs -r docker volume rm 2>/dev/null || true

# ---------------------------------------------------------------------------
# Step 3: Rebuild and restart
# ---------------------------------------------------------------------------
log "Step 3/4: Rebuilding and starting services..."
docker compose -f "${COMPOSE_FILE}" up -d --build --force-recreate 2>/dev/null || \
docker-compose -f "${COMPOSE_FILE}" up -d --build --force-recreate 2>/dev/null || {
  err "Failed to start services"
  exit 1
}

# ---------------------------------------------------------------------------
# Step 4: Wait for initialization
# ---------------------------------------------------------------------------
log "Step 4/4: Waiting for initialization to complete..."

# The init-devnet service runs automatically. Wait for it to finish.
INIT_TIMEOUT=120
elapsed=0
while [ "${elapsed}" -lt "${INIT_TIMEOUT}" ]; do
  # Check if the init container has exited
  status=$(docker inspect -f '{{.State.Status}}' stellarlend-devnet-init 2>/dev/null || echo "unknown")

  case "${status}" in
    exited)
      exit_code=$(docker inspect -f '{{.State.ExitCode}}' stellarlend-devnet-init 2>/dev/null || echo "1")
      if [ "${exit_code}" = "0" ]; then
        log "Initialization completed successfully!"
        break
      else
        err "Init container exited with code ${exit_code}"
        log "Showing init logs:"
        docker logs stellarlend-devnet-init 2>&1 | tail -30
        exit 1
      fi
      ;;
    running)
      if [ $((elapsed % 15)) -eq 0 ]; then
        log "  Init still running... (${elapsed}s / ${INIT_TIMEOUT}s)"
      fi
      ;;
    unknown)
      warn "  Init container not found yet... (${elapsed}s)"
      ;;
  esac

  sleep 3
  elapsed=$((elapsed + 3))
done

if [ "${elapsed}" -ge "${INIT_TIMEOUT}" ]; then
  err "Initialization timed out after ${INIT_TIMEOUT}s"
  log "Showing init logs:"
  docker logs stellarlend-devnet-init 2>&1 | tail -30
  exit 1
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

log ""
log "============================================================"
log "  Devnet reset complete in ${DURATION}s"
log "============================================================"
log ""
log "Services:"
log "  Soroban RPC:  http://localhost:${SOROBAN_RPC_PORT:-8000}/soroban/rpc"
log "  Horizon:      http://localhost:${SOROBAN_RPC_PORT:-8000}"
log "  API:          http://localhost:${API_PORT:-3000}"
log "  Oracle:       http://localhost:${ORACLE_PORT:-4000}"
log "  PostgreSQL:   localhost:${POSTGRES_PORT:-5432}"
log "  Redis:        localhost:${REDIS_PORT:-6379}"
log ""
log "View logs:    docker compose -f ${COMPOSE_FILE} logs -f"
log "Stop:         docker compose -f ${COMPOSE_FILE} down"
log "============================================================"
