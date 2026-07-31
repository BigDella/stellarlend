#!/usr/bin/env bash
# Local gas benchmark runner for StellarLend.

set -euo pipefail

BENCH_DIR="stellar-lend"
BASELINE="$BENCH_DIR/benchmarks/baseline.json"
COVERAGE="$BENCH_DIR/benchmarks/public-functions.json"
HISTORY="$BENCH_DIR/benchmarks/history.jsonl"
OUTPUT="$BENCH_DIR/benchmark-results.json"
DASHBOARD="$BENCH_DIR/benchmark-dashboard.md"
HISTORY_OUT="$BENCH_DIR/benchmark-history.jsonl"

COMPARE=false
UPDATE_BASELINE=false
SHOW_HELP=false

for arg in "$@"; do
    case "$arg" in
        --compare) COMPARE=true ;;
        --update-baseline) UPDATE_BASELINE=true ;;
        --help|-h) SHOW_HELP=true ;;
        *)
            echo "Unknown argument: $arg" >&2
            exit 2
            ;;
    esac
done

if $SHOW_HELP; then
    cat <<'EOF'
StellarLend Gas Benchmark Runner

Usage:
  ./run-benchmarks.sh                   Run all benchmarks and generate dashboard
  ./run-benchmarks.sh --compare         Fail on budget, coverage, or >10% baseline regression
  ./run-benchmarks.sh --update-baseline Run and save current results as the new baseline

Outputs:
  stellar-lend/benchmark-results.json
  stellar-lend/benchmark-dashboard.md
  stellar-lend/benchmark-history.jsonl
EOF
    exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust/Cargo not found. Install from https://rustup.rs" >&2
    exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 not found. It is required for dashboard and regression gates." >&2
    exit 1
fi

if [ ! -d "$BENCH_DIR" ]; then
    echo "stellar-lend directory not found. Run from the project root." >&2
    exit 1
fi

echo "Building benchmark suite..."
(cd "$BENCH_DIR" && cargo build -p stellarlend-benchmarks --release)

echo "Running gas benchmarks..."
(cd "$BENCH_DIR" && cargo run -p stellarlend-benchmarks --bin run_benchmarks --release -- --output "../$OUTPUT")

REPORT_ARGS=(
    --results "$OUTPUT"
    --coverage "$COVERAGE"
    --dashboard "$DASHBOARD"
    --history "$HISTORY"
    --history-out "$HISTORY_OUT"
    --max-regression-pct 10
)

if $COMPARE; then
    REPORT_ARGS+=(--baseline "$BASELINE" --fail-on-regression)
fi

python3 scripts/gas_benchmark_report.py "${REPORT_ARGS[@]}"

if $UPDATE_BASELINE; then
    cp "$OUTPUT" "$BASELINE"
    echo "Baseline updated: $BASELINE"
fi

echo "Benchmarks complete:"
echo "  Results:   $OUTPUT"
echo "  Dashboard: $DASHBOARD"
