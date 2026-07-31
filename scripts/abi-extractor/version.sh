#!/usr/bin/env bash
# =============================================================================
# scripts/abi-extractor/version.sh
#
# ABI versioning: compare current ABIs against the previous version, detect
# breaking vs non-breaking changes, bump the package version accordingly,
# and generate a CHANGELOG entry.
#
# Usage:
#   ./scripts/abi-extractor/version.sh [OPTIONS]
#
# Options:
#   --abi-dir <dir>      Directory containing current ABI JSON files
#                        (default: packages/contract-abis/abi)
#   --baseline-dir <dir> Directory containing baseline ABI JSON files
#                        (default: packages/contract-abis/abi/.baseline)
#   --package-json <f>   Path to package.json (default: packages/contract-abis/package.json)
#   --dry-run            Show what would change without modifying files
#   --help               Show this help message
#
# Exit codes:
#   0  Success
#   1  Error
#   2  Breaking changes detected (useful for CI gating)
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
ABI_DIR="$REPO_ROOT/packages/contract-abis/abi"
BASELINE_DIR="$REPO_ROOT/packages/contract-abis/abi/.baseline"
PACKAGE_JSON="$REPO_ROOT/packages/contract-abis/package.json"
CHANGELOG="$REPO_ROOT/packages/contract-abis/CHANGELOG.md"
DRY_RUN=false

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --abi-dir)      ABI_DIR="$2";      shift 2 ;;
    --baseline-dir) BASELINE_DIR="$2"; shift 2 ;;
    --package-json) PACKAGE_JSON="$2"; shift 2 ;;
    --dry-run)      DRY_RUN=true;      shift   ;;
    --help)
      head -22 "$0" | tail -19
      exit 0
      ;;
    *)
      echo "ERROR: Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

echo "======================================================================"
echo " StellarLend ABI Versioning"
echo "======================================================================"
echo ""

# ---------------------------------------------------------------------------
# Validate inputs
# ---------------------------------------------------------------------------
if [[ ! -d "$ABI_DIR" ]]; then
  echo "ERROR: ABI directory not found: $ABI_DIR" >&2
  echo "Run scripts/abi-extractor/extract.sh first." >&2
  exit 1
fi

if [[ ! -f "$PACKAGE_JSON" ]]; then
  echo "ERROR: package.json not found: $PACKAGE_JSON" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Read current version
# ---------------------------------------------------------------------------
CURRENT_VERSION=$(grep '"version"' "$PACKAGE_JSON" | head -1 | sed 's/.*"version": *"//;s/".*//')
if [[ -z "$CURRENT_VERSION" ]]; then
  echo "ERROR: Could not read version from $PACKAGE_JSON" >&2
  exit 1
fi

echo "Current version: $CURRENT_VERSION"

# Parse semver
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# ---------------------------------------------------------------------------
# Detect changes
# ---------------------------------------------------------------------------
BREAKING_CHANGES=()
ADDITIONS=()
PATCHES=()
HAS_CHANGES=false

