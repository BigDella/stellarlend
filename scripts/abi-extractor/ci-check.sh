#!/usr/bin/env bash
# =============================================================================
# scripts/abi-extractor/ci-check.sh
#
# CI integration script for ABI extraction and breaking-change detection.
# Designed to be called from GitHub Actions or any CI system.
#
# Workflow:
#   1. Extract ABIs from compiled WASM artifacts
#   2. Generate TypeScript (and optionally Rust) types
#   3. Compare against the last tagged/baseline version
#   4. Report changes (outputs summary for CI comment)
#
# Usage:
#   ./scripts/abi-extractor/ci-check.sh [OPTIONS]
#
# Options:
#   --fail-on-breaking    Exit with non-zero if breaking changes detected
#   --generate-types      Also regenerate TypeScript types
#   --generate-rust       Also regenerate Rust client types
#   --skip-build          Skip the contract build step
#   --output-file <file>  Write summary to a file (for CI comment)
#   --help                Show help
#
# Environment variables:
#   GITHUB_STEP_SUMMARY   If set, appends markdown summary (GitHub Actions)
#   GITHUB_OUTPUT         If set, writes output variables (GitHub Actions)
#
# Exit codes:
#   0  No breaking changes (or --fail-on-breaking not set)
#   1  Script error
#   2  Breaking changes detected (only with --fail-on-breaking)
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Resolve paths
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
FAIL_ON_BREAKING=false
GENERATE_TYPES=false
GENERATE_RUST=false
SKIP_BUILD=false
OUTPUT_FILE=""

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --fail-on-breaking) FAIL_ON_BREAKING=true; shift ;;
    --generate-types)   GENERATE_TYPES=true;   shift ;;
    --generate-rust)    GENERATE_RUST=true;    shift ;;
    --skip-build)       SKIP_BUILD=true;       shift ;;
    --output-file)      OUTPUT_FILE="$2";      shift 2 ;;
    --help)
      head -30 "$0" | tail -27
      exit 0
      ;;
    *)
      echo "ERROR: Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

echo "======================================================================"
echo " StellarLend ABI CI Check"
echo "======================================================================"
echo ""

# ---------------------------------------------------------------------------
# Step 1: Extract ABIs
# ---------------------------------------------------------------------------
echo ">>> Step 1: Extract ABIs"

EXTRACT_ARGS=()
if [[ "$SKIP_BUILD" == "true" ]]; then
  EXTRACT_ARGS+=("--skip-build")
fi

"$SCRIPT_DIR/extract.sh" "${EXTRACT_ARGS[@]}" 2>&1 | while IFS= read -r line; do
  echo "  [extract] $line"
done

EXTRACT_EXIT=${PIPESTATUS[0]:-0}
if [[ "$EXTRACT_EXIT" -ne 0 ]]; then
  echo "ERROR: ABI extraction failed (exit code $EXTRACT_EXIT)" >&2
  exit 1
fi
echo ""

# ---------------------------------------------------------------------------
# Step 2: Generate types (optional)
# ---------------------------------------------------------------------------
if [[ "$GENERATE_TYPES" == "true" ]]; then
  echo ">>> Step 2: Generate TypeScript types"

  TYPE_ARGS=()
  if [[ "$GENERATE_RUST" == "true" ]]; then
    TYPE_ARGS+=("--rust")
  fi

  node "$SCRIPT_DIR/generate-types.js" "${TYPE_ARGS[@]}" 2>&1 | while IFS= read -r line; do
    echo "  [types] $line"
  done
  echo ""
fi

# ---------------------------------------------------------------------------
# Step 3: Version comparison
# ---------------------------------------------------------------------------
echo ">>> Step 3: Check for ABI changes"

VERSION_OUTPUT=$("$SCRIPT_DIR/version.sh" --dry-run 2>&1) || true
VERSION_EXIT=${PIPESTATUS[0]:-0}

