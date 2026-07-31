#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-lending_actions}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_DIR="$ROOT_DIR/stellar-lend/fuzz/artifacts/$TARGET"
REGRESSION_DIR="$ROOT_DIR/stellar-lend/fuzz/regressions/$TARGET"

mkdir -p "$REGRESSION_DIR"

if [[ ! -d "$ARTIFACT_DIR" ]]; then
  echo "No artifact directory found for $TARGET: $ARTIFACT_DIR"
  exit 0
fi

crash="$(find "$ARTIFACT_DIR" -maxdepth 1 -type f \( -name 'crash-*' -o -name 'timeout-*' \) | sort | head -n 1)"
if [[ -z "$crash" ]]; then
  echo "No crash or timeout artifact found for $TARGET"
  exit 0
fi

name="$(basename "$crash")"
cp "$crash" "$REGRESSION_DIR/$name"

echo "Copied crash artifact to $REGRESSION_DIR/$name"
echo "Reproduce locally with:"
echo "  bash scripts/fuzz/repro.sh $TARGET stellar-lend/fuzz/regressions/$TARGET/$name"

bash "$ROOT_DIR/scripts/fuzz/repro.sh" "$TARGET" "$REGRESSION_DIR/$name" -- -runs=1 || true
