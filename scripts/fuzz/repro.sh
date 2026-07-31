#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <target> <input_file> [-- <extra libFuzzer args>]"
  echo "Example: $0 lending_actions stellar-lend/fuzz/artifacts/lending_actions/crash-* -- -runs=1"
  exit 2
fi

TARGET="$1"
INPUT="$2"
shift 2

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "$ROOT_DIR/stellar-lend"

export RUST_BACKTRACE=1

if [[ $# -ge 1 && "$1" == "--" ]]; then
  shift
  cargo +nightly fuzz run "$TARGET" "$INPUT" -- "$@"
else
  cargo +nightly fuzz run "$TARGET" "$INPUT" -- -runs=1
fi