echo "$VERSION_OUTPUT" | while IFS= read -r line; do
  echo "  [version] $line"
done
echo ""

# ---------------------------------------------------------------------------
# Parse results
# ---------------------------------------------------------------------------
HAS_BREAKING=false
HAS_CHANGES=false
CHANGE_SUMMARY=""

if echo "$VERSION_OUTPUT" | grep -q "\[BREAK\]"; then
  HAS_BREAKING=true
  HAS_CHANGES=true
fi
if echo "$VERSION_OUTPUT" | grep -q "\[ADD\]\|\[PATCH\]"; then
  HAS_CHANGES=true
fi

# Extract version bump line
BUMP_LINE=$(echo "$VERSION_OUTPUT" | grep "Version bump:" || echo "")

# Count changes
BREAKING_COUNT=$(echo "$VERSION_OUTPUT" | grep -c "\[BREAK\]" || echo "0")
ADDITION_COUNT=$(echo "$VERSION_OUTPUT" | grep -c "\[ADD\]" || echo "0")
PATCH_COUNT=$(echo "$VERSION_OUTPUT" | grep -c "\[PATCH\]" || echo "0")

# ---------------------------------------------------------------------------
# Build summary report
# ---------------------------------------------------------------------------
SUMMARY="## ABI Change Report

"

if [[ "$HAS_CHANGES" == "false" ]]; then
  SUMMARY+="No ABI changes detected.
"
else
  if [[ "$HAS_BREAKING" == "true" ]]; then
    SUMMARY+="**WARNING: Breaking changes detected!**

"
  fi

  SUMMARY+="| Type | Count |
|------|-------|
| Breaking | $BREAKING_COUNT |
| Additions | $ADDITION_COUNT |
| Patches | $PATCH_COUNT |

"

  if [[ -n "$BUMP_LINE" ]]; then
    SUMMARY+="**$BUMP_LINE**

"
  fi

  # List specific changes
  SUMMARY+="### Details
\`\`\`
"
  SUMMARY+="$(echo "$VERSION_OUTPUT" | grep -E '\[(BREAK|ADD|PATCH|OK)\]' || echo "No details available")"
  SUMMARY+="
\`\`\`
"
fi

echo "--- CI Summary ---"
echo "$SUMMARY"
echo "------------------"

# ---------------------------------------------------------------------------
# Write outputs
# ---------------------------------------------------------------------------

# Write to output file if requested
if [[ -n "$OUTPUT_FILE" ]]; then
  echo "$SUMMARY" > "$OUTPUT_FILE"
  echo "Summary written to: $OUTPUT_FILE"
fi

# Write GitHub Actions step summary
if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  echo "$SUMMARY" >> "$GITHUB_STEP_SUMMARY"
  echo "Summary written to GITHUB_STEP_SUMMARY"
fi

# Write GitHub Actions output variables
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  echo "has_changes=$HAS_CHANGES" >> "$GITHUB_OUTPUT"
  echo "has_breaking=$HAS_BREAKING" >> "$GITHUB_OUTPUT"
  echo "breaking_count=$BREAKING_COUNT" >> "$GITHUB_OUTPUT"
  echo "addition_count=$ADDITION_COUNT" >> "$GITHUB_OUTPUT"
  echo "patch_count=$PATCH_COUNT" >> "$GITHUB_OUTPUT"
fi

# ---------------------------------------------------------------------------
# Exit
# ---------------------------------------------------------------------------
echo ""
if [[ "$HAS_BREAKING" == "true" && "$FAIL_ON_BREAKING" == "true" ]]; then
  echo "FAIL: Breaking ABI changes detected. Use --fail-on-breaking to control this behavior."
  exit 2
fi

if [[ "$HAS_CHANGES" == "true" ]]; then
  echo "ABI changes detected. Review the summary above."
else
  echo "No ABI changes. All contracts match baseline."
fi

exit 0
