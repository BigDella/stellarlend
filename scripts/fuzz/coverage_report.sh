#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FUZZ_DIR="$ROOT_DIR/stellar-lend/fuzz"
COVERAGE_DIR="$FUZZ_DIR/coverage"

if [[ $# -gt 0 ]]; then
  targets=("$@")
else
  targets=(lending_critical lending_actions amm_actions bridge_actions)
fi

mkdir -p "$COVERAGE_DIR"

cd "$ROOT_DIR/stellar-lend"

summary="$COVERAGE_DIR/summary.md"
{
  echo "# Fuzz Coverage Report"
  echo
  echo "| Target | Corpus files | Coverage command |"
  echo "| --- | ---: | --- |"
} > "$summary"

for target in "${targets[@]}"; do
  corpus_dir="$FUZZ_DIR/corpus/$target"
  count="$(find "$corpus_dir" -maxdepth 1 -type f | wc -l | tr -d ' ')"
  target_log="$COVERAGE_DIR/$target.log"

  set +e
  cargo +nightly fuzz coverage "$target" "fuzz/corpus/$target" > "$target_log" 2>&1
  status=$?
  set -e

  if [[ "$status" -ne 0 ]]; then
    echo "::warning::coverage generation failed for $target; see $target_log"
  fi

  echo "| \`$target\` | $count | \`cargo +nightly fuzz coverage $target fuzz/corpus/$target\` |" >> "$summary"
done

echo "Coverage summary written to $summary"
