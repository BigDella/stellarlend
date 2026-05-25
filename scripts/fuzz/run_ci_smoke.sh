#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Keep CI fast but meaningful: run each fuzzer for a bounded number of executions.
RUNS="${FUZZ_RUNS:-5000}"
TIMEOUT="${FUZZ_TIMEOUT_SEC:-5}"

bash "$ROOT_DIR/scripts/fuzz/check_corpus.sh"

cd "$ROOT_DIR/stellar-lend"

targets=(lending_critical lending_actions amm_actions bridge_actions)

for t in "${targets[@]}"; do
  echo "Running fuzz smoke: $t (runs=$RUNS timeout=${TIMEOUT}s)"
  cargo +nightly fuzz run "$t" -- -runs="$RUNS" -timeout="$TIMEOUT"
done