# Create baseline dir if it does not exist (first run)
if [[ ! -d "$BASELINE_DIR" ]]; then
  echo "No baseline found — this is the initial version."
  echo "Creating baseline from current ABIs..."

  if [[ "$DRY_RUN" == "false" ]]; then
    mkdir -p "$BASELINE_DIR"
    cp "$ABI_DIR"/*.json "$BASELINE_DIR"/ 2>/dev/null || true
  fi

  echo ""
  echo "Baseline created. No version bump needed for initial extraction."
  exit 0
fi

echo "Baseline dir:    $BASELINE_DIR"
echo ""

# Compare each current ABI against its baseline
for abi_file in "$ABI_DIR"/*.json; do
  [[ ! -f "$abi_file" ]] && continue
  contract_name="$(basename "$abi_file" .json)"
  baseline_file="$BASELINE_DIR/$contract_name.json"

  if [[ ! -f "$baseline_file" ]]; then
    # New contract — non-breaking addition
    ADDITIONS+=("$contract_name: new contract ABI added")
    HAS_CHANGES=true
    echo "  [ADD] $contract_name — new contract"
    continue
  fi

  # Compare WASM hashes first (quick check)
  CURR_HASH=$(grep '"wasm_hash"' "$abi_file" | head -1 | sed 's/.*"wasm_hash": *"//;s/".*//')
  BASE_HASH=$(grep '"wasm_hash"' "$baseline_file" | head -1 | sed 's/.*"wasm_hash": *"//;s/".*//')

  if [[ "$CURR_HASH" == "$BASE_HASH" ]]; then
    echo "  [OK]  $contract_name — unchanged"
    continue
  fi

  HAS_CHANGES=true

  # Deep comparison using jq if available, otherwise basic diff
  if command -v jq >/dev/null 2>&1; then
    # Extract function lists from spec
    CURR_FUNCTIONS=$(jq -r '[.spec[]? | select(.type == "function" or .function != null) | (.function.name // .name)] | sort | .[]' "$abi_file" 2>/dev/null || echo "")
    BASE_FUNCTIONS=$(jq -r '[.spec[]? | select(.type == "function" or .function != null) | (.function.name // .name)] | sort | .[]' "$baseline_file" 2>/dev/null || echo "")

    # Check for removed functions (breaking)
    while IFS= read -r fn_name; do
      [[ -z "$fn_name" ]] && continue
      if ! echo "$CURR_FUNCTIONS" | grep -qx "$fn_name"; then
        BREAKING_CHANGES+=("$contract_name: removed function '$fn_name'")
        echo "  [BREAK] $contract_name — removed function: $fn_name"
      fi
    done <<< "$BASE_FUNCTIONS"

    # Check for added functions (non-breaking)
    while IFS= read -r fn_name; do
      [[ -z "$fn_name" ]] && continue
      if ! echo "$BASE_FUNCTIONS" | grep -qx "$fn_name"; then
        ADDITIONS+=("$contract_name: added function '$fn_name'")
        echo "  [ADD]   $contract_name — added function: $fn_name"
      fi
    done <<< "$CURR_FUNCTIONS"

    # Check for removed structs / types (breaking)
    CURR_TYPES=$(jq -r '[.spec[]? | select(.type == "struct" or .struct != null) | (.struct.name // .name)] | sort | .[]' "$abi_file" 2>/dev/null || echo "")
    BASE_TYPES=$(jq -r '[.spec[]? | select(.type == "struct" or .struct != null) | (.struct.name // .name)] | sort | .[]' "$baseline_file" 2>/dev/null || echo "")

    while IFS= read -r type_name; do
      [[ -z "$type_name" ]] && continue
      if ! echo "$CURR_TYPES" | grep -qx "$type_name"; then
        BREAKING_CHANGES+=("$contract_name: removed type '$type_name'")
        echo "  [BREAK] $contract_name — removed type: $type_name"
      fi
    done <<< "$BASE_TYPES"

    while IFS= read -r type_name; do
      [[ -z "$type_name" ]] && continue
      if ! echo "$BASE_TYPES" | grep -qx "$type_name"; then
        ADDITIONS+=("$contract_name: added type '$type_name'")
        echo "  [ADD]   $contract_name — added type: $type_name"
      fi
    done <<< "$CURR_TYPES"

    # If hash changed but no structural diff detected, it is a patch-level change
    if ! printf '%s\n' "${BREAKING_CHANGES[@]}" "${ADDITIONS[@]}" | grep -q "$contract_name"; then
      PATCHES+=("$contract_name: WASM hash changed (implementation update)")
      echo "  [PATCH] $contract_name — implementation changed"
    fi

  else
    # No jq — fall back to file diff
    if ! diff -q "$baseline_file" "$abi_file" >/dev/null 2>&1; then
      PATCHES+=("$contract_name: ABI changed (install jq for detailed diff)")
      echo "  [PATCH] $contract_name — changed (install jq for details)"
    fi
  fi
done

