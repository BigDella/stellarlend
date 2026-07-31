#!/usr/bin/env bash
# approve.sh — Approval workflow: sign the migration report before mainnet execution.
#
# Usage: ./scripts/migration-simulator/approve.sh [options]
#
# The approver cryptographically signs the report content hash using a Stellar keypair.
# The resulting approval file must be present for index.ts to allow mainnet migration.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONFIG="$REPO_ROOT/environments/migration-sandbox/sandbox.config.json"

REPORT_FILE="${MIGRATION_REPORT_FILE:-}"
APPROVER_KEY="${APPROVER_SECRET_KEY:-}"
APPROVER_ALIAS="${APPROVER_ALIAS:-approver}"
APPROVALS_DIR="${APPROVALS_DIR:-$REPO_ROOT/.migration-sandbox/approvals}"

usage() {
  cat <<EOF
Usage: approve.sh [options]

Options:
  --report-file <path>      Migration plan report JSON from report.ts (required)
  --approver-key <secret>   Approver Stellar secret key for signing (required)
  --approver-alias <name>   Human-readable approver alias (default: approver)
  --approvals-dir <path>    Directory to write approval files
  --help                    Show this help

The approval file is bound to the report's contentHash. index.ts verifies this
binding before allowing a mainnet migration to proceed.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --report-file)    REPORT_FILE="$2"; shift 2 ;;
    --approver-key)   APPROVER_KEY="$2"; shift 2 ;;
    --approver-alias) APPROVER_ALIAS="$2"; shift 2 ;;
    --approvals-dir)  APPROVALS_DIR="$2"; shift 2 ;;
    --help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

command -v node    >/dev/null 2>&1 || { echo "ERROR: node not found" >&2; exit 1; }
command -v stellar >/dev/null 2>&1 || { echo "ERROR: stellar CLI not found" >&2; exit 1; }

[[ -n "$REPORT_FILE"   ]] || { echo "ERROR: --report-file is required" >&2; exit 1; }
[[ -n "$APPROVER_KEY"  ]] || { echo "ERROR: --approver-key is required (or set APPROVER_SECRET_KEY)" >&2; exit 1; }
[[ -f "$REPORT_FILE"   ]] || { echo "ERROR: report file not found: $REPORT_FILE" >&2; exit 1; }

mkdir -p "$APPROVALS_DIR"

# ── Read report content hash ──────────────────────────────────────────────────
CONTENT_HASH="$(node -e "const r=require('$REPORT_FILE'); console.log(r.contentHash)")"
REPORT_LABEL="$(node -e "const r=require('$REPORT_FILE'); console.log(r.label)")"

if [[ -z "$CONTENT_HASH" || "$CONTENT_HASH" == "undefined" ]]; then
  echo "ERROR: report does not contain a contentHash field. Regenerate with report.ts." >&2
  exit 1
fi

echo ">>> Report:       $REPORT_FILE"
echo ">>> Content hash: $CONTENT_HASH"
echo ">>> Label:        $REPORT_LABEL"

# ── Verify pre-flight checks passed ──────────────────────────────────────────
SIM_OK="$(node -e "const r=require('$REPORT_FILE'); console.log(r.preFlightChecks.simulationPassed)")"
INV_OK="$(node -e "const r=require('$REPORT_FILE'); console.log(r.preFlightChecks.invariantsPassed)")"
RB_OK="$(node -e  "const r=require('$REPORT_FILE'); console.log(r.preFlightChecks.rollbackValidated)")"

if [[ "$SIM_OK" != "true" || "$INV_OK" != "true" ]]; then
  echo "ERROR: report pre-flight checks did not pass. Approve only a passing report." >&2
  echo "  simulationPassed=$SIM_OK  invariantsPassed=$INV_OK  rollbackValidated=$RB_OK" >&2
  exit 1
fi

# ── Derive approver public key ────────────────────────────────────────────────
APPROVER_PUBLIC_KEY="$(stellar keys address "$APPROVER_KEY" 2>/dev/null || echo "")"
if [[ -z "$APPROVER_PUBLIC_KEY" ]]; then
  # Fallback: use node to derive public key from secret if stellar keys address isn't available
  APPROVER_PUBLIC_KEY="$(node -e "
    try {
      const { Keypair } = require('@stellar/stellar-sdk');
      const kp = Keypair.fromSecret(process.argv[1]);
      console.log(kp.publicKey());
    } catch(e) { console.log('UNKNOWN'); }
  " "$APPROVER_KEY" 2>/dev/null || echo "UNKNOWN")"
fi

# ── Sign the content hash ─────────────────────────────────────────────────────
# We use node + @stellar/stellar-sdk to produce an Ed25519 signature over the hash.
SIGNATURE="$(node -e "
  const CONTENT_HASH = process.argv[1];
  const SECRET = process.argv[2];
  try {
    const { Keypair } = require('@stellar/stellar-sdk');
    const kp = Keypair.fromSecret(SECRET);
    const sig = kp.sign(Buffer.from(CONTENT_HASH, 'hex'));
    console.log(sig.toString('hex'));
  } catch(e) {
    // Fallback: HMAC-SHA256 when SDK not available (dev environments)
    const crypto = require('crypto');
    const sig = crypto.createHmac('sha256', SECRET).update(CONTENT_HASH).digest('hex');
    console.log('hmac:' + sig);
  }
" "$CONTENT_HASH" "$APPROVER_KEY" 2>/dev/null)"

if [[ -z "$SIGNATURE" ]]; then
  echo "ERROR: failed to produce signature" >&2
  exit 1
fi

# ── Write approval file ───────────────────────────────────────────────────────
APPROVAL_FILE="$APPROVALS_DIR/approval-${REPORT_LABEL}.json"
APPROVED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

node - <<NODE
const fs = require('fs');
const approval = {
  schemaVersion: "1.0.0",
  approvedAt: ${APPROVED_AT@Q},
  reportFile: ${REPORT_FILE@Q},
  reportLabel: ${REPORT_LABEL@Q},
  contentHash: ${CONTENT_HASH@Q},
  approver: {
    alias: ${APPROVER_ALIAS@Q},
    publicKey: ${APPROVER_PUBLIC_KEY@Q},
  },
  signature: ${SIGNATURE@Q},
  preFlightChecks: {
    simulationPassed: $SIM_OK,
    invariantsPassed: $INV_OK,
    rollbackValidated: $RB_OK,
  },
};
fs.writeFileSync(${APPROVAL_FILE@Q}, JSON.stringify(approval, null, 2));
console.log('Approval written to ${APPROVAL_FILE}');
NODE

echo ">>> Approval complete"
echo "MIGRATION_APPROVAL_FILE=$APPROVAL_FILE"
echo ""
echo "This approval is required for mainnet migration."
echo "Pass it to index.ts via --approval-file or MIGRATION_APPROVAL_FILE."
