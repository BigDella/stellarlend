#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FUZZ_DIR="$ROOT_DIR/stellar-lend/fuzz"

MIN_FILES="${MIN_CORPUS_FILES:-10}"

targets=(lending_critical lending_actions amm_actions bridge_actions)

for t in "${targets[@]}"; do
  d="$FUZZ_DIR/corpus/$t"
  if [[ ! -d "$d" ]]; then
    echo "::error::Missing corpus directory: $d"
    exit 1
  fi

  count="$(find "$d" -maxdepth 1 -type f | wc -l | tr -d ' ')"
  if [[ "$count" -lt "$MIN_FILES" ]]; then
    echo "::error::Corpus for $t has $count files (minimum $MIN_FILES): $d"
    exit 1
  fi

  empty="$(find "$d" -maxdepth 1 -type f -size 0c | wc -l | tr -d ' ')"
  if [[ "$empty" -ne 0 ]]; then
    echo "::error::Corpus for $t contains $empty empty files: $d"
    exit 1
  fi
done

echo "Corpus OK (min files per target: $MIN_FILES)"