# Check for removed contracts (breaking)
for baseline_file in "$BASELINE_DIR"/*.json; do
  [[ ! -f "$baseline_file" ]] && continue
  contract_name="$(basename "$baseline_file" .json)"
  if [[ ! -f "$ABI_DIR/$contract_name.json" ]]; then
    BREAKING_CHANGES+=("$contract_name: contract removed entirely")
    HAS_CHANGES=true
    echo "  [BREAK] $contract_name — contract removed"
  fi
done

# ---------------------------------------------------------------------------
# Determine version bump
# ---------------------------------------------------------------------------
echo ""

if [[ "$HAS_CHANGES" == "false" ]]; then
  echo "No changes detected. Version stays at $CURRENT_VERSION."
  exit 0
fi

BUMP_TYPE="patch"
if [[ ${#BREAKING_CHANGES[@]} -gt 0 ]]; then
  BUMP_TYPE="major"
elif [[ ${#ADDITIONS[@]} -gt 0 ]]; then
  BUMP_TYPE="minor"
fi

case "$BUMP_TYPE" in
  major) NEW_MAJOR=$((MAJOR + 1)); NEW_VERSION="$NEW_MAJOR.0.0" ;;
  minor) NEW_MINOR=$((MINOR + 1)); NEW_VERSION="$MAJOR.$NEW_MINOR.0" ;;
  patch) NEW_PATCH=$((PATCH + 1)); NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH" ;;
esac

echo "Change summary:"
echo "  Breaking changes: ${#BREAKING_CHANGES[@]}"
echo "  Additions:        ${#ADDITIONS[@]}"
echo "  Patches:          ${#PATCHES[@]}"
echo ""
echo "Version bump: $CURRENT_VERSION -> $NEW_VERSION ($BUMP_TYPE)"

# ---------------------------------------------------------------------------
# Generate changelog entry
# ---------------------------------------------------------------------------
TODAY=$(date +%Y-%m-%d)
CHANGELOG_ENTRY="## [$NEW_VERSION] - $TODAY

"

if [[ ${#BREAKING_CHANGES[@]} -gt 0 ]]; then
  CHANGELOG_ENTRY+="### Breaking Changes
"
  for change in "${BREAKING_CHANGES[@]}"; do
    CHANGELOG_ENTRY+="- $change
"
  done
  CHANGELOG_ENTRY+="
"
fi

if [[ ${#ADDITIONS[@]} -gt 0 ]]; then
  CHANGELOG_ENTRY+="### Added
"
  for change in "${ADDITIONS[@]}"; do
    CHANGELOG_ENTRY+="- $change
"
  done
  CHANGELOG_ENTRY+="
"
fi

if [[ ${#PATCHES[@]} -gt 0 ]]; then
  CHANGELOG_ENTRY+="### Changed
"
  for change in "${PATCHES[@]}"; do
    CHANGELOG_ENTRY+="- $change
"
  done
  CHANGELOG_ENTRY+="
"
fi

echo ""
echo "--- Changelog entry ---"
echo "$CHANGELOG_ENTRY"
echo "-----------------------"

# ---------------------------------------------------------------------------
# Apply changes (unless --dry-run)
# ---------------------------------------------------------------------------
if [[ "$DRY_RUN" == "true" ]]; then
  echo ""
  echo "[DRY RUN] No files modified."

  if [[ ${#BREAKING_CHANGES[@]} -gt 0 ]]; then
    exit 2
  fi
  exit 0
fi

# Update package.json version
sed -i.bak "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" "$PACKAGE_JSON"
rm -f "${PACKAGE_JSON}.bak"
echo ""
echo "Updated $PACKAGE_JSON -> $NEW_VERSION"

# Prepend changelog entry
if [[ -f "$CHANGELOG" ]]; then
  # Insert after the first "# Changelog" header line
  TEMP_CHANGELOG="$CHANGELOG.tmp"
  {
    head -2 "$CHANGELOG"
    echo ""
    echo "$CHANGELOG_ENTRY"
    tail -n +3 "$CHANGELOG"
  } > "$TEMP_CHANGELOG"
  mv "$TEMP_CHANGELOG" "$CHANGELOG"
else
  cat > "$CHANGELOG" <<EOF
# Changelog

$CHANGELOG_ENTRY
EOF
fi
echo "Updated $CHANGELOG"

# Update baseline
echo "Updating baseline..."
rm -rf "$BASELINE_DIR"
mkdir -p "$BASELINE_DIR"
cp "$ABI_DIR"/*.json "$BASELINE_DIR"/ 2>/dev/null || true
echo "Baseline updated."

echo ""
echo "======================================================================"
echo " Versioning complete: $CURRENT_VERSION -> $NEW_VERSION"
echo "======================================================================"

# Exit with code 2 if breaking changes were found (useful for CI)
if [[ ${#BREAKING_CHANGES[@]} -gt 0 ]]; then
  exit 2
fi

exit 0
