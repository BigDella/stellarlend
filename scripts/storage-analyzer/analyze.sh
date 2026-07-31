#!/usr/bin/env bash
# analyze.sh — Main entry point for the PR storage impact analyzer.
#
# Extracts the storage layout from two checkouts (base and head), runs the
# Node.js comparison script, and outputs a Markdown impact report to stdout.
#
# Usage:
#   ./analyze.sh <base_dir> <head_dir>
#
# Exit codes:
#   0  — no breaking changes
#   1  — breaking storage changes detected (CI should block)
#   2  — script error

set -euo pipefail

BASE_DIR="${1:?Usage: analyze.sh <base_dir> <head_dir>}"
HEAD_DIR="${2:?Usage: analyze.sh <base_dir> <head_dir>}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ── Validate inputs ──────────────────────────────────────────────────
if [ ! -d "$BASE_DIR" ]; then
  echo "Error: base directory does not exist: $BASE_DIR" >&2
  exit 2
fi

if [ ! -d "$HEAD_DIR" ]; then
  echo "Error: head directory does not exist: $HEAD_DIR" >&2
  exit 2
fi

# ── Create temp directory ────────────────────────────────────────────
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

BASE_JSON="$TMP_DIR/base-storage.json"
HEAD_JSON="$TMP_DIR/head-storage.json"

# ── Extract storage layouts ──────────────────────────────────────────
EXTRACT="$SCRIPT_DIR/extract-storage.sh"

if [ ! -x "$EXTRACT" ]; then
  chmod +x "$EXTRACT"
fi

echo "Extracting base storage layout..." >&2
"$EXTRACT" "$BASE_DIR" > "$BASE_JSON" 2>/dev/null

echo "Extracting head storage layout..." >&2
"$EXTRACT" "$HEAD_DIR" > "$HEAD_JSON" 2>/dev/null

# ── Run comparison ───────────────────────────────────────────────────
COMPARE="$SCRIPT_DIR/compare.js"

if [ ! -f "$COMPARE" ]; then
  echo "Error: compare.js not found at $COMPARE" >&2
  exit 2
fi

echo "Comparing storage layouts..." >&2
node "$COMPARE" "$BASE_JSON" "$HEAD_JSON"
